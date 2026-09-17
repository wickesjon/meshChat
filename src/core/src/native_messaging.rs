//! Protected application operations on the *same* ingress, pins and scheduler
//! as the native radio. Native adapters serialize operations with GATT effects,
//! open the protected store/session only for the call, then close both.
use super::*;
use crate::{
    dm,
    friends::{Pin, SendToken, SignedPacket},
    links,
    storage::AcceptResult,
    text,
};

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum MessagingError {
    #[error("invalid friend code or message")]
    Invalid,
    #[error("friend context changed or requires replacement")]
    Stale,
    #[error("protected identity or store unavailable")]
    Unavailable,
    #[error("no link or bounded work available")]
    Busy,
}
impl From<crate::friends::Error> for MessagingError {
    fn from(e: crate::friends::Error) -> Self {
        use crate::friends::Error;
        match e {
            Error::Invalid => Self::Invalid,
            Error::Stale => Self::Stale,
            Error::Full | Error::Ingress(_) => Self::Busy,
            _ => Self::Unavailable,
        }
    }
}
impl From<dm::Error> for MessagingError {
    fn from(e: dm::Error) -> Self {
        match e {
            dm::Error::Invalid => Self::Invalid,
            dm::Error::Stale => Self::Stale,
            dm::Error::Friend(e) => e.into(),
            dm::Error::Ingress(_) => Self::Busy,
            _ => Self::Unavailable,
        }
    }
}
impl From<crate::storage::StorageError> for MessagingError {
    fn from(_: crate::storage::StorageError) -> Self {
        Self::Unavailable
    }
}
impl From<TransportError> for MessagingError {
    fn from(e: TransportError) -> Self {
        match e {
            TransportError::Stale => Self::Stale,
            TransportError::Invalid => Self::Invalid,
            TransportError::Busy => Self::Busy,
            _ => Self::Unavailable,
        }
    }
}
#[derive(Debug, uniffi::Object)]
pub struct FriendHandle {
    token: SendToken,
}
#[derive(Clone, Debug, uniffi::Record)]
pub struct FriendCard {
    pub handle: Arc<FriendHandle>,
    pub keys: Vec<u8>,
    pub petname: String,
    pub fingerprint: String,
    pub replacing: bool,
    pub fresh: bool,
    pub response_age_ms: Option<u64>,
    pub avatar: u8,
}
#[derive(Clone, Debug, uniffi::Record)]
pub struct FriendProposal {
    pub uri: String,
    pub nickname: String,
    pub fingerprint: String,
    pub keys: Vec<u8>,
}
#[derive(Clone, Debug, uniffi::Record)]
pub struct DirectMessage {
    pub id: Vec<u8>,
    pub text: String,
    pub own: bool,
    pub claimed_timestamp: i64,
    pub reactions: Vec<u8>,
    pub own_reaction: Option<u8>,
}
#[derive(Clone, Debug, uniffi::Enum)]
pub enum DirectContent {
    Chat {
        text: String,
    },
    Reaction {
        target: Vec<u8>,
        remove: bool,
        code: u8,
    },
}
#[derive(Clone, Debug, uniffi::Record)]
pub struct MessageSubmission {
    pub id: Vec<u8>,
    pub queued: bool,
    pub queued_links: Vec<LinkHandle>,
    pub effects: TransportEffects,
}
#[derive(Clone, Debug, uniffi::Record)]
pub struct AuthenticationResult {
    pub changed: bool,
    pub authenticated: bool,
}

