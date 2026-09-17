//! Confirmed organizer roots, bounded credential recovery and authenticated posts.
//! Native owners supply protected stores/key sessions and recheck authority at
//! display/egress. Structural relay allowance never grants an organizer badge.
use crate::{
    LinkHandle, codec, framing,
    friends::{self, Friends},
    identity::PublicIdentity,
    ingress::{Ingress, Outcome, State, WorkPermit},
    links::{StaffProposal, UnconfirmedProposal},
    storage::{AcceptResult, EncryptedStore, HistoryItem, StorageError},
    text,
};
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};
use std::sync::Mutex;
use zeroize::Zeroize;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid organizer input")]
    Invalid,
    #[error("organizer authority is unavailable or changed")]
    Authority,
    #[error("organizer state capacity exceeded")]
    Capacity,
    #[error("protected staff session unavailable")]
    Provider,
    #[error("intake refused")]
    Ingress(#[from] crate::ingress::Error),
    #[error("persistence refused")]
    Storage(#[from] StorageError),
}
fn hint(key: &[u8]) -> [u8; 8] {
    Sha256::digest(key)[..8].try_into().unwrap()
}
fn domain(name: &str) -> Vec<u8> {
    format!("meshfest/{name}/v1\0").into_bytes()
}
fn credential_transcript(raw: &[u8]) -> Vec<u8> {
    let body = &raw[..raw.len() - 64];
    let mut out = domain("credential");
    out.extend_from_slice(&(body.len() as u16).to_be_bytes());
    out.extend_from_slice(body);
    out
}
fn transcript(raw: &[u8]) -> Vec<u8> {
    let body = &raw[26..raw.len() - 64];
    let mut out = domain("staff-sign");
    out.extend_from_slice(&raw[..3]);
    out.extend_from_slice(&raw[4..26]);
    out.extend_from_slice(&(body.len() as u16).to_be_bytes());
    out.extend_from_slice(body);
    out
}
#[derive(Clone)]
struct Root {
    key: [u8; 32],
    revision: u64,
    bundle: [u8; 101],
    expiry: u32,
}
#[derive(Clone)]
pub struct Authority {
    root: [u8; 32],
    revision: u64,
    staff: [u8; 32],
    credential: Vec<u8>,
    generation: [u8; 16],
    pin_expiry: Option<u32>,
}
impl Authority {
    pub fn root(&self) -> &[u8; 32] {
        &self.root
    }
    pub fn staff(&self) -> &[u8; 32] {
        &self.staff
    }
    pub fn label(&self) -> &str {
        codec::credential(&self.credential).unwrap().label
    }
}
pub struct Verified {
    pub result: AcceptResult,
    pub authority: Authority,
}
pub struct Job {
    permit: WorkPermit,
    link: LinkHandle,
    raw: Vec<u8>,
    root: Root,
    credential: Vec<u8>,
    validated: bool,
}
struct Cached {
    raw: Vec<u8>,
    validated: Option<([u8; 32], u64)>,
    used: u64,
}
struct Recovery {
    pair: [u8; 16],
    link: LinkHandle,
    expires: u64,
    attempts: u8,
    next: u64,
}
/// Direct request intent; the caller schedules its encoded bytes through the
/// normal outbound owner. A failed send still consumes this bounded attempt.
pub struct Request {
    pub link: LinkHandle,
    pub pair: [u8; 16],
}
pub struct Intake {
    pub outcome: Outcome,
    pub job: Option<Job>,
    pub offer_response: Option<Vec<u8>>,
}
/// Short-lived native-provider session. No private export or automatic persistence.
/// A native adapter must wrap imported material before releasing its QR buffers,
/// and invalidate this session on lock/background/forget; feature wiring is later.
pub struct StaffSession {
    key: Mutex<Option<(SigningKey, bool)>>,
    authority: Authority,
}
impl StaffSession {
    pub fn invalidate(&self) {
        if let Ok(mut key) = self.key.lock() {
            *key = None;
        }
    }
    pub fn credential(&self) -> &[u8] {
        &self.authority.credential
    }
    pub fn authority(&self) -> &Authority {
        &self.authority
    }
}
/// Public-only schedule retained by the native feature across short-lived key
/// sessions: launch, first post, then every five minutes while active.
#[derive(Default)]
pub struct OfferSchedule {
    last: Option<u64>,
    first_post: bool,
}
impl OfferSchedule {
    pub fn due(&mut self, now: u64, posting: bool) -> Result<bool, Error> {
        if self.last.is_some_and(|last| now < last) {
            return Err(Error::Authority);
        }
        let due =
            self.last.is_none_or(|last| now - last >= 300_000) || (posting && !self.first_post);
        self.first_post |= posting;
        if due {
            self.last = Some(now);
        }
        Ok(due)
    }
}
pub struct Organizer {
    generation: [u8; 16],
    sender: [u8; 8],
    roots: Vec<Root>,
    cache: Vec<Cached>,
    recovery: Vec<Recovery>,
    request_times: Vec<(LinkHandle, u64)>,
    now: u64,
}
impl Organizer {
    pub fn new(own: &PublicIdentity) -> Result<Self, Error> {
        Ok(Self {
            generation: own
                .generation
                .as_slice()
                .try_into()
                .map_err(|_| Error::Invalid)?,
            sender: own
                .sender_id
                .as_slice()
                .try_into()
                .map_err(|_| Error::Invalid)?,
            roots: Vec::with_capacity(16),
            cache: Vec::with_capacity(128),
            recovery: Vec::with_capacity(32),
            request_times: Vec::with_capacity(8),
            now: 0,
        })
    }
    pub fn clear(&mut self) {
        self.cache.clear();
        self.recovery.clear();
        self.request_times.clear();
        self.roots.clear();
    }
    fn advance(&mut self, now: u64, wall: Option<i64>) -> Result<(), Error> {
        if now < self.now || now > u64::MAX - 900_000 {
            self.clear();
            return Err(Error::Authority);
        }
        self.now = now;
        self.cache.retain(|c| {
            now - c.used < 900_000
                && wall.is_none_or(|w| {
                    codec::credential(&c.raw).is_ok_and(|v| i64::from(v.not_after) >= w)
                })
        });
        self.recovery.retain(|r| r.expires > now);
        self.request_times.retain(|(_, t)| now - *t < 30_000);
        Ok(())
    }
    fn reload(&mut self, store: &EncryptedStore, wall: Option<i64>) -> Result<i64, Error> {
        if store.identity_generation()? != self.generation {
            self.clear();
            return Err(Error::Authority);
        }
        let time = store.authority_time(wall)?;
        let mut roots = Vec::with_capacity(16);
        for (key, value) in store.event_roots()? {
            if key.len() != 32
                || value.len() < 110
                || value.len() > 142
                || value[0] != 1
                || value[9] != 1
                || value[10..42] != key
            {
                return Err(Error::Invalid);
            }
            let expiry = u32::from_be_bytes(value[42..46].try_into().unwrap());
            if i64::from(expiry) < time {
                continue;
            }
            if roots.len() == 16 {
                return Err(Error::Capacity);
            }
            roots.push(Root {
                key: key.try_into().unwrap(),
                revision: u64::from_be_bytes(value[1..9].try_into().unwrap()),
                bundle: value[9..110].try_into().unwrap(),
                expiry,
            });
        }
        self.cache.iter_mut().for_each(|c| {
            if c.validated
                .is_some_and(|(key, rev)| !roots.iter().any(|r| r.key == key && r.revision == rev))
            {
                c.validated = None;
            }
        });
        self.roots = roots;
        Ok(time)
    }
    fn root(&self, id: &[u8]) -> Option<Root> {
        let key = friends::unique_signing_candidate(
            self.roots
                .iter()
                .filter(|r| hint(&r.key) == id)
                .map(|r| r.key),
        )?;
        self.roots.iter().find(|r| r.key == key).cloned()
    }
    /// Call only after the user explicitly confirms this exact event proposal.
    pub fn adopt(
        &mut self,
        ingress: &mut Ingress,
        store: &EncryptedStore,
        proposal: UnconfirmedProposal,
        now: u64,
        wall: Option<i64>,
    ) -> Result<bool, Error> {
        self.advance(now, wall)?;
        let time = self.reload(store, wall)?;
        let UnconfirmedProposal::Event {
            bundle,
            name,
            root_id,
        } = proposal
        else {
            return Err(Error::Invalid);
        };
        let key: [u8; 32] = bundle[1..33].try_into().unwrap();
        text::validate(&name, text::Kind::EventName).map_err(|_| Error::Invalid)?;
        if bundle[0] != 1
            || hint(&key) != root_id
            || i64::from(u32::from_be_bytes(bundle[33..37].try_into().unwrap())) < time
        {
            return Err(Error::Invalid);
        }
        let Some(permit) = ingress.begin_local_work(1, now)? else {
            return Ok(false);
        };
        let mut signed = domain("event-root");
        signed.extend_from_slice(&bundle[..37]);
        let valid = friends::verify(&key, &signed, &bundle[37..]);
        ingress.finish_work(permit)?;
        if !valid {
            return Err(Error::Invalid);
        }
        let mut value = bundle.to_vec();
        value.extend_from_slice(name.as_bytes());
        store.change_event_root(&key, Some(&value), wall)?;
        self.reload(store, wall)?;
        Ok(true)
    }
    pub fn remove(
        &mut self,
        store: &EncryptedStore,
        root: [u8; 32],
        wall: Option<i64>,
    ) -> Result<(), Error> {
        self.reload(store, wall)?;
        store.change_event_root(&root, None, wall)?;
        self.reload(store, wall)?;
        Ok(())
    }
    fn credential_valid(root: &Root, raw: &[u8], time: i64) -> bool {
        codec::credential(raw).is_ok_and(|c| {
            c.root_id == hint(&root.key)
                && i64::from(c.not_before) <= time
                && time <= i64::from(c.not_after)
                && c.not_after <= root.expiry
                && friends::public_key(c.staff_public_key).is_ok()
                && text::validate(c.label, text::Kind::CredentialLabel).is_ok()
        })
    }
    fn cache(&mut self, raw: &[u8], root: Option<&Root>, used: bool) {
        if let Some(c) = self.cache.iter_mut().find(|c| c.raw == raw) {
            if let Some(root) = root {
                c.validated = Some((root.key, root.revision));
            }
            if used {
                c.used = self.now;
            }
            return;
        }
        if self.cache.len() == 128 {
            let index = self
                .cache
                .iter()
                .enumerate()
                .min_by_key(|(_, c)| (c.validated.is_some(), c.used))
                .unwrap()
                .0;
            self.cache.remove(index);
        }
        self.cache.push(Cached {
            raw: raw.to_vec(),
            validated: root.map(|r| (r.key, r.revision)),
            used: self.now,
        });
    }
    fn cached(&self, root: &Root, staff: &[u8]) -> Option<(Vec<u8>, bool)> {
        let mut candidates = self.cache.iter().filter(|c| {
            codec::credential(&c.raw)
                .is_ok_and(|v| v.root_id == hint(&root.key) && hint(v.staff_public_key) == staff)
        });
        let c = candidates.next()?;
        if candidates.next().is_some() {
            return None;
        }
        Some((
            c.raw.clone(),
            c.validated == Some((root.key, root.revision)),
        ))
    }
    fn request_missing(&mut self, link: &LinkHandle, root: &[u8], staff: &[u8], expires: u64) {
        let mut pair = [0; 16];
        pair[..8].copy_from_slice(root);
        pair[8..].copy_from_slice(staff);
        if self
            .recovery
            .iter()
            .any(|r| r.link == *link && r.pair == pair)
        {
            return;
        }
        if self.recovery.iter().filter(|r| r.link == *link).count() >= 8 {
            let i = self.recovery.iter().position(|r| r.link == *link).unwrap();
            self.recovery.remove(i);
        }
        if self.recovery.len() == 32 {
            self.recovery.remove(0);
        }
        self.recovery.push(Recovery {
            pair,
            link: link.clone(),
            expires,
            attempts: 0,
            next: self.now,
        });
    }
    pub fn next_request(&mut self, friends: &Friends, now: u64) -> Result<Option<Request>, Error> {
        self.advance(now, None)?;
        self.recovery.retain(|r| friends.link_ready(&r.link));
        self.request_times.retain(|(l, _)| friends.link_ready(l));
        let Some(r) = self.recovery.iter_mut().find(|r| {
            r.attempts < 3
                && r.next <= now
                && !self
                    .request_times
                    .iter()
                    .any(|(l, t)| *l == r.link && now - *t < 5000)
        }) else {
            return Ok(None);
        };
        r.attempts += 1;
        r.next = now + 5000;
        if let Some((_, t)) = self.request_times.iter_mut().find(|(l, _)| *l == r.link) {
            *t = now;
        } else {
            if self.request_times.len() == 8 {
                return Ok(None);
            }
            self.request_times.push((r.link.clone(), now));
        }
        Ok(Some(Request {
            link: r.link.clone(),
            pair: r.pair,
        }))
    }
    /// Admitted logical traffic only. HELLO/proof admission remains Friends' owner.
    /// Response is a public credential to schedule as CRED_OFFER, never authority.
    #[allow(clippy::too_many_arguments)]
    pub fn receive(
        &mut self,
        ingress: &mut Ingress,
        friends: &Friends,
        store: &EncryptedStore,
        link: &LinkHandle,
        reported: u64,
        frame: &[u8],
        now: u64,
        wall: Option<i64>,
    ) -> Result<Intake, Error> {
        self.advance(now, wall)?;
        if !friends.link_ready(link) {
            return Err(Error::Authority);
        }
        let capacity = friends
            .receive_capacity(link)
            .map_err(|_| Error::Authority)?;
        let mut output = [0; 1035];
        let outcome =
            ingress.receive_friend(link, (reported, frame), now, &mut output, (false, capacity))?;
        let mut response = None;
        let mut job = None;
        if let Outcome::Complete {
            kind: framing::ObjectKind::Logical,
            len,
            state,
        } = outcome
        {
            if !matches!(state, State::Duplicate) {
                let raw = &output[..len];
                let packet = codec::parse(raw, codec::Context::Live).map_err(|_| Error::Invalid)?;
                match packet.payload() {
                    codec::Payload::CredentialOffer(c) => self.cache(c.raw, None, false),
                    codec::Payload::CredentialRequest {
                        root_id,
                        staff_key_id,
                    } => {
                        let matches: Vec<_> = self
                            .cache
                            .iter()
                            .enumerate()
                            .filter(|(_, c)| {
                                codec::credential(&c.raw).is_ok_and(|v| {
                                    v.root_id == root_id && hint(v.staff_public_key) == staff_key_id
                                })
                            })
                            .map(|(i, _)| i)
                            .collect();
                        if let [i] = matches.as_slice() {
                            self.cache[*i].used = now;
                            response = Some(self.cache[*i].raw.clone());
                        }
                    }
                    _ => {}
                }
                if matches!(state, State::Pending | State::PendingDuplicate) {
                    job = self.prepare_raw(ingress, store, link, raw, now, wall)?;
                }
            }
        }
        Ok(Intake {
            outcome,
            job,
            offer_response: response,
        })
    }
    #[allow(clippy::too_many_arguments)]
    pub fn retry(
        &mut self,
        ingress: &mut Ingress,
        friends: &Friends,
        store: &EncryptedStore,
        link: &LinkHandle,
        position: usize,
        now: u64,
        wall: Option<i64>,
    ) -> Result<Option<Job>, Error> {
        ingress.advance(now)?;
        if !friends.link_ready(link) {
            return Err(Error::Authority);
        }
        let Some(raw) = ingress.pending_bytes(position).map(<[u8]>::to_vec) else {
            return Ok(None);
        };
        self.prepare_raw(ingress, store, link, &raw, now, wall)
    }
    fn prepare_raw(
        &mut self,
        ingress: &mut Ingress,
        store: &EncryptedStore,
        link: &LinkHandle,
        raw: &[u8],
        now: u64,
        wall: Option<i64>,
    ) -> Result<Option<Job>, Error> {
        ingress.advance(now)?;
        self.advance(now, wall)?;
        match self.reload(store, wall) {
            Ok(_) => {}
            Err(Error::Storage(StorageError::ClockUncertain)) => return Ok(None),
            Err(e) => return Err(e),
        }
        let Some(expires) = ingress.pending_deadline(link, raw) else {
            return Ok(None);
        };
        let packet = codec::parse(
            raw,
            if raw[1] == 1 {
                codec::Context::StoredChat
            } else {
                codec::Context::Live
            },
        )
        .map_err(|_| Error::Invalid)?;
        let (root, credential, validated) = match packet.payload() {
            codec::Payload::Chat {
                signature:
                    codec::Signature::Organizer {
                        root_id,
                        staff_key_id,
                        credential,
                        ..
                    },
                ..
            } => {
                let Some(root) = self.root(root_id) else {
                    return Ok(None);
                };
                let (credential, validated) = if let Some(c) = credential {
                    (
                        c.raw.to_vec(),
                        self.cache.iter().any(|v| {
                            v.raw == c.raw && v.validated == Some((root.key, root.revision))
                        }),
                    )
                } else {
                    let Some(c) = self.cached(&root, staff_key_id) else {
                        self.request_missing(link, root_id, staff_key_id, expires);
                        return Ok(None);
                    };
                    c
                };
                (root, credential, validated)
            }
            codec::Payload::CredentialOffer(c) => {
                let Some(root) = self.root(c.root_id) else {
                    return Ok(None);
                };
                (root, c.raw.to_vec(), false)
            }
            _ => return Ok(None),
        };
        let units = if packet.header().kind == 8 || validated {
            1
        } else {
            2
        };
        Ok(ingress
            .begin_authentication(link, raw, units, now)?
            .map(|permit| Job {
                permit,
                link: link.clone(),
                raw: raw.to_vec(),
                root,
                credential,
                validated,
            }))
    }
    pub fn complete(
        &mut self,
        ingress: &mut Ingress,
        store: &EncryptedStore,
        job: Job,
        now: u64,
        wall: Option<i64>,
    ) -> Result<Option<Verified>, Error> {
        ingress.advance(now)?;
        ingress.finish_work(job.permit)?;
        self.advance(now, wall)?;
        let time = self.reload(store, wall)?;
        if !self
            .roots
            .iter()
            .any(|r| r.key == job.root.key && r.revision == job.root.revision)
        {
            return Err(Error::Authority);
        }
        if !Self::credential_valid(&job.root, &job.credential, time) {
            return Err(Error::Authority);
        }
        let c = codec::credential(&job.credential).map_err(|_| Error::Invalid)?;
        if !job.validated
            && !friends::verify(
                &job.root.key,
                &credential_transcript(c.raw),
                c.root_signature,
            )
        {
            self.cache.retain(|c| c.raw != job.credential);
            ingress.resolve_signature(&job.link, &job.raw, false)?;
            return Ok(None);
        }
        if job.raw[1] == 8 {
            self.cache(c.raw, Some(&job.root), true);
            ingress.resolve_signature(&job.link, &job.raw, true)?;
            return Ok(None);
        }
        let packet =
            codec::parse(&job.raw, codec::Context::StoredChat).map_err(|_| Error::Invalid)?;
        let codec::Payload::Chat {
            timestamp,
            signature:
                codec::Signature::Organizer {
                    root_id,
                    staff_key_id,
                    pin_state,
                    pin_expiry,
                    ..
                },
            ..
        } = packet.payload()
        else {
            return Err(Error::Invalid);
        };
        if root_id != hint(&job.root.key) || staff_key_id != hint(c.staff_public_key) {
            return Err(Error::Invalid);
        }
        let staff: [u8; 32] = c.staff_public_key.try_into().unwrap();
        if !friends::verify(
            &staff,
            &transcript(&job.raw),
            &job.raw[job.raw.len() - 64..],
        ) {
            ingress.resolve_signature(&job.link, &job.raw, false)?;
            return Ok(None);
        }
        if text::validate_payload(packet.payload()).is_err() {
            ingress.resolve_signature(&job.link, &job.raw, false)?;
            return Ok(None);
        }
        let authority = Authority {
            root: job.root.key,
            revision: job.root.revision,
            staff,
            credential: job.credential.clone(),
            generation: self.generation,
            // A signed pin outside authority bounds does not invalidate the
            // otherwise valid signed text. Keep raw bytes; suppress its pin.
            pin_expiry: (pin_state == 1
                && pin_expiry <= c.not_after
                && pin_expiry <= job.root.expiry)
                .then_some(pin_expiry),
        };
        let result = self.persist(store, &job.raw, &job.root, &authority, timestamp, 0, wall)?;
        ingress.resolve_signature(&job.link, &job.raw, true)?;
        if result == AcceptResult::Accepted {
            self.cache(c.raw, Some(&job.root), true);
        }
        Ok(Some(Verified { result, authority }))
    }
    #[allow(clippy::too_many_arguments)]
    fn persist(
        &self,
        store: &EncryptedStore,
        raw: &[u8],
        root: &Root,
        authority: &Authority,
        timestamp: u32,
        direction: u8,
        wall: Option<i64>,
    ) -> Result<AcceptResult, Error> {
        let mut subject = vec![3];
        subject.extend_from_slice(&root.key);
        subject.extend_from_slice(&authority.staff);
        let mut provenance = subject.clone();
        provenance.extend_from_slice(&root.bundle);
        provenance.extend_from_slice(&authority.credential);
        let mut immutable = raw.to_vec();
        immutable[3] = 0;
        Ok(store.accept_authenticated(
            HistoryItem {
                conversation: codec::EVENT_CHANNEL.to_vec(),
                direct: false,
                direction,
                logical_type: 1,
                message_id: raw[4..12].to_vec(),
                timestamp: i64::from(timestamp),
                body: raw.to_vec(),
                provenance,
            },
            subject,
            immutable,
            wall,
        )?)
    }
    /// Recheck before displaying current authority or a pin; stored signatures
    /// alone are historical evidence, and replay never reapplies a pin effect.
    pub fn current(
        &mut self,
        store: &EncryptedStore,
        authority: &Authority,
        wall: Option<i64>,
    ) -> Result<(bool, bool), Error> {
        let time = self.reload(store, wall)?;
        if authority.generation != self.generation {
            return Ok((false, false));
        }
        let Some(root) = self
            .roots
            .iter()
            .find(|r| r.key == authority.root && r.revision == authority.revision)
        else {
            return Ok((false, false));
        };
        let valid = Self::credential_valid(root, &authority.credential, time);
        Ok((
            valid,
            valid && authority.pin_expiry.is_some_and(|e| i64::from(e) >= time),
        ))
    }
    /// Trusted native provider invokes only after explicit staff import confirmation.
    /// Import/derivation and credential verification consume two local work units.
    pub fn import_staff(
        &mut self,
        ingress: &mut Ingress,
        store: &EncryptedStore,
        proposal: StaffProposal,
        now: u64,
        wall: Option<i64>,
    ) -> Result<Option<StaffSession>, Error> {
        self.advance(now, wall)?;
        let time = self.reload(store, wall)?;
        let (raw, mut seed) = proposal.into_import_parts();
        let c = codec::credential(&raw).map_err(|_| Error::Invalid)?;
        let root = self.root(c.root_id).ok_or(Error::Authority)?;
        if !Self::credential_valid(&root, &raw, time) {
            return Err(Error::Authority);
        }
        let Some(permit) = ingress.begin_local_work(2, now)? else {
            return Ok(None);
        };
        let key = SigningKey::from_bytes(&seed);
        seed.zeroize();
        let valid = key.verifying_key().as_bytes() == c.staff_public_key
            && friends::verify(&root.key, &credential_transcript(&raw), c.root_signature);
        ingress.finish_work(permit)?;
        if !valid {
            return Err(Error::Invalid);
        }
        let authority = Authority {
            root: root.key,
            revision: root.revision,
            staff: *key.verifying_key().as_bytes(),
            credential: raw,
            generation: self.generation,
            pin_expiry: None,
        };
        let mut record_key = authority.root.to_vec();
        record_key.extend_from_slice(&authority.staff);
        store.put_record(
            crate::storage::RecordKind::StaffCredential,
            record_key,
            authority.credential.clone(),
        )?;
        Ok(Some(StaffSession {
            key: Mutex::new(Some((key, false))),
            authority,
        }))
    }
    /// Signs a structurally canonical organizer CHAT whose final signature is a
    /// placeholder. A fresh protected session must be supplied for each operation.
    #[allow(clippy::too_many_arguments)]
    pub fn sign(
        &mut self,
        ingress: &mut Ingress,
        friends: &Friends,
        store: &EncryptedStore,
        session: &StaffSession,
        link: &LinkHandle,
        mut raw: Vec<u8>,
        now: u64,
        wall: Option<i64>,
    ) -> Result<Option<Vec<u8>>, Error> {
        self.advance(now, wall)?;
        let (current, _) = self.current(store, &session.authority, wall)?;
        if !current || !friends.link_ready(link) {
            return Err(Error::Authority);
        }
        let packet = codec::parse(&raw, codec::Context::Live).map_err(|_| Error::Invalid)?;
        text::validate_payload(packet.payload()).map_err(|_| Error::Invalid)?;
        let codec::Payload::Chat {
            timestamp,
            signature:
                codec::Signature::Organizer {
                    root_id,
                    staff_key_id,
                    credential,
                    pin_state,
                    pin_expiry,
                    ..
                },
            ..
        } = packet.payload()
        else {
            return Err(Error::Invalid);
        };
        let root = self.root(root_id).ok_or(Error::Authority)?;
        let c = codec::credential(session.credential()).map_err(|_| Error::Invalid)?;
        if packet.header().sender_id != self.sender
            || root.key != session.authority.root
            || root.revision != session.authority.revision
            || staff_key_id != hint(&session.authority.staff)
            || credential.is_some_and(|included| included.raw != session.credential())
            || (pin_state == 1 && (pin_expiry > c.not_after || pin_expiry > root.expiry))
        {
            return Err(Error::Invalid);
        }
        let mut locked = session.key.lock().map_err(|_| Error::Provider)?;
        let (key, posted) = locked.as_mut().ok_or(Error::Provider)?;
        if !*posted && credential.is_none() {
            return Err(Error::Invalid);
        }
        let Some(permit) = ingress.begin_work(link, 1, now)? else {
            return Ok(None);
        };
        let signature = key.sign(&transcript(&raw)).to_bytes();
        ingress.finish_work(permit)?;
        let end = raw.len();
        raw[end - 64..].copy_from_slice(&signature);
        let mut authority = session.authority.clone();
        authority.pin_expiry = (pin_state == 1).then_some(pin_expiry);
        if self.persist(store, &raw, &root, &authority, timestamp, 1, wall)?
            != AcceptResult::Accepted
        {
            return Err(Error::Authority);
        }
        *posted = true;
        Ok(Some(raw))
    }
    pub fn cache_count(&self) -> usize {
        self.cache.len()
    }
    pub fn recovery_count(&self) -> usize {
        self.recovery.len()
    }
    pub fn reserved_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.roots.capacity() * std::mem::size_of::<Root>()
            + self.cache.capacity() * (std::mem::size_of::<Cached>() + 130)
            + self.recovery.capacity() * std::mem::size_of::<Recovery>()
            + self.request_times.capacity() * std::mem::size_of::<(LinkHandle, u64)>()
            + 2 * (std::mem::size_of::<Job>() + 556 + 130)
    }
}
/// Canonical public recovery controls. Delivery still requires normal scheduling.
pub fn offer(raw: &[u8], id: [u8; 8], sender: [u8; 8]) -> Result<Vec<u8>, Error> {
    codec::credential(raw).map_err(|_| Error::Invalid)?;
    let mut out = vec![1, 8, 0, 7];
    out.extend_from_slice(&id);
    out.extend_from_slice(&sender);
    out.extend_from_slice(&codec::EVENT_CHANNEL);
    out.extend_from_slice(&(raw.len() as u16).to_be_bytes());
    out.extend_from_slice(raw);
    Ok(out)
}
pub fn request(pair: [u8; 16], id: [u8; 8], sender: [u8; 8]) -> Vec<u8> {
    let mut out = vec![1, 7, 0, 1];
    out.extend_from_slice(&id);
    out.extend_from_slice(&sender);
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&16u16.to_be_bytes());
    out.extend_from_slice(&pair);
    out
}
