//! Encrypted persistence policy. Native adapters own SQLCipher/key access; this
//! module owns schema, parameterized queries, transactions and bounded effects.
//! Authentication is performed by MC-019/020/021 before accept_authenticated;
//! storing provenance does not itself verify a signature or establish liveness.
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};

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
#[derive(uniffi::Object)]
pub struct EncryptedStore {
    db: Box<dyn SqlDatabase>,
    lock: Mutex<()>,
}
impl EncryptedStore {
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
        )
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
            let key=vec![b(&subject),n(item.direction.into()),n(item.logical_type.into()),b(&item.message_id)];
            let rows=self.rows("SELECT digest FROM ledger WHERE subject=? AND direction=? AND logical_type=? AND message_id=?",key.clone(),1)?;
            if let Some(row)=rows.first(){
                if bytes(row.cells.first().ok_or(StorageError::Schema)?)?==digest.as_slice(){return Ok(AcceptResult::Replay)}
                self.exec("UPDATE ledger SET conflict=1 WHERE subject=? AND direction=? AND logical_type=? AND message_id=?",key)?;
                return Ok(AcceptResult::Conflict)
            }
            if self.scalar("SELECT count(*) FROM ledger",vec![])?>=100000{return Err(StorageError::Capacity)}
            let mut values=key;values.push(b(&digest));values.push(n(item.timestamp));
            self.exec("INSERT INTO ledger(subject,direction,logical_type,message_id,digest,timestamp) VALUES(?,?,?,?,?,?)",values)?;
            self.insert(&item)?;Ok(AcceptResult::Accepted)
        })
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
        self.exec(
            "DELETE FROM history WHERE direct=? AND conversation=?",
            vec![n(direct as i64), b(&conversation)],
        )
    }
    pub fn prune(&self, now: Option<i64>) -> Result<(), StorageError> {
        let _lock = self.lock.lock().map_err(|_| StorageError::Unavailable)?;
        let time = self.clock(now)?;
        self.tx(|| self.prune_at(time))
    }
}