enum Guard {
    Direct(SendToken),
    Signed(SignedPacket),
}
struct Queued {
    link: LinkHandle,
    cookie: u64,
    guard: Guard,
}
pub(super) struct Messaging {
    pub(super) identity: PublicIdentity,
    dms: dm::Dms,
    queued: Vec<Queued>,
    retry_cursor: usize,
    avatars: Vec<([u8; 32], u8)>,
}
impl Messaging {
    pub(super) fn new(identity: PublicIdentity) -> Result<Self, TransportError> {
        let dms = dm::Dms::new(&identity).map_err(|_| TransportError::Identity)?;
        Ok(Self {
            identity,
            dms,
            queued: Vec::new(),
            retry_cursor: 0,
            avatars: Vec::new(),
        })
    }
    pub(super) fn disconnected(&mut self) {
        self.dms.clear();
    }
    pub(super) fn finished(&mut self, friends: &mut Friends, event: &relay::ResultEvent) {
        if let Some(i) = self
            .queued
            .iter()
            .position(|q| q.link == event.link && q.cookie == event.cookie)
        {
            let q = self.queued.remove(i);
            if event.status == relay::Status::NativeComplete {
                if let Guard::Signed(packet) = q.guard {
                    let _ = friends.content_transmitted(&packet);
                }
            }
        }
    }
    pub(super) fn egress_guard(&self, link: &LinkHandle, cookie: u64) -> (bool, Option<SendToken>) {
        match self
            .queued
            .iter()
            .find(|q| q.link == *link && q.cookie == cookie)
            .map(|q| &q.guard)
        {
            Some(Guard::Direct(pin)) => (true, Some(pin.clone())),
            Some(Guard::Signed(_)) => (true, None),
            None => (false, None),
        }
    }
    pub(super) fn store(&self, store: &EncryptedStore) -> Result<(), MessagingError> {
        if store.identity_generation()? != self.identity.generation {
            return Err(MessagingError::Stale);
        }
        Ok(())
    }
}
fn fingerprint(keys: &[u8]) -> String {
    // Full SHA-256 of both public keys, not the short routing identifier.
    Sha256::digest(keys)
        .chunks(2)
        .map(|b| format!("{:02X}{:02X}", b[0], b[1]))
        .collect::<Vec<_>>()
        .join(" ")
}
#[uniffi::export]
pub fn friend_proposal(uri: String) -> Result<FriendProposal, MessagingError> {
    match links::parse(&uri).map_err(|_| MessagingError::Invalid)? {
        links::UnconfirmedProposal::Friend { bundle, nickname } => Ok(FriendProposal {
            uri,
            nickname,
            fingerprint: fingerprint(&bundle[1..]),
            keys: bundle[1..].to_vec(),
        }),
        _ => Err(MessagingError::Invalid),
    }
}
#[uniffi::export]
pub fn friend_code(
    identity: PublicIdentity,
    nickname: String,
) -> Result<FriendProposal, MessagingError> {
    if identity.signing_key.len() != 32 || identity.agreement_key.len() != 32 {
        return Err(MessagingError::Invalid);
    }
    text::validate(&nickname, text::Kind::Nickname).map_err(|_| MessagingError::Invalid)?;
    let mut bytes = vec![1];
    bytes.extend(identity.signing_key);
    bytes.extend(identity.agreement_key);
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut binary = String::new();
    for block in bytes.chunks(3) {
        let bits = (u32::from(block[0]) << 16)
            | (u32::from(*block.get(1).unwrap_or(&0)) << 8)
            | u32::from(*block.get(2).unwrap_or(&0));
        for shift in [18, 12, 6, 0].into_iter().take(block.len() + 1) {
            binary.push(ALPHABET[((bits >> shift) & 63) as usize] as char);
        }
    }
    let nick = nickname
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect::<String>();
    friend_proposal(format!("meshfest://friend/{binary}/{nick}"))
}
impl Runtime {
    fn close_for_pin_change(&mut self) -> Result<TransportEffects, MessagingError> {
        let mut out = TransportEffects::default();
        for link in self
            .links
            .iter()
            .map(|l| l.handle.clone())
            .collect::<Vec<_>>()
        {
            self.close(&link, &mut out)?;
        }
        self.messaging.dms.clear();
        self.messaging.avatars.clear();
        self.messaging.queued.clear();
        Ok(out)
    }
    fn card(&mut self, pin: Pin, now: u64) -> FriendCard {
        let observation = self.friends.observation(&pin.token(), now).ok();
        FriendCard {
            handle: Arc::new(FriendHandle { token: pin.token() }),
            avatar: self
                .messaging
                .avatars
                .iter()
                .find(|(key, _)| key.as_slice() == &pin.tuple()[..32])
                .map_or(0, |(_, a)| *a),
            keys: pin.tuple().to_vec(),
            petname: pin.petname().into(),
            fingerprint: fingerprint(pin.tuple()),
            replacing: pin.replacing(),
            fresh: observation.as_ref().is_some_and(|o| o.fresh),
            response_age_ms: observation.and_then(|o| o.response_age_ms),
        }
    }
}
#[uniffi::export]
impl NativeTransport {
    pub fn friend_cards(
        &self,
        store: Arc<EncryptedStore>,
        now: u64,
    ) -> Result<Vec<FriendCard>, MessagingError> {
        let mut s = self.state.lock().map_err(|_| MessagingError::Unavailable)?;
        s.clock(now)?;
        s.messaging.store(&store)?;
        s.friends.reload(&store)?;
        let pins = s.friends.pins().to_vec();
        Ok(pins.into_iter().map(|p| s.card(p, now)).collect())
    }
    /// Explicit UI confirmation only. The supplied URI is parsed again; a
    /// displayed proposal object cannot smuggle a different decoded tuple.
    pub fn confirm_friend(
        &self,
        store: Arc<EncryptedStore>,
        provider: Arc<IdentityKeySession>,
        uri: String,
        petname: String,
        previous: Option<Arc<FriendHandle>>,
        now: u64,
    ) -> Result<TransportEffects, MessagingError> {
        let proposal = links::parse(&uri).map_err(|_| MessagingError::Invalid)?;
        let mut s = self.state.lock().map_err(|_| MessagingError::Unavailable)?;
        s.clock(now)?;
        s.messaging.store(&store)?;
        // Native adapter must stop/drain radio before a pin mutation. This is
        // enforced here so a failed pin transaction cannot lose close effects.
        if !s.links.is_empty() || !s.admissions.is_empty() {
            return Err(MessagingError::Busy);
        }
        if let Some(old) = &previous {
            let pin = s
                .friends
                .pins()
                .iter()
                .find(|p| p.token() == old.token)
                .ok_or(MessagingError::Stale)?;
            if !pin.replacing() {
                return Err(MessagingError::Stale);
            }
        }
        s.friends.reload(&store)?;
        let links::UnconfirmedProposal::Friend { bundle, .. } = &proposal else {
            return Err(MessagingError::Invalid);
        };
        if bundle[1..33] == s.messaging.identity.signing_key
            && bundle[33..] == s.messaging.identity.agreement_key
        {
            return Err(MessagingError::Invalid);
        }
        if s.friends.pins().iter().any(|p| {
            p.tuple().as_slice() == &bundle[1..]
                && previous.as_ref().is_none_or(|old| old.token != p.token())
        }) {
            return Err(MessagingError::Invalid);
        }
        if previous.is_none() && s.friends.pins().len() >= 128 {
            return Err(MessagingError::Busy);
        }
        s.friends.confirm(
            &store,
            &provider,
            proposal,
            &petname,
            previous.as_ref().map(|p| &p.token),
        )?;
        s.close_for_pin_change()
    }
    /// Call only after an explicit remove or replacement action. Network input
    /// never calls this. Replacement retains the old tuple's separate history.
    pub fn change_friend(
        &self,
        store: Arc<EncryptedStore>,
        friend: Arc<FriendHandle>,
        replace: bool,
        now: u64,
    ) -> Result<TransportEffects, MessagingError> {
        let mut s = self.state.lock().map_err(|_| MessagingError::Unavailable)?;
        s.clock(now)?;
        s.messaging.store(&store)?;
        if !s.links.is_empty() || !s.admissions.is_empty() {
            return Err(MessagingError::Busy);
        }
        if replace {
            s.friends.begin_replacement(&store, &friend.token)?
        } else {
            s.friends.remove(&store, &friend.token)?
        }
        s.close_for_pin_change()
    }
    /// Authenticate only the exact bytes already staged by this live transport.
    /// A caller's intake label, nickname or decoded plaintext grants no trust.
    pub fn authenticate_message(
        &self,
        store: Arc<EncryptedStore>,
        provider: Arc<IdentityKeySession>,
        link: LinkHandle,
        bytes: Vec<u8>,
        now: u64,
        wall: i64,
    ) -> Result<AuthenticationResult, MessagingError> {
        let mut s = self.state.lock().map_err(|_| MessagingError::Unavailable)?;
        s.clock(now)?;
        authenticate_pending(&mut s, &store, &provider, link, bytes, now, wall)
    }
    /// Four round-robin candidates per call. Original ingress generation and
    /// byte admission remain authoritative; work still spends the shared budget.
    pub fn retry_messages(
        &self,
        store: Arc<EncryptedStore>,
        provider: Arc<IdentityKeySession>,
        now: u64,
        wall: i64,
    ) -> Result<bool, MessagingError> {
        let mut s = self.state.lock().map_err(|_| MessagingError::Unavailable)?;
        s.clock(now)?;
        s.messaging.store(&store)?;
        let mut changed = false;
        for _ in 0..4 {
            let at = s.messaging.retry_cursor;
            s.messaging.retry_cursor = (at + 1) % 32;
            let Some(bytes) = s.ingress.pending_bytes(at).map(<[u8]>::to_vec) else {
                continue;
            };
            let link = s
                .links
                .iter()
                .find(|l| s.ingress.pending_deadline(&l.handle, &bytes).is_some())
                .map(|l| l.handle.clone());
            if let Some(link) = link {
                changed |= authenticate_pending(&mut s, &store, &provider, link, bytes, now, wall)?
                    .changed;
            }
        }
        Ok(changed)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn send_direct(
        &self,
        store: Arc<EncryptedStore>,
        provider: Arc<IdentityKeySession>,
        friend: Arc<FriendHandle>,
        content: DirectContent,
        cookie: u64,
        now: u64,
        wall: i64,
    ) -> Result<MessageSubmission, MessagingError> {
        if cookie < 3 {
            return Err(MessagingError::Invalid);
        }
        let content = match content {
            DirectContent::Chat { text: value } => dm::Content::Chat(
                text::outgoing(&value, text::Kind::Message).map_err(|_| MessagingError::Invalid)?,
            ),
            DirectContent::Reaction {
                target,
                remove,
                code,
            } => {
                if code >= 8 {
                    return Err(MessagingError::Invalid);
                }
                dm::Content::Reaction {
                    target: target.try_into().map_err(|_| MessagingError::Invalid)?,
                    remove,
                    code,
                }
            }
        };
        let mut s = self.state.lock().map_err(|_| MessagingError::Unavailable)?;
        s.clock(now)?;
        s.messaging.store(&store)?;
        if s.messaging.queued.len() + s.links.len() > 192
            || s.messaging.queued.iter().any(|q| q.cookie == cookie)
        {
            return Err(MessagingError::Busy);
        }
        let links = s
            .links
            .iter()
            .filter(|l| l.admitted)
            .map(|l| l.handle.clone())
            .collect::<Vec<_>>();
        let first = links.first().ok_or(MessagingError::Busy)?;
        let mut id = [0; 8];
        getrandom::fill(&mut id).map_err(|_| MessagingError::Unavailable)?;
        let Runtime {
            ingress,
            friends,
            messaging,
            ..
        } = &mut *s;
        let sent = messaging
            .dms
            .send(
                ingress,
                friends,
                &store,
                &provider,
                first,
                &friend.token,
                id,
                content,
                now,
                Some(wall),
            )?
            .ok_or(MessagingError::Busy)?;
        let mut out = TransportEffects::default();
        s.relay
            .observe(None, sent.bytes(), now)
            .map_err(|_| MessagingError::Unavailable)?;
        for link in links {
            s.friends.validate_send(&store, sent.pin())?;
            // Register before enqueue; its terminal suppression/eviction effects
            // retire the guard just like native completion does.
            s.messaging.queued.push(Queued {
                link: link.clone(),
                cookie,
                guard: Guard::Direct(sent.pin().clone()),
            });
            match s.enqueue(&link, sent.bytes(), relay::Traffic::Own, cookie, &mut out) {
                Ok(()) => {}
                Err(TransportError::Refused { .. }) => s
                    .messaging
                    .queued
                    .retain(|q| q.cookie != cookie || q.link != link),
                Err(e) => return Err(e.into()),
            }
        }
        let queued_links = s
            .messaging
            .queued
            .iter()
            .filter(|q| q.cookie == cookie)
            .map(|q| q.link.clone())
            .collect::<Vec<_>>();
        Ok(MessageSubmission {
            id: id.to_vec(),
            queued: !queued_links.is_empty(),
            queued_links,
            effects: out,
        })
    }
    /// Every fragment from a protected outgoing operation is checked under the
    /// native radio monitor immediately before submission, including queued DMs.
    pub fn authorize_message_egress(
        &self,
        store: Arc<EncryptedStore>,
        link: LinkHandle,
        token: u64,
    ) -> Result<(), MessagingError> {
        let mut s = self.state.lock().map_err(|_| MessagingError::Unavailable)?;
        s.messaging.store(&store)?;
        let pending = s.links[s.index(&link)?]
            .pending
            .as_ref()
            .ok_or(MessagingError::Stale)?;
        if pending.token != token {
            return Err(MessagingError::Stale);
        }
        if s.organizer.guarded(&link, pending.cookie) {
            return Err(MessagingError::Unavailable); // Requires wall-clock organizer revalidation.
        }
        if let Some(pin) = pending.pin.clone() {
            s.friends.validate_send(&store, &pin)?;
        }
        Ok(())
    }
    pub fn message_needs_authorization(
        &self,
        link: LinkHandle,
        token: u64,
    ) -> Result<bool, MessagingError> {
        let s = self.state.lock().map_err(|_| MessagingError::Unavailable)?;
        let pending = s.links[s.index(&link)?]
            .pending
            .as_ref()
            .ok_or(MessagingError::Stale)?;
        if pending.token != token {
            return Err(MessagingError::Stale);
        }
        Ok(pending.protected)
    }
    /// History belongs to the full tuple even after removal/replacement. An old
    /// handle may read old history, but cannot send to a removed/revised pin.
    pub fn direct_history(
        &self,
        store: Arc<EncryptedStore>,
        keys: Vec<u8>,
        now: u64,
        wall: i64,
    ) -> Result<Vec<DirectMessage>, MessagingError> {
        let mut s = self.state.lock().map_err(|_| MessagingError::Unavailable)?;
        s.clock(now)?;
        s.messaging.store(&store)?;
        let tuple: [u8; 64] = keys.try_into().map_err(|_| MessagingError::Invalid)?;
        let mut rows = store.history(tuple.to_vec(), true, 100)?;
        rows.reverse();
        let mut out = vec![];
        for row in rows.into_iter().filter(|r| r.logical_type == 1) {
            if row.provenance.len() != 73
                || row.provenance[0] != 2
                || row.provenance[1..65] != tuple
                || ![64, 144, 304].contains(&row.body.len())
            {
                continue;
            }
            let n = usize::from(u16::from_be_bytes([row.body[4], row.body[5]]));
            let value =
                std::str::from_utf8(row.body.get(6..6 + n).ok_or(MessagingError::Unavailable)?)
                    .map_err(|_| MessagingError::Unavailable)?;
            let mut reactions = vec![0; 9];
            let mut own_reaction = None;
            // Persistent reaction state is already bound to this tuple and
            // target by accept_dm; old conversations remain readable as history.
            let pin = s
                .friends
                .pins()
                .iter()
                .find(|p| *p.tuple() == tuple && !p.replacing())
                .map(|p| p.token());
            let reactions_state = if let Some(pin) = pin {
                let Runtime {
                    messaging, friends, ..
                } = &mut *s;
                messaging.dms.reactions(
                    &store,
                    friends,
                    &pin,
                    row.message_id
                        .as_slice()
                        .try_into()
                        .map_err(|_| MessagingError::Unavailable)?,
                    now,
                    Some(wall),
                )?
            } else {
                store.dm_reactions(&tuple, &row.message_id, Some(wall))?
            };
            for (direction, code) in reactions_state {
                reactions[usize::from(code.min(8))] += 1;
                if direction == 1 {
                    own_reaction = Some(code)
                }
            }
            out.push(DirectMessage {
                id: row.message_id,
                text: text::display(value, text::Kind::Message)
                    .map_err(|_| MessagingError::Unavailable)?
                    .rendered,
                own: row.direction == 1,
                claimed_timestamp: row.timestamp,
                reactions,
                own_reaction,
            });
        }
        Ok(out)
    }
    /// ANNOUNCE is always signed; private CHAT signing policy is selected by
    /// the feature owner. First-success key inclusion is advanced on completion.
    pub fn send_signed(
        &self,
        store: Arc<EncryptedStore>,
        provider: Arc<IdentityKeySession>,
        bytes: Vec<u8>,
        cookie: u64,
        now: u64,
        wall: i64,
    ) -> Result<MessageSubmission, MessagingError> {
        if cookie < 3 {
            return Err(MessagingError::Invalid);
        }
        let mut s = self.state.lock().map_err(|_| MessagingError::Unavailable)?;
        s.clock(now)?;
        s.messaging.store(&store)?;
        let links = s
            .links
            .iter()
            .filter(|l| l.admitted)
            .map(|l| l.handle.clone())
            .collect::<Vec<_>>();
        if links.is_empty()
            || s.messaging.queued.len() + links.len() > 192
            || s.messaging.queued.iter().any(|q| q.cookie == cookie)
        {
            return Err(MessagingError::Busy);
        }
        let mut packets = vec![];
        for link in links {
            let Runtime {
                ingress, friends, ..
            } = &mut *s;
            let signed = friends
                .sign_content(ingress, &provider, &link, &bytes, now)?
                .ok_or(MessagingError::Busy)?;
            packets.push((link, signed));
        }
        // Key inclusion differs with link history. Flood ONE immutable form of
        // this message on every link, using the included-key form if any needs
        // it. Otherwise multi-path reception would create replay conflicts.
        let raw = packets
            .iter()
            .max_by_key(|(_, p)| p.bytes().len())
            .ok_or(MessagingError::Busy)?
            .1
            .bytes()
            .to_vec();
        let p = codec::parse(&raw, codec::Context::Live).map_err(|_| MessagingError::Invalid)?;
        if let codec::Payload::Chat { timestamp, .. } = p.payload() {
            let mut subject = vec![1];
            subject.extend_from_slice(&s.messaging.identity.signing_key);
            let mut immutable = raw.clone();
            immutable[3] = 0;
            let accepted = store.accept_authenticated(
                crate::storage::HistoryItem {
                    conversation: p.header().channel_id.to_vec(),
                    direct: false,
                    direction: 1,
                    logical_type: 1,
                    message_id: p.header().message_id.to_vec(),
                    timestamp: i64::from(timestamp),
                    body: raw.clone(),
                    provenance: subject.clone(),
                },
                subject,
                immutable,
                Some(wall),
            )?;
            if accepted != AcceptResult::Accepted {
                return Err(MessagingError::Stale);
            }
            s.relay
                .observe(None, &raw, now)
                .map_err(|_| MessagingError::Unavailable)?;
        }
        let mut out = TransportEffects::default();
        for (link, signed) in packets {
            s.messaging.queued.push(Queued {
                link: link.clone(),
                cookie,
                guard: Guard::Signed(signed),
            });
            match s.enqueue(
                &link,
                &raw,
                if raw[1] == 2 {
                    relay::Traffic::Local
                } else {
                    relay::Traffic::Own
                },
                cookie,
                &mut out,
            ) {
                Ok(()) => {}
                Err(TransportError::Refused { .. }) => s
                    .messaging
                    .queued
                    .retain(|q| q.link != link || q.cookie != cookie),
                Err(e) => return Err(e.into()),
            }
        }
        let queued_links = s
            .messaging
            .queued
            .iter()
            .filter(|q| q.cookie == cookie)
            .map(|q| q.link.clone())
            .collect::<Vec<_>>();
        Ok(MessageSubmission {
            id: raw[4..12].to_vec(),
            queued: !queued_links.is_empty(),
            queued_links,
            effects: out,
        })
    }
    pub fn messaging_channel_history(
        &self,
        store: Arc<EncryptedStore>,
        owner: Arc<crate::native_channels::NativeChannels>,
        name: String,
        nickname: String,
    ) -> Result<Vec<crate::native_channels::ChannelMessage>, MessagingError> {
        let mut s = self.state.lock().map_err(|_| MessagingError::Unavailable)?;
        s.messaging.store(&store)?;
        s.friends.reload(&store)?;
        owner
            .authenticated_history(store, name, nickname, s.friends.pins())
            .map_err(|_| MessagingError::Unavailable)
    }
}

pub(super) fn authenticate_pending(
    s: &mut Runtime,
    store: &EncryptedStore,
    provider: &IdentityKeySession,
    link: LinkHandle,
    bytes: Vec<u8>,
    now: u64,
    wall: i64,
) -> Result<AuthenticationResult, MessagingError> {
    s.messaging.store(store)?;
    if provider
        .public_identity()
        .map_err(|_| MessagingError::Unavailable)?
        != s.messaging.identity
    {
        return Err(MessagingError::Stale);
    }
    s.index(&link)?;
    if matches!(codec::parse(&bytes,codec::Context::Live).map(|p|p.payload()),Ok(codec::Payload::Announce{timestamp,..}) if i64::from(timestamp)<wall.saturating_sub(172800) || i64::from(timestamp)>wall.saturating_add(300))
    {
        return Ok(AuthenticationResult {
            changed: false,
            authenticated: false,
        });
    }
    let position = (0..32).find(|i| s.ingress.pending_bytes(*i) == Some(bytes.as_slice()));
    let Some(position) = position else {
        return Ok(AuthenticationResult {
            changed: false,
            authenticated: false,
        });
    };
    let Runtime {
        ingress,
        friends,
        messaging,
        ..
    } = s;
    let mut result = AuthenticationResult {
        changed: false,
        authenticated: false,
    };
    if bytes.get(2).is_some_and(|f| f & 1 != 0) {
        if let Some(job) = messaging
            .dms
            .prepare(ingress, friends, store, &link, position, now)?
        {
            match messaging
                .dms
                .complete(ingress, friends, store, provider, job, now, Some(wall))
            {
                Ok(received) => {
                    result.authenticated = true;
                    result.changed = received.result == AcceptResult::Accepted
                }
                Err(
                    dm::Error::Authentication
                    | dm::Error::Invalid
                    | dm::Error::Tag
                    | dm::Error::Storage(crate::storage::StorageError::InvalidInput),
                ) => {}
                Err(e) => return Err(e.into()),
            }
        }
    } else if let Some(job) = friends.retry_pending(ingress, &link, position, now)? {
        match friends.complete(ingress, store, job, now, Some(wall)) {
            Ok(Some(verified)) => {
                result.authenticated = true;
                result.changed = verified.new_history;
                if verified.friend.is_some() {
                    if let Ok(p) = codec::parse(&bytes, codec::Context::Live) {
                        if let codec::Payload::Announce { avatar, .. } = p.payload() {
                            if let Some((_, old)) = messaging
                                .avatars
                                .iter_mut()
                                .find(|(key, _)| *key == verified.signer)
                            {
                                *old = avatar
                            } else if messaging.avatars.len() < 128 {
                                messaging.avatars.push((verified.signer, avatar))
                            }
                        }
                    }
                }
            }
            Ok(None)
            | Err(crate::friends::Error::Storage(crate::storage::StorageError::InvalidInput)) => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(result)
}
