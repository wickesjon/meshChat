//! Encrypted persistence policy. Native adapters own SQLCipher/key access; this
//! module owns schema, parameterized queries, transactions and bounded effects.
//! Authentication is performed by MC-019/020/021 before accept_authenticated;
//! storing provenance does not itself verify a signature or establish liveness.
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};
use zeroize::{Zeroize, Zeroizing};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum StorageError {
    #[error("invalid storage input")]
    InvalidInput,
    #[error("storage unavailable")]
    Unavailable,
    #[error("encrypted database operation failed")]
    Database,
    #[error("unsupported or mismatched schema")]
    Schema,
    #[error("storage capacity reached")]
    Capacity,
    #[error("local clock uncertain")]
    ClockUncertain,
}
impl Zeroize for HistoryItem {
    fn zeroize(&mut self) {
        self.body.zeroize();
        self.provenance.zeroize();
    }
}
#[derive(Clone, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum SqlValue {
    Integer { value: i64 },
    Bytes { value: Vec<u8> },
    Text { value: String },
}
#[derive(Clone, Debug, uniffi::Record)]
pub struct SqlRow {
    pub cells: Vec<SqlValue>,
}
/// Trusted native connection, scoped to one unlocked operation. Implementations
/// bind every value, bound reads, require SQLCipher, and never log SQL arguments.
#[uniffi::export(callback_interface)]
pub trait SqlDatabase: Send + Sync {
    fn execute(&self, sql: String, values: Vec<SqlValue>) -> Result<(), StorageError>;
    fn query(
        &self,
        sql: String,
        values: Vec<SqlValue>,
        limit: u32,
    ) -> Result<Vec<SqlRow>, StorageError>;
}
fn n(v: i64) -> SqlValue {
    SqlValue::Integer { value: v }
}
fn b(v: &[u8]) -> SqlValue {
    SqlValue::Bytes { value: v.to_vec() }
}
fn integer(v: &SqlValue) -> Result<i64, StorageError> {
    if let SqlValue::Integer { value } = v {
        Ok(*value)
    } else {
        Err(StorageError::Schema)
    }
}
fn bytes(v: &SqlValue) -> Result<Vec<u8>, StorageError> {
    if let SqlValue::Bytes { value } = v {
        Ok(value.clone())
    } else {
        Err(StorageError::Schema)
    }
}

#[derive(Clone, Debug, uniffi::Record)]
pub struct HistoryItem {
    /// 4-byte canonical channel ID or full pinned Ed25519/X25519 tuple (64 bytes).
    /// Rotating routing tags are deliberately not a persistence key.
    pub conversation: Vec<u8>,
    pub direct: bool,
    pub direction: u8,
    pub logical_type: u8,
    pub message_id: Vec<u8>,
    pub timestamp: i64,
    pub body: Vec<u8>,
    /// Bounded canonical verification provenance, interpreted by the owner.
    pub provenance: Vec<u8>,
}
#[derive(Clone, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum AcceptResult {
    Accepted,
    Replay,
    Conflict,
}
#[derive(Clone, Debug, uniffi::Enum)]
pub enum RecordKind {
    Setting,
    Subscription,
    Friend,
    EventRoot,
    StaffCredential,
}
impl RecordKind {
    fn number(&self) -> i64 {
        match self {
            Self::Setting => 0,
            Self::Subscription => 1,
            Self::Friend => 2,
            Self::EventRoot => 3,
            Self::StaffCredential => 4,
        }
    }
    fn cap(&self) -> i64 {
        match self {
            Self::Friend => 128,
            Self::Subscription => 32,
            Self::EventRoot => 64,
            Self::StaffCredential => 256,
            Self::Setting => 64,
        }
    }
    fn valid(&self, key: &[u8], value: &[u8]) -> bool {
        let size = match self {
            Self::Setting => !key.is_empty() && key.len() <= 64,
            Self::Subscription => key.len() == 4,
            Self::Friend => key.len() == 64,
            Self::EventRoot => key.len() == 32,
            Self::StaffCredential => key.len() == 64,
        };
        size && value.len() <= 2048
    }
}
type FriendRecord = (Vec<u8>, Vec<u8>);

#[derive(uniffi::Object)]
pub struct EncryptedStore {
    db: Box<dyn SqlDatabase>,
    lock: Mutex<()>,
}
impl EncryptedStore {
    pub(crate) fn identity_generation(&self) -> Result<Vec<u8>, StorageError> {
        let rows = self.rows("SELECT generation FROM identity", vec![], 2)?;
        if rows.len() != 1 {
            return Err(StorageError::Schema);
        }
        bytes(rows[0].cells.first().ok_or(StorageError::Schema)?)
    }

