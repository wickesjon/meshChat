//! Public-channel feature boundary. Native callers supply protected store operations
//! and budget-admitted transport events; this owner never grants cryptographic trust.
use crate::{
    LinkHandle,
    channel::{self, Channel, Public},
    codec::{self, Context, Header, Payload},
    identity::PublicIdentity,
    native_transport::TransportIntake,
    storage::{EncryptedStore, HistoryItem},
    text,
};
use std::sync::{Arc, Mutex};

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum ChannelError {
    #[error("invalid channel, text or message")]
    Invalid,
    #[error("protected store or identity unavailable")]
    Unavailable,
    #[error("posting rate limited")]
    Limited,
}
#[derive(Clone, Debug, uniffi::Record)]
pub struct ChannelReceipt {
    pub link: Option<LinkHandle>,
    pub bytes: Vec<u8>,
    pub intake: TransportIntake,
    pub own: bool,
    pub wall: i64,
    pub now: u64,
}
#[derive(Clone, Debug, uniffi::Record)]
pub struct ChannelInfo {
    pub name: String,
    pub id: Vec<u8>,
    pub glyph: String,
    pub private: bool,
    pub anonymous: bool,
}
#[derive(Clone, Debug, uniffi::Record)]
pub struct ChannelMessage {
    pub id: Vec<u8>,
    pub sender: Vec<u8>,
    pub nickname: String,
    pub text: String,
    pub avatar: u8,
    pub own: bool,
    pub confusable: bool,
    pub claimed_timestamp: u32,
    pub arrival: i64,
    pub reactions: Vec<u16>,
    pub own_reaction: Option<u8>,
    pub verified_petname: Option<String>,
    pub signed: bool,
    pub claim_warning: Option<String>,
}
#[derive(Clone, Debug, uniffi::Record)]
pub struct ChannelWords {
    pub descriptors: Vec<String>,
    pub genres: Vec<String>,
    pub locations: Vec<String>,
}
fn info(c: Channel) -> ChannelInfo {
    ChannelInfo {
        name: c.canonical_name(),
        id: c.id().to_vec(),
        glyph: c.glyph().into(),
        private: matches!(c, Channel::Private(_)),
        anonymous: c == Channel::Public(Public::Confessions),
    }
}
fn parse(name: &str) -> Result<Channel, ChannelError> {
    channel::parse_name(name).map_err(|_| ChannelError::Invalid)
}
#[uniffi::export]
pub fn channel_info(name: String) -> Result<ChannelInfo, ChannelError> {
    Ok(info(parse(&name)?))
}
/// An inert proposal. Only explicit native confirmation may join this channel.
#[uniffi::export]
pub fn channel_link(uri: String) -> Result<ChannelInfo, ChannelError> {
    match crate::links::parse(&uri).map_err(|_| ChannelError::Invalid)? {
        crate::links::UnconfirmedProposal::Channel(channel) => Ok(info(channel)),
        _ => Err(ChannelError::Invalid),
    }
}
#[uniffi::export]
pub fn channel_words() -> ChannelWords {
    let strings = |v: [&str; 20]| v.into_iter().map(String::from).collect();
    ChannelWords {
        descriptors: strings(channel::DESCRIPTORS),
        genres: strings(channel::GENRES),
        locations: strings(channel::LOCATIONS),
    }
}
#[uniffi::export]
pub fn channel_nickname(raw: String) -> Result<String, ChannelError> {
    text::outgoing(&raw, text::Kind::Nickname).map_err(|_| ChannelError::Invalid)
}
fn packet(
    kind: u8,
    sender: [u8; 8],
    channel: [u8; 4],
    payload: &[u8],
) -> Result<Vec<u8>, ChannelError> {
    let mut id = [0; 8];
    getrandom::fill(&mut id).map_err(|_| ChannelError::Unavailable)?;
    let mut bytes = vec![0; 1024];
    let size = codec::serialize(
        Header {
            kind,
            flags: 0,
            ttl: if kind == 2 { 1 } else { 7 },
            message_id: id,
            sender_id: sender,
            channel_id: channel,
        },
        payload,
        Context::Live,
        &mut bytes,
    )
    .map_err(|_| ChannelError::Invalid)?;
    bytes.truncate(size);
    Ok(bytes)
}
struct Bucket {
    capacity: u64,
    interval: u64,
    credit: u64,
    at: u64,
}
impl Bucket {
    fn new(capacity: u64, interval: u64, at: u64) -> Self {
        Self {
            capacity,
            interval,
            credit: capacity * interval,
            at,
        }
    }
    fn wait(&mut self, now: u64) -> u64 {
        self.credit = self
            .credit
            .saturating_add(now - self.at)
            .min(self.capacity * self.interval);
        self.at = now;
        self.interval.saturating_sub(self.credit)
    }
    fn take(&mut self) {
        self.credit -= self.interval;
    }
}
struct Orphan {
    link: LinkHandle,
    bytes: Vec<u8>,
    until: u64,
    wall: i64,
}
struct State {
    now: u64,
    buckets: [Bucket; 5],
    orphans: Vec<Orphan>,
}
#[derive(uniffi::Object)]
pub struct NativeChannels {
    generation: Vec<u8>,
    sender: [u8; 8],
    state: Mutex<State>,
}
impl NativeChannels {
    fn store(&self, store: &EncryptedStore) -> Result<(), ChannelError> {
        if store
            .identity_generation()
            .map_err(|_| ChannelError::Unavailable)?
            != self.generation
        {
            return Err(ChannelError::Unavailable);
        }
        Ok(())
    }
    fn state(&self, now: u64) -> Result<std::sync::MutexGuard<'_, State>, ChannelError> {
        let mut s = self.state.lock().map_err(|_| ChannelError::Unavailable)?;
        if now < s.now || now > u64::MAX - 900_000 {
            return Err(ChannelError::Unavailable);
        }
        s.now = now;
        s.orphans.retain(|o| now < o.until);
        Ok(s)
    }
    fn class(c: Channel) -> usize {
        match c {
            Channel::Public(Public::EventUpdates) => 1,
            Channel::Private(_) => 2,
            _ => 0,
        }
    }
    fn save(
        &self,
        store: &EncryptedStore,
        bytes: &[u8],
        own: bool,
        wall: i64,
    ) -> Result<(), ChannelError> {
        let p = codec::parse(bytes, Context::Live).map_err(|_| ChannelError::Invalid)?;
        let h = p.header();
        store
            .append_unverified(
                HistoryItem {
                    conversation: h.channel_id.to_vec(),
                    direct: false,
                    direction: u8::from(own),
                    logical_type: h.kind,
                    message_id: h.message_id.to_vec(),
                    timestamp: wall,
                    body: bytes.to_vec(),
                    provenance: vec![],
                },
                Some(wall),
            )
            .map_err(|_| ChannelError::Unavailable)
    }
}
#[uniffi::export]
impl NativeChannels {
    #[uniffi::constructor]
    pub fn new(identity: PublicIdentity, now: u64) -> Result<Self, ChannelError> {
        if identity.generation.len() != 16 || now > u64::MAX - 900_000 {
            return Err(ChannelError::Invalid);
        }
        let sender = identity
            .sender_id
            .try_into()
            .map_err(|_| ChannelError::Invalid)?;
        Ok(Self {
            generation: identity.generation,
            sender,
            state: Mutex::new(State {
                now,
                orphans: vec![],
                buckets: [
                    Bucket::new(5, 12_000, now),
                    Bucket::new(2, 30_000, now),
                    Bucket::new(15, 4_000, now),
                    Bucket::new(20, 3_000, now),
                    Bucket::new(30, 2_000, now),
                ],
            }),
        })
    }
    pub fn wait_ms(&self, name: String, reaction: bool, now: u64) -> Result<u64, ChannelError> {
        let c = parse(&name)?;
        let mut s = self.state(now)?;
        Ok(if reaction {
            s.buckets[4].wait(now)
        } else {
            s.buckets[Self::class(c)]
                .wait(now)
                .max(s.buckets[3].wait(now))
        })
    }
    pub fn compose(
        &self,
        name: String,
        nickname: String,
        avatar: u8,
        message: String,
        wall: u32,
        now: u64,
    ) -> Result<Vec<u8>, ChannelError> {
        let c = parse(&name)?;
        let anonymous = c == Channel::Public(Public::Confessions);
        let nick = if anonymous {
            "Anonymous".into()
        } else {
            channel_nickname(nickname)?
        };
        let value =
            text::outgoing(&message, text::Kind::Message).map_err(|_| ChannelError::Invalid)?;
        let mut sender = self.sender;
        if anonymous {
            getrandom::fill(&mut sender).map_err(|_| ChannelError::Unavailable)?;
        }
        let mut payload = wall.to_be_bytes().to_vec();
        payload.extend([if anonymous { 0 } else { avatar }, nick.len() as u8]);
        payload.extend(nick.as_bytes());
        payload.extend([0; 4]);
        payload.extend((value.len() as u16).to_be_bytes());
        payload.extend(value.as_bytes());
        let bytes = packet(1, sender, c.id(), &payload)?;
        let mut s = self.state(now)?;
        let class = Self::class(c);
        if s.buckets[class].wait(now).max(s.buckets[3].wait(now)) > 0 {
            return Err(ChannelError::Limited);
        }
        s.buckets[class].take();
        s.buckets[3].take();
        Ok(bytes)
    }
    pub fn reaction(
        &self,
        name: String,
        target: Vec<u8>,
        code: u8,
        remove: bool,
        now: u64,
    ) -> Result<Vec<u8>, ChannelError> {
        let c = parse(&name)?;
        if target.len() != 8 || code >= 8 {
            return Err(ChannelError::Invalid);
        }
        let mut payload = target;
        payload.extend([u8::from(remove), code]);
        let bytes = packet(6, self.sender, c.id(), &payload)?;
        let mut s = self.state(now)?;
        if s.buckets[4].wait(now) > 0 {
            return Err(ChannelError::Limited);
        }
        s.buckets[4].take();
        Ok(bytes)
    }
    pub fn announce(
        &self,
        nickname: String,
        avatar: u8,
        peers: u8,
        wall: u32,
    ) -> Result<Vec<u8>, ChannelError> {
        let nick = channel_nickname(nickname)?;
        let mut payload = wall.to_be_bytes().to_vec();
        payload.extend([avatar, peers, 0, nick.len() as u8]);
        payload.extend(nick.as_bytes());
        payload.extend([0; 6]);
        packet(2, self.sender, [0; 4], &payload)
    }
    /// Only budget-admitted, non-duplicate events from the current transport owner.
    /// Pending signed/encrypted work belongs to its authentication owner and may
    /// not be persisted or displayed by this unsigned-channel boundary.
    pub fn accept(
        &self,
        store: Arc<EncryptedStore>,
        receipt: ChannelReceipt,
    ) -> Result<bool, ChannelError> {
        let ChannelReceipt {
            link,
            bytes,
            intake,
            own,
            wall,
            now,
        } = receipt;
        self.store(&store)?;
        let mut s = self.state(now)?;
        if intake != TransportIntake::Unverified {
            return Ok(false);
        }
        let p = codec::parse(&bytes, Context::Live).map_err(|_| ChannelError::Invalid)?;
        if p.header().flags & 7 != 0 {
            return Ok(false);
        }
        text::validate_payload(p.payload()).map_err(|_| ChannelError::Invalid)?;
        let h = p.header();
        if !matches!(p.payload(), Payload::Chat { .. } | Payload::Reaction { .. }) {
            return Ok(false);
        }
        let held = store
            .history(h.channel_id.to_vec(), false, 100)
            .map_err(|_| ChannelError::Unavailable)?;
        // History uses local insertion order. A backward wall
        // clock adjustment must not place a new arrival before an older one.
        let wall = held.first().map_or(wall, |row| wall.max(row.timestamp));
        if held.iter().any(|x| {
            x.message_id == h.message_id
                && x.logical_type == h.kind
                && x.body.get(12..20) == Some(&h.sender_id)
        }) {
            return Ok(false);
        }
        if let Payload::Reaction { target, .. } = p.payload() {
            let targets = held
                .iter()
                .filter(|x| x.logical_type == 1 && x.message_id == target)
                .count();
            if targets != 1 {
                if targets == 0 && !own {
                    if let Some(link) = link {
                        if !s.orphans.iter().any(|o| o.bytes == bytes) {
                            if s.orphans.iter().filter(|o| o.link == link).count() == 32 {
                                let at = s.orphans.iter().position(|o| o.link == link).unwrap();
                                s.orphans.remove(at);
                            }
                            if s.orphans.len() == 256 {
                                s.orphans.remove(0);
                            }
                            s.orphans.push(Orphan {
                                link,
                                bytes,
                                until: now + 120_000,
                                wall,
                            });
                        }
                    }
                }
                return Ok(false);
            }
        }
        self.save(&store, &bytes, own, wall)?;
        if h.kind == 1 {
            let mut i = 0;
            while i < s.orphans.len() {
                let o = &s.orphans[i];
                let matches = codec::parse(&o.bytes, Context::Live).is_ok_and(|r| r.header().channel_id == h.channel_id && matches!(r.payload(), Payload::Reaction { target, .. } if target == h.message_id));
                if matches {
                    self.save(&store, &o.bytes, false, o.wall)?;
                    s.orphans.remove(i);
                } else {
                    i += 1;
                }
            }
        }
        Ok(true)
    }
    pub fn disconnected(&self, link: LinkHandle, now: u64) -> Result<(), ChannelError> {
        self.state(now)?.orphans.retain(|o| o.link != link);
        Ok(())
    }
    pub fn history(
        &self,
        store: Arc<EncryptedStore>,
        name: String,
        own_nickname: String,
    ) -> Result<Vec<ChannelMessage>, ChannelError> {
        self.history_impl(store, name, own_nickname, None)
    }
}
impl NativeChannels {
    pub(crate) fn authenticated_history(
        &self,
        store: Arc<EncryptedStore>,
        name: String,
        nickname: String,
        pins: &[crate::friends::Pin],
    ) -> Result<Vec<ChannelMessage>, ChannelError> {
        self.history_impl(store, name, nickname, Some(pins))
    }
    fn history_impl(
        &self,
        store: Arc<EncryptedStore>,
        name: String,
        own_nickname: String,
        pins: Option<&[crate::friends::Pin]>,
    ) -> Result<Vec<ChannelMessage>, ChannelError> {
        self.store(&store)?;
        let c = parse(&name)?;
        let anonymous = c == Channel::Public(Public::Confessions);
        let mut rows = store
            .history(c.id().to_vec(), false, 100)
            .map_err(|_| ChannelError::Unavailable)?;
        rows.reverse();
        let mut out = vec![];
        for row in &rows {
            let Ok(p) = codec::parse(&row.body, Context::Live) else {
                continue;
            };
            if p.header().channel_id != c.id() {
                continue;
            }
            let signed = p.header().flags & 7 == 2
                && pins.is_some()
                && !anonymous
                && row.provenance.len() == 33
                && row.provenance[0] == 1;
            if p.header().flags & 7 != 0 && !signed {
                continue;
            }
            let candidates = pins
                .unwrap_or(&[])
                .iter()
                .filter(|pin| {
                    signed && !pin.replacing() && pin.tuple()[..32] == row.provenance[1..]
                })
                .collect::<Vec<_>>();
            let verified_petname = if candidates.len() == 1 {
                Some(candidates[0].petname().to_string())
            } else {
                None
            };
            if let Payload::Chat {
                timestamp,
                avatar,
                nickname,
                text: value,
                ..
            } = p.payload()
            {
                let nickname = if anonymous {
                    "Anonymous".into()
                } else {
                    text::display(nickname, text::Kind::Nickname)
                        .map_err(|_| ChannelError::Invalid)?
                        .rendered
                };
                out.push(ChannelMessage {
                    id: p.header().message_id.to_vec(),
                    sender: p.header().sender_id.to_vec(),
                    confusable: !anonymous
                        && row.direction == 0
                        && text::confusable_with_own(&nickname, &own_nickname).unwrap_or(false),
                    claim_warning: if !signed && !anonymous && row.direction == 0 {
                        pins.unwrap_or(&[])
                            .iter()
                            .find(|pin| {
                                use sha2::Digest;
                                sha2::Sha256::digest(&pin.tuple()[..32])[..8]
                                    == p.header().sender_id
                                    || text::confusable_with_own(&nickname, pin.petname())
                                        .unwrap_or(false)
                            })
                            .map(|p| format!("Claims to be {} - not verified", p.petname()))
                    } else {
                        None
                    },
                    nickname: verified_petname.clone().unwrap_or(nickname),
                    verified_petname,
                    signed,
                    text: text::display(value, text::Kind::Message)
                        .map_err(|_| ChannelError::Invalid)?
                        .rendered,
                    avatar: if anonymous { 0 } else { avatar },
                    own: row.direction == 1,
                    claimed_timestamp: timestamp,
                    arrival: row.timestamp,
                    reactions: vec![0; 9],
                    own_reaction: None,
                });
            }
        }
        for message in &mut out {
            if rows
                .iter()
                .filter(|r| r.logical_type == 1 && r.message_id == message.id)
                .count()
                != 1
            {
                continue;
            }
            let mut active: std::collections::BTreeMap<[u8; 8], u8> = Default::default();
            for row in &rows {
                let Ok(p) = codec::parse(&row.body, Context::Live) else {
                    continue;
                };
                if let Payload::Reaction {
                    target,
                    action,
                    code,
                } = p.payload()
                {
                    if target == message.id {
                        if action & 1 != 0 {
                            active.remove(&p.header().sender_id);
                        } else {
                            active.insert(p.header().sender_id, code);
                        }
                    }
                }
            }
            message.own_reaction = active.get(&self.sender).copied();
            for code in active.values() {
                let slot = usize::from((*code).min(8));
                message.reactions[slot] = (message.reactions[slot] + 1).min(30);
            }
        }
        Ok(out)
    }
}
