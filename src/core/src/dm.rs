//! Authenticated encrypted messages. The caller owns native provider lifetimes,
//! admitted HELLO links and monotonic time including suspend; no private exports.
use crate::{
    LinkHandle, codec,
    friends::{Friends, Pin, SendToken},
    identity::{IdentityKeySession, PublicIdentity},
    ingress::{Ingress, WorkPermit},
    storage::{AcceptResult, EncryptedStore, HistoryItem, StorageError},
    text,
};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid encrypted input")]
    Invalid,
    #[error("encryption authentication failed")]
    Authentication,
    #[error("recipient tag did not match")]
    Tag,
    #[error("OS entropy unavailable or unexpected consumption")]
    Entropy,
    #[error("protected provider unavailable")]
    Provider,
    #[error("identity or pin changed")]
    Stale,
    #[error("ambiguous friend hints")]
    Ambiguous,
    #[error("friend state refused")]
    Friend(#[from] crate::friends::Error),
    #[error("intake refused")]
    Ingress(#[from] crate::ingress::Error),
    #[error("persistence refused")]
    Storage(#[from] StorageError),
}
pub(crate) fn info(sender: &[u8; 64], recipient: &[u8; 64]) -> Vec<u8> {
    let mut out = b"meshfest/dm-info/v1\0".to_vec();
    out.extend_from_slice(&[2, 0, 32, 0, 1, 0, 3]);
    out.extend_from_slice(sender);
    out.extend_from_slice(recipient);
    out
}
pub(crate) fn aad(raw: &[u8]) -> Vec<u8> {
    let mut out = b"meshfest/dm-aad/v1\0".to_vec();
    out.extend_from_slice(&raw[..3]);
    out.extend_from_slice(&raw[4..67]);
    out
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Content {
    Chat(String),
    Reaction {
        target: [u8; 8],
        remove: bool,
        code: u8,
    },
}
impl Content {
    fn encode(&self, timestamp: u32) -> Result<Zeroizing<Vec<u8>>, Error> {
        let mut out = Zeroizing::new(timestamp.to_be_bytes().to_vec());
        match self {
            Self::Chat(s) => {
                text::validate(s, text::Kind::Message).map_err(|_| Error::Invalid)?;
                out.extend_from_slice(&(s.len() as u16).to_be_bytes());
                out.extend_from_slice(s.as_bytes());
                let n = bucket(out.len())?;
                out.resize(n, 0);
            }
            Self::Reaction {
                target,
                remove,
                code,
            } => {
                out.extend_from_slice(target);
                out.push(u8::from(*remove));
                out.push(*code);
                out.resize(64, 0);
            }
        }
        Ok(out)
    }
    fn decode(kind: u8, raw: &[u8]) -> Result<(u32, Self), Error> {
        if ![64, 144, 304].contains(&raw.len()) {
            return Err(Error::Invalid);
        }
        let timestamp = u32::from_be_bytes(raw[..4].try_into().unwrap());
        let (used, content) = match kind {
            1 => {
                let n = usize::from(u16::from_be_bytes(raw[4..6].try_into().unwrap()));
                let end = 6 + n;
                let s = std::str::from_utf8(raw.get(6..end).ok_or(Error::Invalid)?)
                    .map_err(|_| Error::Invalid)?;
                text::validate(s, text::Kind::Message).map_err(|_| Error::Invalid)?;
                if bucket(end)? != raw.len() {
                    return Err(Error::Invalid);
                }
                (end, Self::Chat(s.into()))
            }
            6 if raw.len() == 64 => (
                14,
                Self::Reaction {
                    target: raw[4..12].try_into().unwrap(),
                    remove: raw[12] & 1 != 0,
                    code: raw[13],
                },
            ),
            _ => return Err(Error::Invalid),
        };
        if raw[used..].iter().any(|x| *x != 0) {
            return Err(Error::Invalid);
        }
        Ok((timestamp, content))
    }
    fn kind(&self) -> u8 {
        match self {
            Self::Chat(_) => 1,
            Self::Reaction { .. } => 6,
        }
    }
}
fn bucket(n: usize) -> Result<usize, Error> {
    [64, 144, 304]
        .into_iter()
        .find(|x| *x >= n)
        .ok_or(Error::Invalid)
}
fn epoch(wall: Option<i64>) -> Result<u64, Error> {
    Ok(u64::try_from(wall.ok_or(StorageError::ClockUncertain)?)
        .map_err(|_| StorageError::ClockUncertain)?
        / 3600)
}
pub struct Job {
    permit: WorkPermit,
    link: LinkHandle,
    raw: Vec<u8>,
    pin: Pin,
}
pub struct Sent {
    raw: Vec<u8>,
    pin: SendToken,
}
impl Sent {
    pub fn bytes(&self) -> &[u8] {
        &self.raw
    }
    pub fn pin(&self) -> &SendToken {
        &self.pin
    }
}
#[derive(Debug)]
pub struct Received {
    pub pin: Pin,
    pub result: AcceptResult,
    pub content: Option<Content>,
}
struct Orphan {
    pin: SendToken,
    id: [u8; 8],
    direction: u8,
    link: LinkHandle,
    expires: u64,
}
pub struct Dms {
    own: [u8; 64],
    generation: [u8; 16],
    orphans: Vec<Orphan>,
    now: u64,
}
/// Resolve only a bounded bucket already filtered by BOTH sender hints. No
/// nickname, tag collision or arrival-order preference can resolve ambiguity.
pub fn unique_tuple(candidates: impl Iterator<Item = [u8; 64]>) -> Result<Option<[u8; 64]>, Error> {
    let mut found = None;
    for next in candidates {
        if found.is_some_and(|old| old != next) {
            return Err(Error::Ambiguous);
        }
        found = Some(next)
    }
    Ok(found)
}
impl Dms {
    pub fn new(own: &PublicIdentity) -> Result<Self, Error> {
        let mut tuple = [0; 64];
        tuple[..32].copy_from_slice(
            &<[u8; 32]>::try_from(own.signing_key.as_slice()).map_err(|_| Error::Invalid)?,
        );
        tuple[32..].copy_from_slice(
            &<[u8; 32]>::try_from(own.agreement_key.as_slice()).map_err(|_| Error::Invalid)?,
        );
        Ok(Self {
            own: tuple,
            generation: own
                .generation
                .as_slice()
                .try_into()
                .map_err(|_| Error::Invalid)?,
            orphans: Vec::with_capacity(256),
            now: 0,
        })
    }
    fn check(
        &mut self,
        store: &EncryptedStore,
        provider: &IdentityKeySession,
    ) -> Result<(), Error> {
        if !provider.matches_generation(&self.generation)
            || store.identity_generation()? != self.generation
        {
            self.clear();
            return Err(Error::Stale);
        }
        Ok(())
    }
    /// Only existing admitted encrypted pending bytes can acquire crypto work.
    /// Call Friends::receive for live intake; SYNC uses its ordered MC-015 owner.
    pub fn prepare(
        &self,
        ingress: &mut Ingress,
        friends: &mut Friends,
        store: &EncryptedStore,
        link: &LinkHandle,
        position: usize,
        now: u64,
    ) -> Result<Option<Job>, Error> {
        if !friends.link_ready(link) {
            return Err(Error::Stale);
        }
        friends.reload(store)?;
        if !friends.link_ready(link) {
            return Err(Error::Stale);
        }
        let Some(raw) = ingress.pending_bytes(position).map(<[u8]>::to_vec) else {
            return Ok(None);
        };
        let packet = codec::parse(
            &raw,
            if raw[1] == 1 {
                codec::Context::StoredChat
            } else {
                codec::Context::Live
            },
        )
        .map_err(|_| Error::Invalid)?;
        let codec::Payload::Encrypted { recipient_hint, .. } = packet.payload() else {
            return Ok(None);
        };
        let tuple = unique_tuple(
            friends
                .pins()
                .iter()
                .filter(|p| !p.replacing())
                .map(|p| *p.tuple())
                .filter(|p| {
                    Sha256::digest(&p[..32])[..8] == packet.header().sender_id
                        && Sha256::digest(&p[32..])[..8] == *recipient_hint
                }),
        )?;
        let Some(tuple) = tuple else { return Ok(None) };
        let pin = friends
            .pins()
            .iter()
            .find(|p| *p.tuple() == tuple)
            .ok_or(Error::Stale)?
            .clone();
        let Some(permit) = ingress.begin_authentication(link, &raw, 8, now)? else {
            return Ok(None);
        };
        Ok(Some(Job {
            permit,
            link: link.clone(),
            raw,
            pin,
        }))
    }
    #[allow(clippy::too_many_arguments)]
    pub fn complete(
        &mut self,
        ingress: &mut Ingress,
        friends: &mut Friends,
        store: &EncryptedStore,
        provider: &IdentityKeySession,
        job: Job,
        now: u64,
        wall: Option<i64>,
    ) -> Result<Received, Error> {
        ingress.advance(now)?;
        ingress.finish_work(job.permit)?;
        self.advance(now)?;
        self.check(store, provider)
            .inspect_err(|_| friends.clear_session_trust())?;
        let peer = friends.validate_send(store, &job.pin.token())?;
        let plaintext = match provider.dm_open(&self.own, &peer, &job.raw, epoch(wall)?) {
            Ok(p) => p,
            Err(Error::Authentication | Error::Invalid) => {
                ingress.resolve_signature(&job.link, &job.raw, false)?;
                return Err(Error::Authentication);
            }
            Err(e) => return Err(e),
        };
        let (timestamp, content) = match Content::decode(job.raw[1], &plaintext) {
            Ok(value) => value,
            Err(e) => {
                ingress.resolve_signature(&job.link, &job.raw, false)?;
                return Err(e);
            }
        };
        let mut immutable = job.raw.clone();
        immutable[3] = 0;
        let mut subject = vec![2];
        subject.extend_from_slice(&peer);
        let result = store.accept_dm(
            HistoryItem {
                conversation: peer.to_vec(),
                direct: true,
                direction: 0,
                logical_type: job.raw[1],
                message_id: job.raw[4..12].to_vec(),
                timestamp: i64::from(timestamp),
                body: plaintext.to_vec(),
                provenance: subject.clone(),
            },
            subject,
            immutable,
            wall,
        )?;
        let display = if result == AcceptResult::Accepted {
            self.reaction_or_orphan(
                store,
                &job.pin.token(),
                &peer,
                job.raw[4..12].try_into().unwrap(),
                0,
                &job.link,
                now,
                &content,
            )?
        } else {
            false
        };
        ingress.resolve_signature(&job.link, &job.raw, true)?;
        Ok(Received {
            pin: job.pin,
            content: display.then_some(content),
            result,
        })
    }
    /// Commits encrypted-local outgoing history before returning a sendable wire
    /// object. Native egress must revalidate its pin immediately before sending.
    #[allow(clippy::too_many_arguments)]
    pub fn send(
        &mut self,
        ingress: &mut Ingress,
        friends: &mut Friends,
        store: &EncryptedStore,
        provider: &IdentityKeySession,
        link: &LinkHandle,
        pin: &SendToken,
        id: [u8; 8],
        content: Content,
        now: u64,
        wall: Option<i64>,
    ) -> Result<Option<Sent>, Error> {
        self.advance(now)?;
        self.check(store, provider)
            .inspect_err(|_| friends.clear_session_trust())?;
        if !friends.link_ready(link) {
            return Err(Error::Stale);
        }
        let peer = friends.validate_send(store, pin)?;
        let timestamp =
            u32::try_from(wall.ok_or(StorageError::ClockUncertain)?).map_err(|_| Error::Invalid)?;
        let plain = content.encode(timestamp)?;
        let Some(permit) = ingress.begin_work(link, 7, now)? else {
            return Ok(None);
        };
        let mut header = [0; 26];
        header[..4].copy_from_slice(&[1, content.kind(), 1, 7]);
        header[4..12].copy_from_slice(&id);
        header[12..20].copy_from_slice(&Sha256::digest(&self.own[..32])[..8]);
        header[24..26].copy_from_slice(&((plain.len() + 57) as u16).to_be_bytes());
        let sealed = provider.dm_seal(&self.own, &peer, &mut header, &plain, epoch(wall)?);
        ingress.finish_work(permit)?;
        let raw = sealed?;
        let mut immutable = raw.clone();
        immutable[3] = 0;
        let mut subject = vec![2];
        subject.extend_from_slice(&peer);
        let result = store.accept_dm(
            HistoryItem {
                conversation: peer.to_vec(),
                direct: true,
                direction: 1,
                logical_type: content.kind(),
                message_id: id.to_vec(),
                timestamp: i64::from(timestamp),
                body: plain.to_vec(),
                provenance: subject.clone(),
            },
            subject,
            immutable,
            wall,
        )?;
        if result != AcceptResult::Accepted {
            return Err(Error::Stale);
        }
        self.reaction_or_orphan(store, pin, &peer, id, 1, link, now, &content)?;
        Ok(Some(Sent {
            raw,
            pin: pin.clone(),
        }))
    }
}

impl Drop for Content {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        if let Self::Chat(s) = self {
            s.zeroize();
        }
    }
}
impl Dms {
    pub fn clear(&mut self) {
        self.orphans.clear();
    }
    fn advance(&mut self, now: u64) -> Result<(), Error> {
        if now < self.now || now > u64::MAX - 120_000 {
            self.clear();
            return Err(Error::Stale);
        }
        self.now = now;
        self.orphans.retain(|o| o.expires > now);
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    fn reaction_or_orphan(
        &mut self,
        store: &EncryptedStore,
        pin: &SendToken,
        peer: &[u8; 64],
        id: [u8; 8],
        direction: u8,
        link: &LinkHandle,
        now: u64,
        content: &Content,
    ) -> Result<bool, Error> {
        if !matches!(content, Content::Reaction { .. }) {
            return Ok(true);
        }
        if store.recover_dm_reaction(peer, &id, direction)? {
            return Ok(true);
        }
        if self.orphans.iter().filter(|o| o.link == *link).count() >= 32 {
            let i = self.orphans.iter().position(|o| o.link == *link).unwrap();
            self.orphans.remove(i);
        }
        if self.orphans.len() == 256 {
            self.orphans.remove(0);
        }
        self.orphans.push(Orphan {
            pin: pin.clone(),
            id,
            direction,
            link: link.clone(),
            expires: now + 120_000,
        });
        Ok(false)
    }
    /// Reconstructs visible reactions from committed state. Orphans are volatile,
    /// bounded and never revived on restart or refreshed by authenticated replay.
    pub fn reactions(
        &mut self,
        store: &EncryptedStore,
        friends: &mut Friends,
        pin: &SendToken,
        target: [u8; 8],
        now: u64,
        wall: Option<i64>,
    ) -> Result<Vec<(u8, u8)>, Error> {
        self.advance(now)?;
        let peer = friends.validate_send(store, pin)?;
        let mut i = 0;
        while i < self.orphans.len() {
            let o = &self.orphans[i];
            if &o.pin != pin {
                i += 1;
                continue;
            }
            if store.recover_dm_reaction(&peer, &o.id, o.direction)? {
                self.orphans.remove(i);
            } else {
                i += 1;
            }
        }
        Ok(store.dm_reactions(&peer, &target, wall)?)
    }
    pub fn orphan_count(&self) -> usize {
        self.orphans.len()
    }
    pub fn reserved_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.orphans.capacity() * std::mem::size_of::<Orphan>()
            + 2 * (std::mem::size_of::<Job>() + 387)
    }
}
