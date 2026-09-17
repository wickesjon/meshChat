//! Canonical organizer operations on the shared native ingress and scheduler.
use super::*;
use crate::{
    links,
    organizer::{Authority, OfferSchedule, Organizer, StaffSession},
    text,
};
use zeroize::Zeroizing;

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum OrganizerError {
    #[error("invalid event or staff credential")]
    Invalid,
    #[error("adopted authority unavailable or expired")]
    Authority,
    #[error("bounded organizer work unavailable")]
    Busy,
    #[error("protected organizer state unavailable")]
    Unavailable,
}
impl From<crate::organizer::Error> for OrganizerError {
    fn from(e: crate::organizer::Error) -> Self {
        match e {
            crate::organizer::Error::Invalid => Self::Invalid,
            crate::organizer::Error::Authority => Self::Authority,
            crate::organizer::Error::Capacity => Self::Busy,
            _ => Self::Unavailable,
        }
    }
}
impl From<crate::storage::StorageError> for OrganizerError {
    fn from(_: crate::storage::StorageError) -> Self {
        Self::Unavailable
    }
}
impl From<TransportError> for OrganizerError {
    fn from(_: TransportError) -> Self {
        Self::Unavailable
    }
}
#[derive(Clone, Debug, uniffi::Record)]
pub struct EventProposal {
    pub name: String,
    pub key: Vec<u8>,
    pub fingerprint: String,
    pub expiry: u32,
}
#[derive(Clone, Debug, uniffi::Record)]
pub struct EventCard {
    pub event: EventProposal,
    pub active: bool,
}
#[derive(Clone, Debug, uniffi::Record)]
pub struct StaffCard {
    pub label: String,
    pub credential: Vec<u8>,
    pub root_id: Vec<u8>,
    pub staff_key: Vec<u8>,
    pub not_before: u32,
    pub not_after: u32,
}
#[derive(Clone, Debug, uniffi::Record)]
pub struct EventMessage {
    pub content_key: Vec<u8>,
    pub id: Vec<u8>,
    pub nickname: String,
    pub text: String,
    pub timestamp: u32,
    pub own: bool,
    pub staff_label: Option<String>,
    pub pinned: bool,
}
#[derive(uniffi::Object)]
pub struct NativeStaffSession {
    session: StaffSession,
}
#[uniffi::export]
impl NativeStaffSession {
    pub fn invalidate(&self) {
        self.session.invalidate();
    }
}
impl Drop for NativeStaffSession {
    fn drop(&mut self) {
        self.session.invalidate();
    }
}
fn fingerprint(key: &[u8]) -> String {
    Sha256::digest(key)
        .chunks(2)
        .map(|b| format!("{:02X}{:02X}", b[0], b[1]))
        .collect::<Vec<_>>()
        .join(" ")
}
#[uniffi::export]
pub fn event_proposal(uri: String) -> Result<EventProposal, OrganizerError> {
    match links::parse(&uri).map_err(|_| OrganizerError::Invalid)? {
        links::UnconfirmedProposal::Event { bundle, name, .. } => Ok(EventProposal {
            name,
            key: bundle[1..33].to_vec(),
            fingerprint: fingerprint(&bundle[1..33]),
            expiry: u32::from_be_bytes(bundle[33..37].try_into().unwrap()),
        }),
        _ => Err(OrganizerError::Invalid),
    }
}
fn staff_card(raw: &[u8]) -> Result<StaffCard, OrganizerError> {
    let c = codec::credential(raw).map_err(|_| OrganizerError::Invalid)?;
    text::validate(c.label, text::Kind::CredentialLabel).map_err(|_| OrganizerError::Invalid)?;
    Ok(StaffCard {
        label: c.label.into(),
        credential: c.raw.to_vec(),
        root_id: c.root_id.to_vec(),
        staff_key: c.staff_public_key.to_vec(),
        not_before: c.not_before,
        not_after: c.not_after,
    })
}
#[uniffi::export]
pub fn staff_proposal(uri: String) -> Result<StaffCard, OrganizerError> {
    let uri = Zeroizing::new(uri);
    match links::parse(&uri).map_err(|_| OrganizerError::Invalid)? {
        links::UnconfirmedProposal::Staff(p) => staff_card(p.credential()),
        _ => Err(OrganizerError::Invalid),
    }
}
struct Queued {
    link: LinkHandle,
    cookie: u64,
    authority: Authority,
    epoch: u64,
}
pub(super) struct NativeOrganizer {
    core: Organizer,
    sender: [u8; 8],
    generation: Vec<u8>,
    queued: Vec<Queued>,
    epoch: u64,
    discovery: Vec<(String, u64)>,
    drawer: Vec<(Vec<u8>, u64)>,
    schedule: OfferSchedule,
    retry: usize,
    post_credit: u64,
    post_at: u64,
}
impl NativeOrganizer {
    pub(super) fn new(identity: &PublicIdentity) -> Result<Self, TransportError> {
        Ok(Self {
            core: Organizer::new(identity).map_err(|_| TransportError::Identity)?,
            sender: identity
                .sender_id
                .as_slice()
                .try_into()
                .map_err(|_| TransportError::Identity)?,
            generation: identity.generation.clone(),
            queued: vec![],
            epoch: 0,
            discovery: vec![],
            drawer: vec![],
            schedule: OfferSchedule::default(),
            retry: 0,
            post_credit: 60_000,
            post_at: 0,
        })
    }
    fn store(&self, store: &EncryptedStore) -> Result<(), OrganizerError> {
        if store.identity_generation()? != self.generation {
            return Err(OrganizerError::Authority);
        }
        Ok(())
    }
    pub(super) fn guarded(&self, link: &LinkHandle, cookie: u64) -> bool {
        self.queued
            .iter()
            .any(|q| q.link == *link && q.cookie == cookie)
    }
    pub(super) fn finished(&mut self, event: &relay::ResultEvent) {
        self.queued
            .retain(|q| q.link != event.link || q.cookie != event.cookie);
    }
}
fn packet(kind: u8, flags: u8, sender: [u8; 8], body: &[u8]) -> Result<Vec<u8>, OrganizerError> {
    let mut id = [0; 8];
    getrandom::fill(&mut id).map_err(|_| OrganizerError::Unavailable)?;
    let mut out = vec![0; 1024];
    let len = codec::serialize(
        codec::Header {
            kind,
            flags,
            ttl: if kind == 7 { 1 } else { 7 },
            message_id: id,
            sender_id: sender,
            channel_id: if kind == 7 {
                [0; 4]
            } else {
                codec::EVENT_CHANNEL
            },
        },
        body,
        codec::Context::Live,
        &mut out,
    )
    .map_err(|_| OrganizerError::Invalid)?;
    out.truncate(len);
    Ok(out)
}
impl Runtime {
    pub(super) fn organizer_received(
        &mut self,
        link: &LinkHandle,
        raw: &[u8],
        now: u64,
        out: &mut TransportEffects,
    ) -> Result<(), TransportError> {
        let Ok(p) = codec::parse(raw, codec::Context::Live) else {
            return Ok(());
        };
        if text::validate_payload(p.payload()).is_err() {
            return Ok(());
        }
        self.organizer
            .discovery
            .retain(|(_, at)| now.saturating_sub(*at) < 900_000);
        self.organizer
            .drawer
            .retain(|(_, at)| now.saturating_sub(*at) < 900_000);
        match p.payload() {
            codec::Payload::EventInfo { name, .. } => {
                if !self.organizer.discovery.iter().any(|(n, _)| n == name) {
                    if self.organizer.discovery.len() == 16 {
                        self.organizer.discovery.remove(0);
                    }
                    self.organizer.discovery.push((name.into(), now));
                }
            }
            codec::Payload::Chat { .. } if p.header().channel_id == codec::EVENT_CHANNEL => {
                if !self.organizer.drawer.iter().any(|(b, _)| b == raw) {
                    if self.organizer.drawer.len() == 100 {
                        self.organizer.drawer.remove(0);
                    }
                    self.organizer.drawer.push((raw.to_vec(), now));
                }
            }
            _ => {}
        }
        if let Ok(Some(credential)) = self.organizer.core.admitted_control(raw, now) {
            if let Ok(response) = packet(8, 0, self.organizer.sender, &credential) {
                let cookie = (1u64 << 63) | self.next()?;
                let _ = self.enqueue(link, &response, relay::Traffic::Local, cookie, out);
            }
        }
        Ok(())
    }
    fn organizer_authenticate(
        &mut self,
        store: &EncryptedStore,
        now: u64,
        wall: i64,
    ) -> Result<(), OrganizerError> {
        let links = self
            .links
            .iter()
            .filter(|l| l.admitted)
            .map(|l| l.handle.clone())
            .collect::<Vec<_>>();
        // At most eight candidates per call; core cryptographic budgets still apply.
        for _ in 0..8 {
            let position = self.organizer.retry % 32;
            self.organizer.retry = (position + 1) % 32;
            for link in &links {
                let Some(raw) = self.ingress.pending_bytes(position) else {
                    break;
                };
                if !matches!(
                    codec::parse(raw, codec::Context::Live).map(|p| p.payload()),
                    Ok(codec::Payload::CredentialOffer(_)
                        | codec::Payload::Chat {
                            signature: codec::Signature::Organizer { .. },
                            ..
                        })
                ) {
                    break;
                }
                match self.organizer.core.retry(
                    &mut self.ingress,
                    &self.friends,
                    store,
                    link,
                    position,
                    now,
                    Some(wall),
                ) {
                    Ok(Some(job)) => {
                        match self.organizer.core.complete(
                            &mut self.ingress,
                            store,
                            job,
                            now,
                            Some(wall),
                        ) {
                            Ok(_)
                            | Err(
                                crate::organizer::Error::Invalid
                                | crate::organizer::Error::Authority
                                | crate::organizer::Error::Storage(
                                    crate::storage::StorageError::InvalidInput,
                                ),
                            ) => {}
                            Err(e) => return Err(e.into()),
                        }
                        break;
                    }
                    Ok(None)
                    | Err(crate::organizer::Error::Invalid | crate::organizer::Error::Authority) => {
                    }
                    Err(e) => return Err(e.into()),
                }
            }
        }
        Ok(())
    }
}
#[uniffi::export]
impl NativeTransport {
    pub fn event_cards(
        &self,
        store: Arc<EncryptedStore>,
        wall: i64,
    ) -> Result<Vec<EventCard>, OrganizerError> {
        let s = self.state.lock().map_err(|_| OrganizerError::Unavailable)?;
        s.organizer.store(&store)?;
        let time = store.authority_time(Some(wall))?;
        store
            .event_roots()?
            .into_iter()
            .map(|(key, value)| {
                if value.len() < 110 {
                    return Err(OrganizerError::Invalid);
                }
                let expiry = u32::from_be_bytes(value[42..46].try_into().unwrap());
                Ok(EventCard {
                    event: EventProposal {
                        name: String::from_utf8(value[110..].to_vec())
                            .map_err(|_| OrganizerError::Invalid)?,
                        fingerprint: fingerprint(&key),
                        key,
                        expiry,
                    },
                    active: i64::from(expiry) >= time,
                })
            })
            .collect()
    }
    pub fn confirm_event(
        &self,
        store: Arc<EncryptedStore>,
        uri: String,
        now: u64,
        wall: i64,
    ) -> Result<(), OrganizerError> {
        let p = links::parse(&uri).map_err(|_| OrganizerError::Invalid)?;
        let mut s = self.state.lock().map_err(|_| OrganizerError::Unavailable)?;
        s.clock(now)?;
        s.organizer.store(&store)?;
        let Runtime {
            organizer, ingress, ..
        } = &mut *s;
        if !organizer.core.adopt(ingress, &store, p, now, Some(wall))? {
            return Err(OrganizerError::Busy);
        }
        Ok(())
    }
    pub fn remove_event(
        &self,
        store: Arc<EncryptedStore>,
        key: Vec<u8>,
        wall: i64,
    ) -> Result<(), OrganizerError> {
        let mut s = self.state.lock().map_err(|_| OrganizerError::Unavailable)?;
        s.organizer.store(&store)?;
        s.organizer.core.remove(
            &store,
            key.as_slice()
                .try_into()
                .map_err(|_| OrganizerError::Invalid)?,
            Some(wall),
        )?;
        Ok(())
    }
    /// Called by the protected provider after an explicit confirmation, or for
    /// a short-lived reload of its already confirmed encrypted credential.
    pub fn import_staff_key(
        &self,
        store: Arc<EncryptedStore>,
        uri: String,
        now: u64,
        wall: i64,
    ) -> Result<Arc<NativeStaffSession>, OrganizerError> {
        let uri = Zeroizing::new(uri);
        let links::UnconfirmedProposal::Staff(p) =
            links::parse(&uri).map_err(|_| OrganizerError::Invalid)?
        else {
            return Err(OrganizerError::Invalid);
        };
        let mut s = self.state.lock().map_err(|_| OrganizerError::Unavailable)?;
        s.clock(now)?;
        s.organizer.store(&store)?;
        let Runtime {
            organizer, ingress, ..
        } = &mut *s;
        let session = organizer
            .core
            .import_staff(ingress, &store, p, now, Some(wall))?
            .ok_or(OrganizerError::Busy)?;
        Ok(Arc::new(NativeStaffSession { session }))
    }
    pub fn forget_staff_operations(&self) -> Result<(), OrganizerError> {
        let mut s = self.state.lock().map_err(|_| OrganizerError::Unavailable)?;
        s.organizer.epoch = s
            .organizer
            .epoch
            .checked_add(1)
            .ok_or(OrganizerError::Unavailable)?;
        s.organizer.schedule = OfferSchedule::default();
        Ok(())
    }
    pub fn organizer_tick(
        &self,
        store: Arc<EncryptedStore>,
        credential: Option<Vec<u8>>,
        now: u64,
        wall: i64,
    ) -> Result<TransportEffects, OrganizerError> {
        let mut s = self.state.lock().map_err(|_| OrganizerError::Unavailable)?;
        s.clock(now)?;
        s.organizer.store(&store)?;
        s.organizer_authenticate(&store, now, wall)?;
        let mut out = TransportEffects::default();
        let Runtime {
            organizer, friends, ..
        } = &mut *s;
        if let Some(req) = organizer.core.next_request(friends, now)? {
            let raw = packet(7, 0, organizer.sender, &req.pair)?;
            let cookie = (1u64 << 63) | s.next()?;
            let _ = s.enqueue(&req.link, &raw, relay::Traffic::Local, cookie, &mut out);
        }
        if let Some(c) = credential {
            if s.organizer.schedule.due(now, false)? {
                let raw = packet(8, 0, s.organizer.sender, &c)?;
                let links = s
                    .links
                    .iter()
                    .filter(|l| l.admitted)
                    .map(|l| l.handle.clone())
                    .collect::<Vec<_>>();
                for link in links {
                    let cookie = (1u64 << 63) | s.next()?;
                    let _ = s.enqueue(&link, &raw, relay::Traffic::Own, cookie, &mut out);
                }
            }
        }
        Ok(out)
    }
    pub fn event_discoveries(&self, now: u64) -> Result<Vec<String>, OrganizerError> {
        let mut s = self.state.lock().map_err(|_| OrganizerError::Unavailable)?;
        s.clock(now)?;
        s.organizer
            .discovery
            .retain(|(_, at)| now.saturating_sub(*at) < 900_000);
        Ok(s.organizer
            .discovery
            .iter()
            .map(|(name, _)| name.clone())
            .collect())
    }
    pub fn event_messages(
        &self,
        store: Arc<EncryptedStore>,
        wall: i64,
        now: u64,
    ) -> Result<Vec<EventMessage>, OrganizerError> {
        let mut s = self.state.lock().map_err(|_| OrganizerError::Unavailable)?;
        s.clock(now)?;
        s.organizer.store(&store)?;
        s.organizer_authenticate(&store, now, wall)?;
        let mut rows = vec![];
        for item in store
            .history(codec::EVENT_CHANNEL.to_vec(), false, 100)?
            .into_iter()
            .rev()
        {
            let authority = s.organizer.core.historical_authority(&store, &item, wall)?;
            let (mut label, mut pinned) = (None, false);
            if let Some(a) = authority {
                let (valid, pin) = s.organizer.core.current(&store, &a, Some(wall))?;
                if valid {
                    label = Some(a.label().into());
                    pinned = pin;
                }
            }
            if let Some(row) = message(&item.body, item.direction == 1, label, pinned) {
                rows.push(row);
            }
        }
        s.organizer
            .drawer
            .retain(|(_, at)| now.saturating_sub(*at) < 900_000);
        for (raw, _) in &s.organizer.drawer {
            if rows.iter().any(|r| {
                r.content_key == {
                    let mut b = raw.clone();
                    b[3] = 0;
                    Sha256::digest(&b).to_vec()
                }
            }) {
                continue;
            }
            if let Some(row) = message(raw, false, None, false) {
                rows.push(row)
            }
        }
        if rows.len() > 100 {
            rows.drain(..rows.len() - 100);
        }
        Ok(rows)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn post_event(
        &self,
        store: Arc<EncryptedStore>,
        session: Arc<NativeStaffSession>,
        nickname: String,
        text: String,
        avatar: u8,
        pin_expiry: Option<u32>,
        cookie: u64,
        now: u64,
        wall: i64,
    ) -> Result<messaging::MessageSubmission, OrganizerError> {
        let mut s = self.state.lock().map_err(|_| OrganizerError::Unavailable)?;
        s.clock(now)?;
        s.organizer.store(&store)?;
        let links = s
            .links
            .iter()
            .filter(|l| l.admitted)
            .map(|l| l.handle.clone())
            .collect::<Vec<_>>();
        if cookie < 3
            || links.is_empty()
            || s.organizer.queued.len() + links.len() > 192
            || s.organizer.queued.iter().any(|q| q.cookie == cookie)
        {
            return Err(OrganizerError::Busy);
        }
        if pin_expiry.is_some_and(|e| i64::from(e) <= wall) {
            return Err(OrganizerError::Invalid);
        }
        let nick =
            text::outgoing(&nickname, text::Kind::Nickname).map_err(|_| OrganizerError::Invalid)?;
        let text =
            text::outgoing(&text, text::Kind::Message).map_err(|_| OrganizerError::Invalid)?;
        s.organizer.post_credit = s
            .organizer
            .post_credit
            .saturating_add(now - s.organizer.post_at)
            .min(60_000);
        s.organizer.post_at = now;
        if s.organizer.post_credit < 6_000 {
            return Err(OrganizerError::Busy);
        }
        let c =
            codec::credential(session.session.credential()).map_err(|_| OrganizerError::Invalid)?;
        let mut body = u32::try_from(wall)
            .map_err(|_| OrganizerError::Invalid)?
            .to_be_bytes()
            .to_vec();
        body.push(avatar);
        body.push(nick.len() as u8);
        body.extend_from_slice(nick.as_bytes());
        body.extend_from_slice(&[0; 4]);
        body.extend_from_slice(&(text.len() as u16).to_be_bytes());
        body.extend_from_slice(text.as_bytes());
        body.push(u8::from(pin_expiry.is_some()));
        body.extend_from_slice(&pin_expiry.unwrap_or(0).to_be_bytes());
        body.extend_from_slice(c.root_id);
        body.extend_from_slice(&Sha256::digest(c.staff_public_key)[..8]);
        body.push(1);
        body.extend_from_slice(&(c.raw.len() as u16).to_be_bytes());
        body.extend_from_slice(c.raw);
        body.extend_from_slice(&[0; 64]);
        let raw = packet(1, 6, s.organizer.sender, &body)?;
        let Runtime {
            organizer,
            ingress,
            friends,
            ..
        } = &mut *s;
        let raw = organizer
            .core
            .sign(
                ingress,
                friends,
                &store,
                &session.session,
                &links[0],
                raw,
                now,
                Some(wall),
            )?
            .ok_or(OrganizerError::Busy)?;
        s.organizer.post_credit -= 6_000;
        let mut out = TransportEffects::default();
        let mut queued_links = vec![];
        let epoch = s.organizer.epoch;
        for link in &links {
            s.organizer.queued.push(Queued {
                link: link.clone(),
                cookie,
                authority: session.session.authority().clone(),
                epoch,
            });
            match s.enqueue(link, &raw, relay::Traffic::Own, cookie, &mut out) {
                Ok(()) => {
                    if out.events.iter().any(|e|matches!(e,TransportEvent::Finished {link:l,cookie:c,..} if l==link&&*c==cookie)) {
                        s.organizer.queued.retain(|q|q.link!=*link||q.cookie!=cookie);
                    } else {queued_links.push(link.clone());}
                },
                Err(TransportError::Refused { .. }) => s
                    .organizer
                    .queued
                    .retain(|q| q.link != *link || q.cookie != cookie),
                Err(e) => return Err(e.into()),
            }
        }
        if s.organizer.schedule.due(now, true)? {
            let offer = packet(8, 0, s.organizer.sender, session.session.credential())?;
            for link in links {
                let cookie = (1u64 << 63) | s.next()?;
                let _ = s.enqueue(&link, &offer, relay::Traffic::Own, cookie, &mut out);
            }
        }
        Ok(messaging::MessageSubmission {
            id: raw[4..12].to_vec(),
            queued: !queued_links.is_empty(),
            queued_links,
            effects: out,
        })
    }
    /// Android's protected owner calls this for every native fragment. Other
    /// message guards retain their existing authorization API.
    pub fn authorize_organizer_egress(
        &self,
        store: Arc<EncryptedStore>,
        link: LinkHandle,
        token: u64,
        wall: i64,
    ) -> Result<bool, OrganizerError> {
        let mut s = self.state.lock().map_err(|_| OrganizerError::Unavailable)?;
        s.organizer.store(&store)?;
        let p = s.links[s.index(&link)?]
            .pending
            .as_ref()
            .ok_or(OrganizerError::Authority)?;
        if p.token != token {
            return Err(OrganizerError::Authority);
        }
        let Some(q) = s
            .organizer
            .queued
            .iter()
            .find(|q| q.link == link && q.cookie == p.cookie)
        else {
            return Ok(false);
        };
        if q.epoch != s.organizer.epoch {
            return Err(OrganizerError::Authority);
        }
        let authority = q.authority.clone();
        if !s.organizer.core.current(&store, &authority, Some(wall))?.0 {
            return Err(OrganizerError::Authority);
        }
        Ok(true)
    }
}
fn message(raw: &[u8], own: bool, label: Option<String>, pinned: bool) -> Option<EventMessage> {
    let p = codec::parse(raw, codec::Context::StoredChat).ok()?;
    if text::validate_payload(p.payload()).is_err() {
        return None;
    }
    if let codec::Payload::Chat {
        nickname,
        text,
        timestamp,
        ..
    } = p.payload()
    {
        let mut immutable = raw.to_vec();
        immutable[3] = 0;
        Some(EventMessage {
            content_key: Sha256::digest(&immutable).to_vec(),
            id: p.header().message_id.to_vec(),
            nickname: nickname.into(),
            text: text.into(),
            timestamp,
            own,
            staff_label: label,
            pinned,
        })
    } else {
        None
    }
}
