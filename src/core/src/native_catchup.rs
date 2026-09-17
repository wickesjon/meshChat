//! Native requester around the canonical link-bound, budgeted SYNC sessions.
//! Stored bytes never become live relay input or grant authentication by label.
use super::*;
use crate::sync::session::{End, Event, Identity, Request, RequestToken, Role};

#[derive(Default)]
pub(super) struct Catchup {
    requests: Vec<(LinkHandle, RequestToken)>,
    received: Vec<(LinkHandle, Vec<u8>, State, u64)>,
    complete: u64,
    incomplete: u64,
}
impl Catchup {
    pub fn ended(&mut self, events: Vec<Event>) {
        for event in events.into_iter().filter(|e| e.role == Role::Requesting) {
            self.requests.retain(|(link, _)| *link != event.link);
            if event.end == End::Complete {
                increment(&mut self.complete, 1);
            } else {
                increment(&mut self.incomplete, 1);
            }
        }
    }
    pub fn token(&self, link: &LinkHandle) -> Option<RequestToken> {
        self.requests
            .iter()
            .find(|(old, _)| old == link)
            .map(|(_, token)| *token)
    }
    pub fn received(&mut self, link: &LinkHandle, raw: Vec<u8>, state: State, now: u64) {
        self.received
            .retain(|(_, _, _, at)| now < at.saturating_add(120_000));
        if self.received.len() < 32 && matches!(state, State::Unverified | State::Pending) {
            self.received.push((link.clone(), raw, state, now));
        }
    }
    pub fn disconnected(&mut self, link: &LinkHandle) {
        if self.token(link).is_some() {
            increment(&mut self.incomplete, 1);
        }
        self.requests.retain(|(old, _)| old != link);
        self.received.retain(|(old, _, _, _)| old != link);
    }
}
#[derive(Clone, Debug, uniffi::Record)]
pub struct CatchupProgress {
    pub active: u32,
    pub complete: u64,
    pub incomplete: u64,
    /// Public sender-ID/message-ID pairs admitted to the historical owner.
    /// These arrival hints never grant a verified label or delivery state.
    pub arrivals: Vec<Vec<u8>>,
}
impl Runtime {
    fn sync_identity(&mut self) -> Result<Identity, TransportError> {
        Ok(Identity {
            message_id: self.next()?.to_be_bytes(),
            sender_id: self
                .messaging
                .identity
                .sender_id
                .as_slice()
                .try_into()
                .map_err(|_| TransportError::Identity)?,
        })
    }
    pub(super) fn cancel_catchup(&mut self, link: &LinkHandle) {
        if let Some(token) = self.catchup.token(link) {
            let mut events = Vec::new();
            let _ = self
                .beacon
                .sessions
                .cancel_request(token, self.now, &mut |e| events.push(e));
            self.catchup.ended(events);
        }
    }
    pub(super) fn continue_catchup(
        &mut self,
        link: &LinkHandle,
        out: &mut TransportEffects,
    ) -> Result<(), TransportError> {
        let identity = self.sync_identity()?;
        let mut raw = [0; 546];
        let request = self
            .beacon
            .sessions
            .continuation(link, identity, self.now, &mut raw)
            .map_err(|_| TransportError::Stale)?;
        if self
            .enqueue(link, &raw[..request.len], relay::Traffic::Local, 0, out)
            .is_err()
        {
            self.cancel_catchup(link);
        }
        Ok(())
    }
}
#[uniffi::export]
impl NativeTransport {
    /// Explicit foreground/reconnection request. Canonical session and ingress
    /// quotas remain authoritative; repeated calls cannot extend a deadline.
    pub fn request_catchup(
        &self,
        link: LinkHandle,
        now: u64,
    ) -> Result<TransportEffects, TransportError> {
        let mut s = self.state.lock().map_err(|_| TransportError::Unavailable)?;
        s.clock(now)?;
        if !s.links[s.index(&link)?].admitted {
            return Err(TransportError::Stale);
        }
        if s.catchup.token(&link).is_some() {
            return Err(TransportError::Busy);
        }
        let identity = s.sync_identity()?;
        let (held_count, filter) = s
            .beacon
            .cache
            .bloom(now)
            .map_err(|_| TransportError::Unavailable)?;
        let mut raw = [0; 546];
        let Runtime {
            beacon, ingress, ..
        } = &mut *s;
        let request = beacon
            .sessions
            .request(
                &link,
                Request {
                    identity,
                    held_count,
                    filter,
                },
                ingress,
                now,
                &mut raw,
            )
            .map_err(|_| TransportError::Busy)?;
        s.catchup.requests.push((link.clone(), request.token));
        let mut out = TransportEffects::default();
        if let Err(error) = s.enqueue(
            &link,
            &raw[..request.len],
            relay::Traffic::Local,
            0,
            &mut out,
        ) {
            s.cancel_catchup(&link);
            return Err(error);
        }
        Ok(out)
    }
    /// Only bytes admitted through the owned requesting session reach history.
    /// Signed/encrypted items use the same pending authentication work as live
    /// traffic; arbitrary wrappers or UI labels cannot manufacture provenance.
    pub fn process_catchup(
        &self,
        store: Arc<EncryptedStore>,
        provider: Arc<IdentityKeySession>,
        owner: Arc<crate::native_channels::NativeChannels>,
        now: u64,
        wall: i64,
    ) -> Result<CatchupProgress, messaging::MessagingError> {
        let mut s = self
            .state
            .lock()
            .map_err(|_| messaging::MessagingError::Unavailable)?;
        s.clock(now)?;
        let mut arrivals = Vec::new();
        s.messaging.store(&store)?;
        if provider
            .public_identity()
            .map_err(|_| messaging::MessagingError::Unavailable)?
            != s.messaging.identity
        {
            return Err(messaging::MessagingError::Stale);
        }
        let items = std::mem::take(&mut s.catchup.received);
        for (link, raw, state, at) in items {
            if now >= at.saturating_add(120_000) || s.index(&link).is_err() {
                continue;
            }
            s.organizer_received(&link, &raw, now, &mut TransportEffects::default())?;
            let changed = if raw.get(2).is_some_and(|flags| flags & 4 != 0) {
                s.organizer_authenticate(&store, now, wall)
                    .map_err(|_| messaging::MessagingError::Unavailable)?;
                true
            } else if state == State::Unverified {
                owner
                    .accept_stored(store.clone(), &raw, now, wall)
                    .map_err(|_| messaging::MessagingError::Unavailable)?
            } else {
                messaging::authenticate_pending(
                    &mut s,
                    &store,
                    &provider,
                    link,
                    raw.clone(),
                    now,
                    wall,
                )?
                .changed
            };
            if changed {
                let mut key = raw[12..20].to_vec();
                key.extend_from_slice(&raw[4..12]);
                arrivals.push(key);
            }
        }
        Ok(CatchupProgress {
            active: s.catchup.requests.len() as u32,
            complete: s.catchup.complete,
            incomplete: s.catchup.incomplete,
            arrivals,
        })
    }
}