    pub(crate) fn friend_records(&self) -> Result<Vec<FriendRecord>, StorageError> {
        let _lock = self.lock.lock().map_err(|_| StorageError::Unavailable)?;
        let mut records = Vec::new();
        for offset in [0, 64] {
            for row in self.rows(
                "SELECT key,value FROM records WHERE kind=2 ORDER BY key LIMIT 64 OFFSET ?",
                vec![n(offset)],
                64,
            )? {
                if row.cells.len() != 2 {
                    return Err(StorageError::Schema);
                }
                records.push((bytes(&row.cells[0])?, bytes(&row.cells[1])?));
            }
        }
        Ok(records)
    }

    /// MC-019 confirmed pin mutation. The durable counter prevents remove/re-add
    /// or restart from reviving old send handles. Replacement is one transaction.
    pub(crate) fn change_friend(
        &self,
        previous: Option<(&[u8], u64)>,
        replacement: Option<(&[u8], &str)>,
    ) -> Result<u64, StorageError> {
        let _lock = self.lock.lock().map_err(|_| StorageError::Unavailable)?;
        self.tx(|| {
            if let Some((key, revision)) = previous {
                let old = self.rows("SELECT value FROM records WHERE kind=2 AND key=?", vec![b(key)], 1)?;
                let value = bytes(old.first().and_then(|r| r.cells.first()).ok_or(StorageError::Schema)?)?;
                if value.len() < 9 || value[0] != 1 || value[1..9] != revision.to_be_bytes() { return Err(StorageError::Schema); }
            }
            if let Some((key, _)) = replacement {
                if key.len() != 64 { return Err(StorageError::InvalidInput); }
                if previous.is_none_or(|(old, _)| old != key) && self.scalar("SELECT count(*) FROM records WHERE kind=2 AND key=?", vec![b(key)])? != 0 { return Err(StorageError::Schema); }
            }
            // Internal metadata kind 5 is outside the public RecordKind API.
            let counter_key = b"mc019.pin.counter";
            let rows = self.rows("SELECT value FROM records WHERE kind=5 AND key=?", vec![b(counter_key)], 1)?;
            let counter = match rows.first() {
                Some(row) => u64::from_be_bytes(bytes(row.cells.first().ok_or(StorageError::Schema)?)?.try_into().map_err(|_| StorageError::Schema)?),
                None => 0,
            }.checked_add(1).ok_or(StorageError::Capacity)?;
            self.exec("INSERT INTO records(kind,key,value) VALUES(5,?,?) ON CONFLICT(kind,key) DO UPDATE SET value=excluded.value", vec![b(counter_key),b(&counter.to_be_bytes())])?;
            if let Some((key, _)) = previous { self.exec("DELETE FROM records WHERE kind=2 AND key=?", vec![b(key)])?; }
            if let Some((key, petname)) = replacement {
                let mut value = vec![1]; value.extend_from_slice(&counter.to_be_bytes()); value.push(0); value.extend_from_slice(petname.as_bytes());
                self.exec("INSERT INTO records(kind,key,value) VALUES(2,?,?)", vec![b(key), b(&value)])?;
                if self.scalar("SELECT count(*) FROM records WHERE kind=2", vec![])? > 128 { return Err(StorageError::Capacity); }
            }
            Ok(counter)
        })
    }

    pub(crate) fn begin_friend_replacement(
        &self,
        key: &[u8],
        revision: u64,
    ) -> Result<(), StorageError> {
        let _lock = self.lock.lock().map_err(|_| StorageError::Unavailable)?;
        self.tx(|| {
            let rows = self.rows(
                "SELECT value FROM records WHERE kind=2 AND key=?",
                vec![b(key)],
                1,
            )?;
            let mut value = bytes(
                rows.first()
                    .and_then(|r| r.cells.first())
                    .ok_or(StorageError::Schema)?,
            )?;
            if value.len() < 10 || value[0] != 1 || value[1..9] != revision.to_be_bytes() {
                return Err(StorageError::Schema);
            }
            value[9] = 1;
            self.exec(
                "UPDATE records SET value=? WHERE kind=2 AND key=?",
                vec![b(&value), b(key)],
            )
        })
    }

