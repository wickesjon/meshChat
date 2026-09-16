//! Confirmed two-key pins and admitted, strict friend verification (MC-019).
//! No private material is retained. Native feature integration supplies a fresh
//! unlocked store/session per operation; network claims never mutate pins.
use crate::{
    LinkHandle, codec, framing,
    identity::{IdentityKeySession, PublicIdentity},
    ingress::{self, Ingress, Outcome, State, WorkPermit},
    links::UnconfirmedProposal,
    storage::{AcceptResult, EncryptedStore, HistoryItem, StorageError},
    text,
};
use ed25519_dalek::{Signature, VerifyingKey};
use sha2::{Digest, Sha256};

mod proof;
pub use proof::{Observation, Role};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid friend input")]
    Invalid,
    #[error("friend or identity state changed")]
    Stale,
    #[error("bounded friend state full")]
    Full,
    #[error("protected provider unavailable")]
    Provider,
    #[error("ingress refused work")]
    Ingress(#[from] ingress::Error),
    #[error("storage refused effect")]
    Storage(#[from] StorageError),
}
fn canonical_coordinate(raw: &[u8], ed: bool) -> bool {
    let Ok(mut value) = <[u8; 32]>::try_from(raw) else {
        return false;
    };
    if ed {
        value[31] &= 127;
    }
    let mut prime = [255; 32];
    prime[0] = 237;
    prime[31] = 127;
    value.iter().rev().cmp(prime.iter().rev()).is_lt()
}
pub(crate) fn public_key(raw: &[u8]) -> Result<VerifyingKey, Error> {
    if !canonical_coordinate(raw, true) {
        return Err(Error::Invalid);
    }
    let bytes = raw.try_into().map_err(|_| Error::Invalid)?;
    let key = VerifyingKey::from_bytes(&bytes).map_err(|_| Error::Invalid)?;
    if key.is_weak() {
        return Err(Error::Invalid);
    }
    Ok(key)
}
pub(crate) fn verify(key: &[u8; 32], transcript: &[u8], signature: &[u8]) -> bool {
    let Ok(public) = public_key(key) else {
        return false;
    };
    if signature.len() != 64 || !canonical_coordinate(&signature[..32], true) {
        return false;
    }
    Signature::from_slice(signature).is_ok_and(|s| public.verify_strict(transcript, &s).is_ok())
}
fn hint(key: &[u8]) -> [u8; 8] {
    Sha256::digest(key)[..8].try_into().unwrap()
}
/// Resolve one already bounded short-ID bucket by full key equality. This pure
/// helper grants no trust; actual packet verification still uses strict Ed25519.
pub fn unique_signing_candidate(candidates: impl Iterator<Item = [u8; 32]>) -> Option<[u8; 32]> {
    let mut found = None;
    for key in candidates {
        if found.is_some_and(|old| old != key) {
            return None;
        }
        found = Some(key);
    }
    found
}
fn transcript(raw: &[u8]) -> Vec<u8> {
    let body = &raw[26..raw.len() - 64];
    let mut out = b"meshfest/friend-sign/v1\0".to_vec();
    out.extend_from_slice(&raw[..3]);
    out.extend_from_slice(&raw[4..26]);
    out.extend_from_slice(&(body.len() as u16).to_be_bytes());
    out.extend_from_slice(body);
    out
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SendToken {
    generation: [u8; 16],
    tuple: [u8; 64],
    revision: u64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pin {
    token: SendToken,
    petname: String,
    replacing: bool,
}
impl Pin {
    pub fn tuple(&self) -> &[u8; 64] {
        &self.token.tuple
    }
    pub fn petname(&self) -> &str {
        &self.petname
    }
    pub fn replacing(&self) -> bool {
        self.replacing
    }
    pub fn token(&self) -> SendToken {
        self.token.clone()
    }
}
struct CachedKey {
    key: [u8; 32],
    used: u64,
}

/// Only begin/retry can construct a job from already admitted bytes. Completion
/// runs the real verifier internally; there is no caller-supplied trust result.
pub struct SignatureJob {
    permit: WorkPermit,
    link: LinkHandle,
    raw: Vec<u8>,
    key: [u8; 32],
    generation: [u8; 16],
}
pub struct SignedPacket {
    raw: Vec<u8>,
    link: LinkHandle,
    generation: [u8; 16],
    includes_key: bool,
}
impl SignedPacket {
    pub fn bytes(&self) -> &[u8] {
        &self.raw
    }
}
#[derive(Debug, PartialEq, Eq)]
pub struct VerifiedContent {
    pub signer: [u8; 32],
    pub friend: Option<Pin>,
    /// True only after a new CHAT ledger/history transaction commits. ANNOUNCE
    /// and authenticated replays never refresh history, pins or presence.
    pub new_history: bool,
    pub replay: bool,
    pub conflict: bool,
}

pub struct Friends {
    generation: [u8; 16],
    own_ed: [u8; 32],
    pins: Vec<Pin>,
    keys: Vec<CachedKey>,
    sessions: Vec<proof::Session>,
    now: u64,
    last_responses: Vec<(SendToken, u64)>,
}
impl Friends {
    pub fn open(store: &EncryptedStore, own: &PublicIdentity, now: u64) -> Result<Self, Error> {
        let generation = own
            .generation
            .as_slice()
            .try_into()
            .map_err(|_| Error::Invalid)?;
        let own_ed: [u8; 32] = own
            .signing_key
            .as_slice()
            .try_into()
            .map_err(|_| Error::Invalid)?;
        public_key(&own_ed)?;
        let mut result = Self {
            generation,
            own_ed,
            pins: Vec::with_capacity(128),
            keys: Vec::with_capacity(256),
            sessions: Vec::with_capacity(8),
            now,
            last_responses: Vec::with_capacity(128),
        };
        result.reload(store)?;
        Ok(result)
    }
    pub(crate) fn reload(&mut self, store: &EncryptedStore) -> Result<(), Error> {
        if store.identity_generation()? != self.generation {
            self.clear_session_trust();
            return Err(Error::Stale);
        }
        let mut pins = Vec::with_capacity(128);
        for (key, value) in store.friend_records()? {
            let tuple: [u8; 64] = key.try_into().map_err(|_| Error::Invalid)?;
            public_key(&tuple[..32])?;
            if !canonical_coordinate(&tuple[32..], false)
                || !(11..=30).contains(&value.len())
                || value[0] != 1
                || value[9] > 1
            {
                return Err(Error::Invalid);
            }
            let petname = std::str::from_utf8(&value[10..]).map_err(|_| Error::Invalid)?;
            text::validate(petname, text::Kind::Nickname).map_err(|_| Error::Invalid)?;
            let revision = u64::from_be_bytes(value[1..9].try_into().unwrap());
            if revision == 0 {
                return Err(Error::Invalid);
            }
            pins.push(Pin {
                token: SendToken {
                    generation: self.generation,
                    tuple,
                    revision,
                },
                petname: petname.into(),
                replacing: value[9] == 1,
            });
        }
        if self.pins != pins {
            self.clear_session_trust();
            self.keys.clear();
        }
        self.last_responses
            .retain(|(token, _)| pins.iter().any(|p| p.token == *token && !p.replacing));
        self.pins = pins;
        Ok(())
    }
    pub fn pins(&self) -> &[Pin] {
        &self.pins
    }
    /// Explicit user confirmation only. A URI parser proposal never calls this
    /// itself. The protected agreement validates the QR X key before any commit.
    pub fn confirm(
        &mut self,
        store: &EncryptedStore,
        provider: &IdentityKeySession,
        proposal: UnconfirmedProposal,
        petname: &str,
        previous: Option<&SendToken>,
    ) -> Result<SendToken, Error> {
        self.reload(store)?;
        let UnconfirmedProposal::Friend { bundle, .. } = proposal else {
            return Err(Error::Invalid);
        };
        text::validate(petname, text::Kind::Nickname).map_err(|_| Error::Invalid)?;
        if bundle[0] != 1 {
            return Err(Error::Invalid);
        }
        public_key(&bundle[1..33])?;
        if !canonical_coordinate(&bundle[33..], false) {
            return Err(Error::Invalid);
        }
        let own = provider.public_identity().map_err(|_| Error::Provider)?;
        if own.generation != self.generation || own.signing_key != self.own_ed {
            return Err(Error::Stale);
        }
        let mut shared = provider
            .agree(bundle[33..].to_vec())
            .map_err(|_| Error::Provider)?;
        use zeroize::Zeroize;
        shared.zeroize();
        let old = if let Some(token) = previous {
            self.current(token)?;
            Some((token.tuple.as_slice(), token.revision))
        } else {
            None
        };
        let tuple: [u8; 64] = bundle[1..].try_into().unwrap();
        let revision = store.change_friend(old, Some((&tuple, petname)))?;
        self.reload(store)?;
        Ok(SendToken {
            generation: self.generation,
            tuple,
            revision,
        })
    }
    fn current(&self, token: &SendToken) -> Result<&Pin, Error> {
        self.pins
            .iter()
            .find(|p| p.token == *token)
            .ok_or(Error::Stale)
    }
    pub fn begin_replacement(
        &mut self,
        store: &EncryptedStore,
        token: &SendToken,
    ) -> Result<(), Error> {
        self.reload(store)?;
        self.current(token)?;
        store.begin_friend_replacement(&token.tuple, token.revision)?;
        self.reload(store)
    }
    pub fn remove(&mut self, store: &EncryptedStore, token: &SendToken) -> Result<(), Error> {
        self.reload(store)?;
        self.current(token)?;
        store.change_friend(Some((&token.tuple, token.revision)), None)?;
        self.reload(store)
    }
    pub fn validate_send(
        &mut self,
        store: &EncryptedStore,
        token: &SendToken,
    ) -> Result<[u8; 64], Error> {
        self.reload(store)?;
        let pin = self.current(token)?;
        if pin.replacing {
            return Err(Error::Stale);
        }
        Ok(token.tuple)
    }
    fn advance(&mut self, now: u64) -> Result<(), Error> {
        if now < self.now || now > u64::MAX - 900_000 {
            self.clear_session_trust();
            self.last_responses.clear();
            return Err(Error::Stale);
        }
        self.now = now;
        self.keys.retain(|key| now - key.used < 900_000);
        Ok(())
    }
    pub fn clear_session_trust(&mut self) {
        self.sessions.clear();
    }
    fn pinned(&self, key: &[u8; 32]) -> Option<Pin> {
        let mut pins = self
            .pins
            .iter()
            .filter(|p| p.tuple()[..32] == *key && !p.replacing);
        let first = pins.next()?;
        if pins.next().is_some() {
            None
        } else {
            Some(first.clone())
        }
    }
    fn resolve_key(&self, sig: codec::FriendSignature<'_>, sender: [u8; 8]) -> Option<[u8; 32]> {
        if let Some(raw) = sig.public_key {
            let key: [u8; 32] = raw.try_into().ok()?;
            return (hint(&key) == sender && sig.key_id == sender && public_key(&key).is_ok())
                .then_some(key);
        }
        unique_signing_candidate(
            self.pins
                .iter()
                .map(|p| <[u8; 32]>::try_from(&p.tuple()[..32]).unwrap())
                .chain(self.keys.iter().map(|k| k.key))
                .filter(|key| hint(key) == sender && sig.key_id == sender),
        )
    }
    fn cache(&mut self, key: [u8; 32]) {
        if let Some(old) = self.keys.iter_mut().find(|k| k.key == key) {
            old.used = self.now;
            return;
        }
        if self.keys.len() == 256 {
            let i = self
                .keys
                .iter()
                .enumerate()
                .min_by_key(|(_, k)| k.used)
                .unwrap()
                .0;
            self.keys.swap_remove(i);
        }
        self.keys.push(CachedKey {
            key,
            used: self.now,
        });
    }
    pub fn cached_keys(&self) -> usize {
        self.keys.len()
    }
    pub fn reserved_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.pins.capacity() * std::mem::size_of::<Pin>()
            + self
                .pins
                .iter()
                .map(|p| p.petname.capacity())
                .sum::<usize>()
            + self.keys.capacity() * std::mem::size_of::<CachedKey>()
            + self.sessions.capacity() * std::mem::size_of::<proof::Session>()
            + self.last_responses.capacity() * std::mem::size_of::<(SendToken, u64)>()
            + 2 * (std::mem::size_of::<SignatureJob>() + 1024)
    }
    fn prepare(
        &mut self,
        ingress: &mut Ingress,
        link: &LinkHandle,
        raw: &[u8],
        now: u64,
    ) -> Result<Option<SignatureJob>, Error> {
        self.advance(now)?;
        let packet = codec::parse(
            raw,
            if raw[1] == 1 {
                codec::Context::StoredChat
            } else {
                codec::Context::Live
            },
        )
        .map_err(|_| Error::Invalid)?;
        let sig = match packet.payload() {
            codec::Payload::Chat {
                signature: codec::Signature::Friend(sig),
                ..
            } => sig,
            codec::Payload::Announce {
                signature: Some(sig),
                ..
            } => sig,
            _ => return Ok(None),
        };
        let Some(key) = self.resolve_key(sig, packet.header().sender_id) else {
            return Ok(None);
        };
        let Some(permit) = ingress.begin_signature(link, raw, now)? else {
            return Ok(None);
        };
        Ok(Some(SignatureJob {
            permit,
            link: link.clone(),
            raw: raw.to_vec(),
            key,
            generation: self.generation,
        }))
    }
    pub fn receive(
        &mut self,
        ingress: &mut Ingress,
        link: &LinkHandle,
        reported: u64,
        frame: &[u8],
        now: u64,
    ) -> Result<(Outcome, Option<SignatureJob>), Error> {
        self.advance(now)?;
        let mut output = [0; 1035];
        let cap = self.receive_capacity(link)?;
        let outcome = ingress.receive_friend(
            link,
            (reported, frame),
            now,
            &mut output,
            (!self.link_ready(link), cap),
        )?;
        if let Outcome::Complete {
            kind: framing::ObjectKind::Transport(kind @ (2 | 3)),
            len,
            ..
        } = outcome
        {
            self.control(ingress, link, kind, &output[..len], now)?;
        }
        let job = match outcome {
            Outcome::Complete {
                kind: framing::ObjectKind::Logical,
                len,
                state: State::Pending | State::PendingDuplicate,
            } if self.link_ready(link) => self.prepare(ingress, link, &output[..len], now)?,
            _ => None,
        };
        Ok((outcome, job))
    }
    /// Retry only an existing admitted record. SYNC callers must first use the
    /// MC-015 ordered receive/admit_stored_sync path; receive() alone never
    /// authorizes an unsolicited SYNC inner record.
    pub fn retry_pending(
        &mut self,
        ingress: &mut Ingress,
        link: &LinkHandle,
        position: usize,
        now: u64,
    ) -> Result<Option<SignatureJob>, Error> {
        if !self.link_ready(link) {
            return Err(Error::Stale);
        }
        let Some(raw) = ingress.pending_bytes(position).map(<[u8]>::to_vec) else {
            return Ok(None);
        };
        self.prepare(ingress, link, &raw, now)
    }
    pub fn complete(
        &mut self,
        ingress: &mut Ingress,
        store: &EncryptedStore,
        job: SignatureJob,
        now: u64,
        wall: Option<i64>,
    ) -> Result<Option<VerifiedContent>, Error> {
        ingress.advance(now)?;
        ingress.finish_work(job.permit)?;
        self.advance(now)?;
        self.reload(store)?;
        if job.generation != self.generation {
            return Err(Error::Stale);
        }
        let raw = &job.raw;
        if !verify(&job.key, &transcript(raw), &raw[raw.len() - 64..]) {
            ingress.resolve_signature(&job.link, raw, false)?;
            return Ok(None);
        }
        let packet = codec::parse(
            raw,
            if raw[1] == 1 {
                codec::Context::StoredChat
            } else {
                codec::Context::Live
            },
        )
        .map_err(|_| Error::Invalid)?;
        let result = if let codec::Payload::Chat { timestamp, .. } = packet.payload() {
            let mut immutable = raw.clone();
            immutable[3] = 0;
            let mut subject = vec![1];
            subject.extend_from_slice(&job.key);
            store.accept_authenticated(
                HistoryItem {
                    conversation: packet.header().channel_id.to_vec(),
                    direct: false,
                    direction: 0,
                    logical_type: 1,
                    message_id: packet.header().message_id.to_vec(),
                    timestamp: i64::from(timestamp),
                    body: raw.clone(),
                    provenance: subject.clone(),
                },
                subject,
                immutable,
                wall,
            )?
        } else {
            AcceptResult::Replay
        };
        ingress.resolve_signature(&job.link, raw, true)?;
        self.cache(job.key);
        Ok(Some(VerifiedContent {
            signer: job.key,
            friend: self.pinned(&job.key),
            new_history: result == AcceptResult::Accepted,
            replay: result == AcceptResult::Replay && packet.header().kind == 1,
            conflict: result == AcceptResult::Conflict,
        }))
    }
}