    fn exec(&self, sql: &str, values: Vec<SqlValue>) -> Result<(), StorageError> {
        self.db.execute(sql.into(), values)
    }
    fn rows(
        &self,
        sql: &str,
        values: Vec<SqlValue>,
        limit: u32,
    ) -> Result<Vec<SqlRow>, StorageError> {
        let rows = self.db.query(sql.into(), values, limit)?;
        if rows.len() > limit as usize
            || rows.iter().any(|r| {
                r.cells.len() > 12
                    || r.cells.iter().any(|v| match v {
                        SqlValue::Bytes { value } => value.len() > 4096,
                        SqlValue::Text { value } => value.len() > 4096,
                        _ => false,
                    })
            })
        {
            return Err(StorageError::Database);
        }
        Ok(rows)
    }
    fn scalar(&self, sql: &str, values: Vec<SqlValue>) -> Result<i64, StorageError> {
        let rows = self.rows(sql, values, 1)?;
        integer(
            rows.first()
                .and_then(|r| r.cells.first())
                .ok_or(StorageError::Schema)?,
        )
    }
    fn tx<T>(&self, work: impl FnOnce() -> Result<T, StorageError>) -> Result<T, StorageError> {
        self.exec("BEGIN IMMEDIATE", vec![])?;
        match work() {
            Ok(result) => match self.exec("COMMIT", vec![]) {
                Ok(()) => Ok(result),
                Err(e) => {
                    let _ = self.exec("ROLLBACK", vec![]);
                    Err(e)
                }
            },
            Err(e) => {
                let _ = self.exec("ROLLBACK", vec![]);
                Err(e)
            }
        }
    }
    fn migrate(&self, generation: &[u8], create: bool) -> Result<(), StorageError> {
        self.tx(||{
        let version=self.scalar("PRAGMA user_version",vec![])?;
        if !(0..=2).contains(&version) || (version==0&&!create) || (version!=0&&create){return Err(StorageError::Schema)}
        if version==0 {
            self.exec("CREATE TABLE identity(generation BLOB NOT NULL CHECK(length(generation)=16))",vec![])?;
            self.exec("INSERT INTO identity VALUES (?)",vec![b(generation)])?;
            self.exec("CREATE TABLE records(kind INTEGER NOT NULL, key BLOB NOT NULL, value BLOB NOT NULL CHECK(length(value)<=2048), PRIMARY KEY(kind,key)) WITHOUT ROWID",vec![])?;
            self.exec("CREATE TABLE history(id INTEGER PRIMARY KEY, conversation BLOB NOT NULL, direct INTEGER NOT NULL, direction INTEGER NOT NULL, logical_type INTEGER NOT NULL, message_id BLOB NOT NULL CHECK(length(message_id)=8), timestamp INTEGER NOT NULL, body BLOB NOT NULL CHECK(length(body)<=1024), provenance BLOB NOT NULL CHECK(length(provenance)<=512))",vec![])?;
            self.exec("CREATE INDEX history_conversation ON history(direct,conversation,timestamp,id)",vec![])?;
            self.exec("PRAGMA user_version=1",vec![])?;
        }
        let stored=self.rows("SELECT generation FROM identity",vec![],2)?;
        if stored.len()!=1 || bytes(stored[0].cells.first().ok_or(StorageError::Schema)?)?!=generation{return Err(StorageError::Schema)}
        if version<2 {
            self.exec("CREATE TABLE ledger(subject BLOB NOT NULL, direction INTEGER NOT NULL, logical_type INTEGER NOT NULL, message_id BLOB NOT NULL, digest BLOB NOT NULL, timestamp INTEGER NOT NULL, conflict INTEGER NOT NULL DEFAULT 0, PRIMARY KEY(subject,direction,logical_type,message_id)) WITHOUT ROWID",vec![])?;
            self.exec("CREATE INDEX ledger_expiry ON ledger(timestamp)",vec![])?;
            self.exec("CREATE TABLE clock(high_water INTEGER NOT NULL, uncertain INTEGER NOT NULL)",vec![])?;
            self.exec("INSERT INTO clock VALUES(0,0)",vec![])?;
            self.exec("PRAGMA user_version=2",vec![])?;
        }
        Ok(())
    })
    }
    /// Persist uncertainty even when the caller's authenticated effect is refused.
    fn clock(&self, now: Option<i64>) -> Result<i64, StorageError> {
        let accepted = self.tx(|| {
            let rows = self.rows("SELECT high_water,uncertain FROM clock", vec![], 2)?;
            if rows.len() != 1 || rows[0].cells.len() != 2 {
                return Err(StorageError::Schema);
            }
            let high = integer(&rows[0].cells[0])?;
            let uncertain = integer(&rows[0].cells[1])? != 0;
            let valid = now.filter(|v| *v >= 0 && *v <= i64::MAX - 172800);
            let good = valid.is_some_and(|v| {
                if uncertain {
                    v >= high
                } else {
                    v >= high.saturating_sub(300)
                }
            });
            if good {
                let next = high.max(valid.unwrap());
                self.exec("UPDATE clock SET high_water=?,uncertain=0", vec![n(next)])?;
                Ok(Some(next))
            } else {
                self.exec("UPDATE clock SET uncertain=1", vec![])?;
                Ok(None)
            }
        })?;
        accepted.ok_or(StorageError::ClockUncertain)
    }
    fn prune_at(&self, now: i64) -> Result<(), StorageError> {
        self.exec(
            "DELETE FROM history WHERE timestamp < ?",
            vec![n(now.saturating_sub(172800))],
        )?;
        self.exec(
            "DELETE FROM ledger WHERE timestamp < ?",
            vec![n(now.saturating_sub(172800))],
        )?;
        self.prune_dm_reactions()
    }
    fn validate_item(item: &HistoryItem) -> Result<(), StorageError> {
        if item.conversation.len() != if item.direct { 64 } else { 4 }
            || item.direction > 1
            || ![1, 6].contains(&item.logical_type)
            || item.message_id.len() != 8
            || item.timestamp < 0
            || item.timestamp > i64::MAX - 172800
            || item.body.len() > 1024
            || item.provenance.len() > 512
        {
            return Err(StorageError::InvalidInput);
        }
        Ok(())
    }
    fn insert(&self, item: &HistoryItem) -> Result<(), StorageError> {
        self.exec("INSERT INTO history(conversation,direct,direction,logical_type,message_id,timestamp,body,provenance) VALUES(?,?,?,?,?,?,?,?)",vec![b(&item.conversation),n(item.direct as i64),n(item.direction.into()),n(item.logical_type.into()),b(&item.message_id),n(item.timestamp),b(&item.body),b(&item.provenance)])?;
        self.exec("DELETE FROM history WHERE direct=? AND conversation=? AND id NOT IN (SELECT id FROM history WHERE direct=? AND conversation=? ORDER BY timestamp DESC,id DESC LIMIT ?)",vec![n(item.direct as i64),b(&item.conversation),n(item.direct as i64),b(&item.conversation),n(if item.direct{1000}else{5000})])?;
        // Never prune live replay tombstones to make room. History may refuse a
        // new transaction at global caps; explicit deletion/age pruning recovers.
        if self.scalar("SELECT count(*) FROM history",vec![])?>20000 || self.scalar("SELECT coalesce(sum(length(body)+length(provenance)),0) FROM history",vec![])?>32*1024*1024 || self.scalar("SELECT count(*) FROM history WHERE direct=1",vec![])?>5000 || self.scalar("SELECT coalesce(sum(length(body)+length(provenance)),0) FROM history WHERE direct=1",vec![])?>8*1024*1024 {return Err(StorageError::Capacity)}
        Ok(())
    }
}
#[uniffi::export]
impl EncryptedStore {
    /// Native-only session construction after SQLCipher opens under the matching
    /// unlocked identity. Existing empty/wrong-key databases never become fresh.
    #[uniffi::constructor]
    pub fn open(
        db: Box<dyn SqlDatabase>,
        generation: Vec<u8>,
        create: bool,
        now: Option<i64>,
    ) -> Result<Arc<Self>, StorageError> {
        if generation.len() != 16 || generation.iter().all(|b| *b == 0) {
            return Err(StorageError::InvalidInput);
        }
        let store = Arc::new(Self {
            db,
            lock: Mutex::new(()),
        });
        let cipher = store.rows("PRAGMA cipher_version", vec![], 1)?;
        if !matches!(cipher.first().and_then(|r|r.cells.first()),Some(SqlValue::Text{value}) if !value.is_empty())
        {
            return Err(StorageError::Database);
        }
        store.exec("PRAGMA foreign_keys=ON", vec![])?;
        if store.scalar("PRAGMA secure_delete=ON", vec![])? != 1 {
            return Err(StorageError::Database);
        }
        store.exec("PRAGMA temp_store=MEMORY", vec![])?;
        store.rows("PRAGMA max_page_count=16384", vec![], 1)?;
        store.migrate(&generation, create)?;
        match store.clock(now) {
            Ok(time) => store.tx(|| store.prune_at(time))?,
            Err(StorageError::ClockUncertain) => {}
            Err(e) => return Err(e),
        }
        Ok(store)
    }
    pub fn put_record(
        &self,
        kind: RecordKind,
        key: Vec<u8>,
        value: Vec<u8>,
    ) -> Result<(), StorageError> {
        if !kind.valid(&key, &value) {
            return Err(StorageError::InvalidInput);
        }
        let _lock = self.lock.lock().map_err(|_| StorageError::Unavailable)?;
        self.tx(||{
            self.exec("INSERT INTO records(kind,key,value) VALUES(?,?,?) ON CONFLICT(kind,key) DO UPDATE SET value=excluded.value",vec![n(kind.number()),b(&key),b(&value)])?;
            if self.scalar("SELECT count(*) FROM records WHERE kind=?",vec![n(kind.number())])?>kind.cap(){return Err(StorageError::Capacity)}
            Ok(())
        })
    }
    pub fn get_record(
        &self,
        kind: RecordKind,
        key: Vec<u8>,
    ) -> Result<Option<Vec<u8>>, StorageError> {
        if !kind.valid(&key, &[]) {
            return Err(StorageError::InvalidInput);
        }
        let _lock = self.lock.lock().map_err(|_| StorageError::Unavailable)?;
        let rows = self.rows(
            "SELECT value FROM records WHERE kind=? AND key=?",
            vec![n(kind.number()), b(&key)],
            1,
        )?;
        rows.first()
            .map(|r| bytes(r.cells.first().ok_or(StorageError::Schema)?))
            .transpose()
    }
    pub fn delete_record(&self, kind: RecordKind, key: Vec<u8>) -> Result<(), StorageError> {
        if !kind.valid(&key, &[]) {
            return Err(StorageError::InvalidInput);
        }
        let _lock = self.lock.lock().map_err(|_| StorageError::Unavailable)?;
        self.exec(
            "DELETE FROM records WHERE kind=? AND key=?",
            vec![n(kind.number()), b(&key)],
        )
    }
    pub fn append_unverified(
        &self,
        item: HistoryItem,
        now: Option<i64>,
    ) -> Result<(), StorageError> {
        Self::validate_item(&item)?;
        if item.direct || !item.provenance.is_empty() {
            return Err(StorageError::InvalidInput);
        }
        let _lock = self.lock.lock().map_err(|_| StorageError::Unavailable)?;
        let time = self.clock(now)?;
        if item.timestamp < time.saturating_sub(172800) || item.timestamp > time + 300 {
            return Err(StorageError::InvalidInput);
        }
        self.tx(|| {
            self.prune_at(time)?;
            self.insert(&item)
        })
    }
    /// Only protocol owners invoke after successful authentication. Subject is
    /// domain byte + full signer(32), friend tuple(64), or root/staff pair(64).
    /// immutable_bytes must be the authenticated TTL-normalized representation.
    pub fn accept_authenticated(
        &self,
        item: HistoryItem,
        subject: Vec<u8>,
        immutable_bytes: Vec<u8>,
        now: Option<i64>,
    ) -> Result<AcceptResult, StorageError> {
        self.accept_record(item, subject, immutable_bytes, now, false)
            .map(|(result, _)| result)
    }
    /// A target with multiple authenticated meanings or a recorded conflict is
    /// ambiguous even across logical types/directions; never select one by ID.
    pub fn ambiguous_target(
        &self,
        subject: Vec<u8>,
        message_id: Vec<u8>,
    ) -> Result<bool, StorageError> {
        if subject.len() > 65 || message_id.len() != 8 {
            return Err(StorageError::InvalidInput);
        }
        let _lock = self.lock.lock().map_err(|_| StorageError::Unavailable)?;
        Ok(self.scalar("SELECT count(*)+coalesce(sum(conflict),0) FROM ledger WHERE subject=? AND message_id=?",vec![b(&subject),b(&message_id)])?>1)
    }
    pub fn history(
        &self,
        conversation: Vec<u8>,
        direct: bool,
        limit: u32,
    ) -> Result<Vec<HistoryItem>, StorageError> {
        if conversation.len() != if direct { 64 } else { 4 } || limit == 0 || limit > 100 {
            return Err(StorageError::InvalidInput);
        }
        let _lock = self.lock.lock().map_err(|_| StorageError::Unavailable)?;
        self.rows("SELECT direction,logical_type,message_id,timestamp,body,provenance FROM history WHERE direct=? AND conversation=? ORDER BY timestamp DESC,id DESC LIMIT ?",vec![n(direct as i64),b(&conversation),n(limit.into())],limit)?.into_iter().map(|row|{
            if row.cells.len()!=6{return Err(StorageError::Schema)}let c=row.cells;
            Ok(HistoryItem{conversation:conversation.clone(),direct,direction:integer(&c[0])?.try_into().map_err(|_|StorageError::Schema)?,logical_type:integer(&c[1])?.try_into().map_err(|_|StorageError::Schema)?,message_id:bytes(&c[2])?,timestamp:integer(&c[3])?,body:bytes(&c[4])?,provenance:bytes(&c[5])?})
        }).collect()
    }
    /// Visible history deletion does not delete live replay records.
    pub fn delete_history(&self, conversation: Vec<u8>, direct: bool) -> Result<(), StorageError> {
        if conversation.len() != if direct { 64 } else { 4 } {
            return Err(StorageError::InvalidInput);
        }
        let _lock = self.lock.lock().map_err(|_| StorageError::Unavailable)?;
        self.tx(|| {
            self.exec(
                "DELETE FROM history WHERE direct=? AND conversation=?",
                vec![n(direct as i64), b(&conversation)],
            )?;
            self.prune_dm_reactions()
        })
    }
    pub fn prune(&self, now: Option<i64>) -> Result<(), StorageError> {
        let _lock = self.lock.lock().map_err(|_| StorageError::Unavailable)?;
        let time = self.clock(now)?;
        self.tx(|| self.prune_at(time))
    }
}

impl EncryptedStore {
    fn accept_record(
        &self,
        item: HistoryItem,
        subject: Vec<u8>,
        immutable_bytes: Vec<u8>,
        now: Option<i64>,
        dm: bool,
    ) -> Result<(AcceptResult, bool), StorageError> {
        let mut item = Zeroizing::new(item);
        Self::validate_item(&item)?;
        let valid = match subject.first() {
            Some(1) => subject.len() == 33,
            Some(2) | Some(3) => subject.len() == 65,
            _ => false,
        };
        if !valid
            || immutable_bytes.is_empty()
            || immutable_bytes.len() > 2048
            || item.provenance.is_empty()
            || (item.direct && (subject.first() != Some(&2) || subject[1..] != item.conversation))
        {
            return Err(StorageError::InvalidInput);
        }
        let _lock = self.lock.lock().map_err(|_| StorageError::Unavailable)?;
        let time = self.clock(now)?;
        if item.timestamp < time.saturating_sub(172800) || item.timestamp > time + 300 {
            return Err(StorageError::InvalidInput);
        }
        let digest = Sha256::digest(&immutable_bytes);
        self.tx(||{
            self.prune_at(time)?;
            // Public signatures identify the same broadcast in either local
            // direction. Preserve DM direction separation and history metadata.
            // Read BOTH legacy public directions; no migration deletes evidence.
            let public=matches!(subject.first(),Some(1|3));
            let direction=if public {0} else {item.direction};
            let key=vec![b(&subject),n(direction.into()),n(item.logical_type.into()),b(&item.message_id)];
            let (query,lookup,limit)=if public {
                ("SELECT digest FROM ledger WHERE subject=? AND logical_type=? AND message_id=? AND direction IN (0,1)",
                 vec![b(&subject),n(item.logical_type.into()),b(&item.message_id)],2)
            } else {
                ("SELECT digest FROM ledger WHERE subject=? AND direction=? AND logical_type=? AND message_id=?",key.clone(),1)
            };
            let rows=self.rows(query,lookup.clone(),limit)?;
            if !rows.is_empty(){
                let mut same=true;
                for row in &rows { same &= bytes(row.cells.first().ok_or(StorageError::Schema)?)?==digest.as_slice(); }
                if same{return Ok((AcceptResult::Replay, false))}
                // Legacy rows with different digests are already ambiguous;
                // never select a preferred direction and issue another effect.
                if public {
                    self.exec("UPDATE ledger SET conflict=1 WHERE subject=? AND logical_type=? AND message_id=? AND direction IN (0,1)",lookup)?;
                } else {
                    self.exec("UPDATE ledger SET conflict=1 WHERE subject=? AND direction=? AND logical_type=? AND message_id=?",key)?;
                }
                return Ok((AcceptResult::Conflict, false))
            }
            if self.scalar("SELECT count(*) FROM ledger",vec![])?>=100000{return Err(StorageError::Capacity)}
            let mut values=key;values.push(b(&digest));values.push(n(item.timestamp));
            self.exec("INSERT INTO ledger(subject,direction,logical_type,message_id,digest,timestamp) VALUES(?,?,?,?,?,?)",values)?;
            if dm {
                let name=b"mc020.sequence";
                let rows=self.rows("SELECT value FROM records WHERE kind=5 AND key=?",vec![b(name)],1)?;
                let sequence=match rows.first(){Some(r)=>u64::from_be_bytes(bytes(r.cells.first().ok_or(StorageError::Schema)?)?.try_into().map_err(|_|StorageError::Schema)?),None=>0}.checked_add(1).ok_or(StorageError::Capacity)?;
                self.exec("INSERT INTO records(kind,key,value) VALUES(5,?,?) ON CONFLICT(kind,key) DO UPDATE SET value=excluded.value",vec![b(name),b(&sequence.to_be_bytes())])?;
                item.provenance=subject.clone();item.provenance.extend_from_slice(&sequence.to_be_bytes());
            }
            self.insert(&item)?;
            let applied = if dm && item.logical_type==6 { self.apply_dm_reaction(&item.conversation,&item.message_id,item.direction)? } else { true };
            self.prune_dm_reactions()?;
            Ok((AcceptResult::Accepted, applied))
        })
    }
    pub(crate) fn accept_dm(
        &self,
        item: HistoryItem,
        subject: Vec<u8>,
        immutable: Vec<u8>,
        now: Option<i64>,
    ) -> Result<(AcceptResult, bool), StorageError> {
        self.accept_record(item, subject, immutable, now, true)
    }
    fn prune_dm_reactions(&self) -> Result<(), StorageError> {
        self.exec("DELETE FROM records WHERE kind=6 AND NOT EXISTS (SELECT 1 FROM history WHERE direct=1 AND logical_type=1 AND conversation=substr(records.key,1,64) AND message_id=substr(records.key,65,8))",vec![])
    }
    // Returns false only when no unique authenticated visible target exists.
    // Counter comparison makes recovery idempotent and prevents an older orphan
    // from overwriting a more recently accepted reaction after a storage retry.
    fn apply_dm_reaction(
        &self,
        peer: &[u8],
        reaction: &[u8],
        direction: u8,
    ) -> Result<bool, StorageError> {
        let rows=self.rows("SELECT body,provenance FROM history WHERE direct=1 AND logical_type=6 AND conversation=? AND message_id=? AND direction=?",vec![b(peer),b(reaction),n(direction.into())],2)?;
        if rows.len() != 1 {
            return Ok(false);
        }
        let body = bytes(&rows[0].cells[0])?;
        let provenance = bytes(&rows[0].cells[1])?;
        if body.len() != 64
            || provenance.len() != 73
            || provenance[0] != 2
            || provenance[1..65] != *peer
        {
            return Err(StorageError::Schema);
        }
        let target = &body[4..12];
        let mut subject = vec![2];
        subject.extend_from_slice(peer);
        if self.scalar("SELECT count(*)+coalesce(sum(conflict),0) FROM ledger WHERE subject=? AND message_id=?",vec![b(&subject),b(target)])?!=1{return Ok(false)}
        if self.scalar("SELECT count(*) FROM history WHERE direct=1 AND logical_type=1 AND conversation=? AND message_id=?",vec![b(peer),b(target)])?!=1{return Ok(false)}
        let mut key = peer.to_vec();
        key.extend_from_slice(target);
        key.push(direction);
        let old = self.rows(
            "SELECT value FROM records WHERE kind=6 AND key=?",
            vec![b(&key)],
            1,
        )?;
        if let Some(r) = old.first() {
            let value = bytes(&r.cells[0])?;
            if value.len() != 10 {
                return Err(StorageError::Schema);
            }
            if value[..8] >= provenance[65..] {
                return Ok(true);
            }
        }
        let mut value = provenance[65..].to_vec();
        value.push(body[12] & 1);
        value.push(body[13]);
        self.exec("INSERT INTO records(kind,key,value) VALUES(6,?,?) ON CONFLICT(kind,key) DO UPDATE SET value=excluded.value",vec![b(&key),b(&value)])?;
        Ok(true)
    }
    pub(crate) fn recover_dm_reaction(
        &self,
        peer: &[u8],
        id: &[u8],
        direction: u8,
        now: Option<i64>,
    ) -> Result<bool, StorageError> {
        let _lock = self.lock.lock().map_err(|_| StorageError::Unavailable)?;
        let time = self.clock(now)?;
        self.tx(|| {
            self.prune_at(time)?;
            self.apply_dm_reaction(peer, id, direction)
        })
    }
    pub(crate) fn dm_reactions(
        &self,
        peer: &[u8],
        target: &[u8],
        now: Option<i64>,
    ) -> Result<Vec<(u8, u8)>, StorageError> {
        let _lock = self.lock.lock().map_err(|_| StorageError::Unavailable)?;
        let time = self.clock(now)?;
        self.tx(||{
            self.prune_at(time)?;
            let mut subject=vec![2];subject.extend_from_slice(peer);
            if self.scalar("SELECT count(*)+coalesce(sum(conflict),0) FROM ledger WHERE subject=? AND message_id=?",vec![b(&subject),b(target)])?!=1{return Ok(vec![])}
            let mut prefix=peer.to_vec();prefix.extend_from_slice(target);
            let rows=self.rows("SELECT key,value FROM records WHERE kind=6 AND substr(key,1,72)=? ORDER BY key",vec![b(&prefix)],2)?;
            let mut out=Vec::new();
            for row in rows{let key=bytes(&row.cells[0])?;let value=bytes(&row.cells[1])?;if key.len()!=73||value.len()!=10{return Err(StorageError::Schema)}
            if value[8]==0{out.push((key[72],value[9]));}}
            Ok(out)
        })
    }
}

impl EncryptedStore {
    pub(crate) fn authority_time(&self, wall: Option<i64>) -> Result<i64, StorageError> {
        let _lock = self.lock.lock().map_err(|_| StorageError::Unavailable)?;
        self.clock(wall)
    }
    pub(crate) fn event_roots(&self) -> Result<Vec<FriendRecord>, StorageError> {
        let _lock = self.lock.lock().map_err(|_| StorageError::Unavailable)?;
        self.rows(
            "SELECT key,value FROM records WHERE kind=3 ORDER BY key",
            vec![],
            64,
        )?
        .into_iter()
        .map(|r| {
            if r.cells.len() != 2 {
                return Err(StorageError::Schema);
            }
            Ok((bytes(&r.cells[0])?, bytes(&r.cells[1])?))
        })
        .collect()
    }
    /// Confirmed adoption/removal only. A durable revision prevents old authority
    /// tokens and in-flight jobs surviving a remove/re-adopt or expiry renewal.
    pub(crate) fn change_event_root(
        &self,
        key: &[u8; 32],
        bundle_name: Option<&[u8]>,
        wall: Option<i64>,
    ) -> Result<u64, StorageError> {
        let _lock = self.lock.lock().map_err(|_| StorageError::Unavailable)?;
        let time = self.clock(wall)?;
        self.tx(|| {
            for row in self.rows("SELECT key,value FROM records WHERE kind=3",vec![],64)? {
                if row.cells.len()!=2 { return Err(StorageError::Schema); }
                let value=bytes(&row.cells[1])?;
                if value.len()<110 || value[0]!=1 { return Err(StorageError::Schema); }
                let expiry=u32::from_be_bytes(value[42..46].try_into().map_err(|_|StorageError::Schema)?);
                if i64::from(expiry)<time {
                    self.exec("DELETE FROM records WHERE kind=3 AND key=?",vec![row.cells[0].clone()])?;
                }
            }
            let name=b"mc021.root.counter";
            let rows=self.rows("SELECT value FROM records WHERE kind=5 AND key=?",vec![b(name)],1)?;
            let revision=match rows.first(){Some(r)=>u64::from_be_bytes(bytes(r.cells.first().ok_or(StorageError::Schema)?)?.try_into().map_err(|_|StorageError::Schema)?),None=>0}.checked_add(1).ok_or(StorageError::Capacity)?;
            self.exec("INSERT INTO records(kind,key,value) VALUES(5,?,?) ON CONFLICT(kind,key) DO UPDATE SET value=excluded.value",vec![b(name),b(&revision.to_be_bytes())])?;
            self.exec("DELETE FROM records WHERE kind=3 AND key=?",vec![b(key)])?;
            if let Some(bundle_name)=bundle_name {
                if bundle_name.len()<101 || bundle_name.len()>165 || bundle_name[0]!=1 || bundle_name[1..33]!=*key {return Err(StorageError::InvalidInput);}
                let mut value=vec![1];value.extend_from_slice(&revision.to_be_bytes());value.extend_from_slice(bundle_name);
                self.exec("INSERT INTO records(kind,key,value) VALUES(3,?,?)",vec![b(key),b(&value)])?;
                if self.scalar("SELECT count(*) FROM records WHERE kind=3",vec![])?>16 {return Err(StorageError::Capacity);}
            }
            Ok(revision)
        })
    }
}
