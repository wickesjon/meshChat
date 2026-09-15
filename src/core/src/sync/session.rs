//! Link-bound SYNC sessions. Outer ingress admission is required before requests
//! or deferred responses reach this component; no authentication is inferred.
use super::{Cache, bloom_contains};
use crate::ingress::{Ingress, State};
use crate::{
    LinkHandle,
    codec::{self, Context, Payload},
};
use sha2::{Digest, Sha256};
use std::mem::size_of;
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("invalid monotonic time")]
    Time,
    #[error("stale link/session/cursor/completion")]
    Stale,
    #[error("invalid SYNC packet or state")]
    Invalid,
    #[error("bounded session capacity exhausted")]
    Full,
    #[error("session or cursor sequence exhausted")]
    Sequence,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Serving,
    Requesting,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum End {
    Complete,
    Truncated,
    Failed,
    TimedOut,
    Disconnected,
    Conflict,
    Admission,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub link: LinkHandle,
    pub session: u16,
    pub role: Role,
    pub end: End,
    pub items: u16,
    pub bytes: u32,
}
struct Link {
    handle: LinkHandle,
    used: [u8; 8192],
    next_local: u32,
}
struct Serve {
    link: usize,
    id: u16,
    filter: [u8; 512],
    held_count: u16,
    deadline: u64,
    count: usize,
    position: usize,
    sequence: u16,
    items: u16,
    bytes: u32,
    page: u16,
    waiting: u32,
    inflight: Option<(u16, u8, u32)>,
}
struct ServeSlot {
    state: Option<Serve>,
    references: Vec<u64>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestOutcome {
    Started,
    Continued,
    Duplicate,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServeToken {
    instance: u64,
    generation: u64,
    id: u16,
    sequence: u16,
}
#[derive(Debug, Clone, Copy)]
pub struct Served {
    pub token: ServeToken,
    pub len: usize,
    pub flags: u8,
}
#[derive(Debug, Clone, Copy)]
pub struct Reservations {
    pub serving: usize,
    pub requesting: usize,
    pub gap_objects: usize,
    pub gap_bytes_per_session: usize,
    pub allocated_bytes: usize,
    pub snapshot_bytes: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct Identity {
    pub message_id: [u8; 8],
    pub sender_id: [u8; 8],
}
pub struct Request {
    pub identity: Identity,
    pub held_count: u16,
    pub filter: [u8; 512],
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestToken {
    instance: u64,
    generation: u64,
    id: u16,
}
#[derive(Debug, Clone, Copy)]
pub struct Requested {
    pub token: RequestToken,
    pub len: usize,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Received {
    Duplicate,
    Buffered,
    Processed,
    Page { cursor: u32 },
    Complete { truncated: bool },
}
pub struct Sink<'a> {
    pub events: &'a mut dyn FnMut(Event),
    pub messages: &'a mut dyn FnMut(&[u8], State),
}
struct Pending {
    sequence: u16,
    len: usize,
    raw: [u8; 1035],
}
struct Receiver {
    link: usize,
    id: u16,
    held_count: u16,
    filter: [u8; 512],
    prepared: u64,
    started: Option<u64>,
    expected: u16,
    seen: [[u8; 32]; 144],
    pending: [Option<Pending>; 4],
    gap: Option<u64>,
    items: u16,
    bytes: u32,
    page: u16,
    cursor: u32,
}
fn request_bytes(
    id: u16,
    cursor: u32,
    count: u16,
    filter: &[u8; 512],
    identity: Identity,
    out: &mut [u8],
) -> Result<usize, Error> {
    let mut payload = [0; 520];
    payload[..2].copy_from_slice(&id.to_be_bytes());
    payload[2..4].copy_from_slice(&count.to_be_bytes());
    payload[4..8].copy_from_slice(&cursor.to_be_bytes());
    payload[8..].copy_from_slice(filter);
    codec::serialize(
        codec::Header {
            kind: 3,
            flags: 0,
            ttl: 1,
            message_id: identity.message_id,
            sender_id: identity.sender_id,
            channel_id: [0; 4],
        },
        &payload,
        Context::Live,
        out,
    )
    .map_err(|_| Error::Invalid)
}
/// Encoded-byte accounting, independent of codec validity. Sizing witnesses may
/// use 1024-byte envelopes; this never makes such an envelope a valid CHAT.
pub fn fits_session_budget(items: u16, bytes: u32, next: usize) -> bool {
    items < 8 && bytes <= 8192 && next <= (8192 - bytes) as usize
}
pub struct Sessions {
    instance: u64,
    generation: u64,
    now: u64,
    cursor: u32,
    links: Vec<Option<Link>>,
    serving: Vec<ServeSlot>,
    requesting: Vec<Option<Receiver>>,
}
impl Sessions {
    pub fn new(instance: u64, now: u64) -> Result<Self, Error> {
        if instance == 0 || now > u64::MAX - 120_000 {
            return Err(Error::Invalid);
        }
        Ok(Self {
            instance,
            generation: 0,
            now,
            cursor: 0,
            links: std::iter::repeat_with(|| None).take(8).collect(),
            requesting: std::iter::repeat_with(|| None).take(2).collect(),
            serving: (0..2)
                .map(|_| ServeSlot {
                    state: None,
                    references: vec![0; 5000],
                })
                .collect(),
        })
    }
    fn time(&mut self, now: u64) -> Result<(), Error> {
        if now < self.now || now > u64::MAX - 120_000 {
            return Err(Error::Time);
        }
        self.now = now;
        Ok(())
    }
    fn link(&self, h: &LinkHandle) -> Result<usize, Error> {
        self.links
            .iter()
            .position(|l| l.as_ref().is_some_and(|l| &l.handle == h))
            .ok_or(Error::Stale)
    }
    pub fn register(&mut self, h: &LinkHandle, now: u64) -> Result<(), Error> {
        self.time(now)?;
        if h.instance_nonce != self.instance || h.generation <= self.generation {
            return Err(Error::Stale);
        }
        let i = self
            .links
            .iter()
            .position(Option::is_none)
            .ok_or(Error::Full)?;
        self.links[i] = Some(Link {
            handle: h.clone(),
            used: [0; 8192],
            next_local: 1,
        });
        self.generation = h.generation;
        Ok(())
    }
    fn end_serve(&mut self, i: usize, end: End, emit: &mut impl FnMut(Event)) {
        let s = self.serving[i].state.take().unwrap();
        self.serving[i].references.fill(0);
        emit(Event {
            link: self.links[s.link].as_ref().unwrap().handle.clone(),
            session: s.id,
            role: Role::Serving,
            end,
            items: s.items,
            bytes: s.bytes,
        });
    }
    pub fn advance(&mut self, now: u64, emit: &mut impl FnMut(Event)) -> Result<(), Error> {
        self.time(now)?;
        for i in 0..2 {
            if self.serving[i]
                .state
                .as_ref()
                .is_some_and(|s| s.deadline <= now)
            {
                self.end_serve(i, End::TimedOut, emit);
            }
        }
        for i in 0..2 {
            if self.requesting[i].as_ref().is_some_and(|r| {
                now >= r
                    .started
                    .map(|t| t + 120_000)
                    .unwrap_or(r.prepared + 30_000)
                    || r.gap.is_some_and(|t| now >= t + 30_000)
            }) {
                let r = self.requesting[i].take().unwrap();
                self.end_request(r, End::TimedOut, emit);
            }
        }
        Ok(())
    }
    pub fn disconnect(
        &mut self,
        h: &LinkHandle,
        now: u64,
        emit: &mut impl FnMut(Event),
    ) -> Result<(), Error> {
        self.advance(now, emit)?;
        let link = self.link(h)?;
        for i in 0..2 {
            if self.serving[i]
                .state
                .as_ref()
                .is_some_and(|s| s.link == link)
            {
                self.end_serve(i, End::Disconnected, emit);
            }
        }
        for i in 0..2 {
            if self.requesting[i].as_ref().is_some_and(|r| r.link == link) {
                let r = self.requesting[i].take().unwrap();
                self.end_request(r, End::Disconnected, emit);
            }
        }
        self.links[link] = None;
        Ok(())
    }
    /// The incoming initial request's serving bucket must already be charged by
    /// Ingress. Duplicate/cursor handling never creates a fresh walk or deadline.
    pub fn accept_request(
        &mut self,
        h: &LinkHandle,
        raw: &[u8],
        cache: &mut Cache,
        now: u64,
        emit: &mut impl FnMut(Event),
    ) -> Result<RequestOutcome, Error> {
        self.advance(now, emit)?;
        let link = self.link(h)?;
        let p = codec::parse(raw, Context::Live).map_err(|_| Error::Invalid)?;
        let Payload::SyncRequest {
            session_id: id,
            item_count,
            cursor,
            bloom,
        } = p.payload()
        else {
            return Err(Error::Invalid);
        };
        let active = self
            .serving
            .iter()
            .position(|s| s.state.as_ref().is_some_and(|s| s.link == link));
        if cursor == 0 && active.is_none_or(|i| self.serving[i].state.as_ref().unwrap().id != id) {
            let l = self.links[link].as_mut().unwrap();
            let bit = 1 << (id % 8);
            let byte = usize::from(id) / 8;
            if l.used[byte] & bit != 0 {
                return Err(Error::Stale);
            }
            l.used[byte] |= bit;
        }
        if let Some(i) = active {
            let s = self.serving[i].state.as_mut().unwrap();
            if s.id != id {
                return Err(Error::Full);
            }
            if s.filter != bloom || s.held_count != item_count {
                self.end_serve(i, End::Conflict, emit);
                return Err(Error::Invalid);
            }
            if cursor == 0 {
                return Ok(RequestOutcome::Duplicate);
            }
            if cursor != s.waiting || s.inflight.is_some() {
                return Err(Error::Stale);
            }
            s.waiting = 0;
            s.page = 0;
            return Ok(RequestOutcome::Continued);
        }
        if cursor != 0 {
            return Err(Error::Stale);
        }
        // Consume attempted IDs even at the global cap: replay of an admitted
        // but refused request cannot later create an unbudgeted new walk.
        let slot = self
            .serving
            .iter_mut()
            .find(|s| s.state.is_none())
            .ok_or(Error::Full)?;
        let count = cache
            .snapshot(now, &mut slot.references)
            .map_err(|_| Error::Full)?;
        slot.state = Some(Serve {
            link,
            id,
            filter: bloom.try_into().unwrap(),
            held_count: item_count,
            deadline: now + 120_000,
            count,
            position: 0,
            sequence: 0,
            items: 0,
            bytes: 0,
            page: 0,
            waiting: 0,
            inflight: None,
        });
        Ok(RequestOutcome::Started)
    }
    /// Stream one transport object at a time. The native driver feeds this to
    /// Relay and calls served_complete on its terminal result (including failure).
    pub fn next_served(
        &mut self,
        h: &LinkHandle,
        cache: &mut Cache,
        now: u64,
        out: &mut [u8],
        emit: &mut impl FnMut(Event),
    ) -> Result<Option<Served>, Error> {
        if out.len() < 1035 {
            return Err(Error::Full);
        }
        self.advance(now, emit)?;
        let link = self.link(h)?;
        let i = self
            .serving
            .iter()
            .position(|s| s.state.as_ref().is_some_and(|s| s.link == link))
            .ok_or(Error::Stale)?;
        let slot = &mut self.serving[i];
        let s = slot.state.as_mut().unwrap();
        if s.inflight.is_some() || s.waiting != 0 {
            return Ok(None);
        }
        let mut selected = None;
        while s.position < s.count {
            match cache.copy(slot.references[s.position], now, &mut out[11..]) {
                Ok((len, _)) => {
                    let id: [u8; 8] = out[15..23].try_into().unwrap();
                    if bloom_contains(&s.filter, &id) {
                        s.position += 1;
                        continue;
                    }
                    selected = Some(len);
                    break;
                }
                Err(super::Error::Missing) => s.position += 1,
                Err(_) => return Err(Error::Invalid),
            }
        }
        let (flags, cursor, len) = match selected {
            None => (5, 0, 0),
            Some(n) if !fits_session_budget(s.items, s.bytes, n) => (7, 0, 0),
            Some(_) if s.page >= 4 => {
                self.cursor = self.cursor.checked_add(1).ok_or(Error::Sequence)?;
                (3, self.cursor, 0)
            }
            Some(n) => {
                s.position += 1;
                s.page += 1;
                s.items += 1;
                s.bytes += n as u32;
                (0, 0, n)
            }
        };
        out[..2].copy_from_slice(&s.id.to_be_bytes());
        out[2..4].copy_from_slice(&s.sequence.to_be_bytes());
        out[4] = flags;
        out[5..9].copy_from_slice(&cursor.to_be_bytes());
        out[9..11].copy_from_slice(&(len as u16).to_be_bytes());
        s.inflight = Some((s.sequence, flags, cursor));
        Ok(Some(Served {
            token: ServeToken {
                instance: self.instance,
                generation: h.generation,
                id: s.id,
                sequence: s.sequence,
            },
            len: len + 11,
            flags,
        }))
    }
    pub fn served_complete(
        &mut self,
        token: ServeToken,
        success: bool,
        now: u64,
        emit: &mut impl FnMut(Event),
    ) -> Result<(), Error> {
        self.advance(now, emit)?;
        if token.instance != self.instance {
            return Err(Error::Stale);
        }
        let h = LinkHandle {
            instance_nonce: token.instance,
            generation: token.generation,
        };
        let link = self.link(&h)?;
        let i = self
            .serving
            .iter()
            .position(|s| {
                s.state.as_ref().is_some_and(|s| {
                    s.link == link
                        && s.id == token.id
                        && s.inflight.is_some_and(|(seq, _, _)| seq == token.sequence)
                })
            })
            .ok_or(Error::Stale)?;
        if !success {
            self.end_serve(i, End::Failed, emit);
            return Ok(());
        }
        let s = self.serving[i].state.as_mut().unwrap();
        let (_, flags, cursor) = s.inflight.take().unwrap();
        match flags {
            5 => self.end_serve(i, End::Complete, emit),
            7 => self.end_serve(i, End::Truncated, emit),
            _ => {
                s.sequence = s.sequence.checked_add(1).ok_or(Error::Sequence)?;
                if flags == 3 {
                    s.waiting = cursor;
                }
            }
        }
        Ok(())
    }
    fn end_request(&self, r: Receiver, end: End, emit: &mut impl FnMut(Event)) {
        emit(Event {
            link: self.links[r.link].as_ref().unwrap().handle.clone(),
            session: r.id,
            role: Role::Requesting,
            end,
            items: r.items,
            bytes: r.bytes,
        });
    }
    pub fn request(
        &mut self,
        h: &LinkHandle,
        request: Request,
        ingress: &mut Ingress,
        now: u64,
        out: &mut [u8],
    ) -> Result<Requested, Error> {
        self.time(now)?;
        let link = self.link(h)?;
        if out.len() < 546 || request.held_count > 5000 {
            return Err(Error::Invalid);
        }
        if self.requesting.iter().flatten().any(|r| r.link == link) {
            return Err(Error::Full);
        }
        let i = self
            .requesting
            .iter()
            .position(Option::is_none)
            .ok_or(Error::Full)?;
        let id = u16::try_from(self.links[link].as_ref().unwrap().next_local)
            .map_err(|_| Error::Sequence)?;
        if !ingress
            .new_requesting_session(h, now)
            .map_err(|_| Error::Stale)?
        {
            return Err(Error::Full);
        }
        let len = request_bytes(
            id,
            0,
            request.held_count,
            &request.filter,
            request.identity,
            out,
        )?;
        self.links[link].as_mut().unwrap().next_local += 1;
        self.requesting[i] = Some(Receiver {
            link,
            id,
            held_count: request.held_count,
            filter: request.filter,
            prepared: now,
            started: None,
            expected: 0,
            seen: [[0; 32]; 144],
            pending: std::array::from_fn(|_| None),
            gap: None,
            items: 0,
            bytes: 0,
            page: 0,
            cursor: 0,
        });
        Ok(Requested {
            token: RequestToken {
                instance: self.instance,
                generation: h.generation,
                id,
            },
            len,
        })
    }
    /// Call on the first native attempt of the initial request. Repeated calls or
    /// continuation sends never extend the original absolute session deadline.
    pub fn request_started(&mut self, token: RequestToken, now: u64) -> Result<(), Error> {
        self.time(now)?;
        if token.instance != self.instance {
            return Err(Error::Stale);
        }
        let h = LinkHandle {
            instance_nonce: token.instance,
            generation: token.generation,
        };
        let link = self.link(&h)?;
        let r = self
            .requesting
            .iter_mut()
            .flatten()
            .find(|r| r.link == link && r.id == token.id)
            .ok_or(Error::Stale)?;
        if r.started.is_none() {
            if now >= r.prepared + 30_000 {
                return Err(Error::Stale);
            }
            r.started = Some(now);
        }
        Ok(())
    }
    pub fn cancel_request(
        &mut self,
        token: RequestToken,
        now: u64,
        emit: &mut impl FnMut(Event),
    ) -> Result<(), Error> {
        self.advance(now, emit)?;
        if token.instance != self.instance {
            return Err(Error::Stale);
        }
        let h = LinkHandle {
            instance_nonce: token.instance,
            generation: token.generation,
        };
        let link = self.link(&h)?;
        let i = self
            .requesting
            .iter()
            .position(|r| {
                r.as_ref()
                    .is_some_and(|r| r.link == link && r.id == token.id)
            })
            .ok_or(Error::Stale)?;
        let r = self.requesting[i].take().unwrap();
        self.end_request(r, End::Failed, emit);
        Ok(())
    }
    pub fn continuation(
        &mut self,
        h: &LinkHandle,
        identity: Identity,
        now: u64,
        out: &mut [u8],
    ) -> Result<Requested, Error> {
        self.time(now)?;
        let link = self.link(h)?;
        let r = self
            .requesting
            .iter_mut()
            .flatten()
            .find(|r| r.link == link)
            .ok_or(Error::Stale)?;
        if out.len() < 546 {
            return Err(Error::Full);
        }
        if r.cursor == 0 || r.started.is_none_or(|t| now >= t + 120_000) {
            return Err(Error::Stale);
        }
        let len = request_bytes(r.id, r.cursor, r.held_count, &r.filter, identity, out)?;
        r.cursor = 0;
        Ok(Requested {
            token: RequestToken {
                instance: self.instance,
                generation: h.generation,
                id: r.id,
            },
            len,
        })
    }
    /// Only complete objects from Ingress::receive_deferred_sync may enter here.
    /// Correlate/order before invoking ordinary inner admission exactly once.
    pub fn receive(
        &mut self,
        h: &LinkHandle,
        raw: &[u8],
        ingress: &mut Ingress,
        now: u64,
        sink: &mut Sink<'_>,
    ) -> Result<Received, Error> {
        self.advance(now, &mut sink.events)?;
        let link = self.link(h)?;
        let i = self
            .requesting
            .iter()
            .position(|r| r.as_ref().is_some_and(|r| r.link == link))
            .ok_or(Error::Stale)?;
        let crate::framing::Transport::SyncItem { session, .. } =
            crate::framing::transport(1, raw).map_err(|_| Error::Invalid)?
        else {
            return Err(Error::Invalid);
        };
        if self.requesting[i].as_ref().unwrap().id != session {
            return Err(Error::Stale);
        }
        let mut r = self.requesting[i].take().unwrap();
        let result = Self::receive_inner(&mut r, h, raw, ingress, now, sink.messages);
        match result {
            Ok(Received::Complete { truncated }) => self.end_request(
                r,
                if truncated {
                    End::Truncated
                } else {
                    End::Complete
                },
                &mut sink.events,
            ),
            Ok(_) => self.requesting[i] = Some(r),
            Err(end) => {
                self.end_request(r, end, &mut sink.events);
                return Err(Error::Invalid);
            }
        }
        result.map_err(|_| Error::Invalid)
    }
    fn receive_inner(
        r: &mut Receiver,
        h: &LinkHandle,
        raw: &[u8],
        ingress: &mut Ingress,
        now: u64,
        message: &mut dyn FnMut(&[u8], State),
    ) -> Result<Received, End> {
        let crate::framing::Transport::SyncItem {
            session, sequence, ..
        } = crate::framing::transport(1, raw).map_err(|_| End::Conflict)?
        else {
            return Err(End::Conflict);
        };
        if session != r.id || r.started.is_none() {
            return Err(End::Conflict);
        }
        let digest: [u8; 32] = Sha256::digest(raw).into();
        if sequence < r.expected {
            return if r.seen[usize::from(sequence)] == digest {
                Ok(Received::Duplicate)
            } else {
                Err(End::Conflict)
            };
        }
        if let Some(p) = r.pending.iter().flatten().find(|p| p.sequence == sequence) {
            return if p.raw[..p.len] == *raw {
                Ok(Received::Duplicate)
            } else {
                Err(End::Conflict)
            };
        }
        if r.cursor != 0 {
            return Err(End::Conflict);
        }
        if sequence > r.expected {
            let p = r
                .pending
                .iter_mut()
                .find(|p| p.is_none())
                .ok_or(End::Failed)?;
            let mut pending = Pending {
                sequence,
                len: raw.len(),
                raw: [0; 1035],
            };
            pending.raw[..raw.len()].copy_from_slice(raw);
            *p = Some(pending);
            r.gap.get_or_insert(now);
            return Ok(Received::Buffered);
        }
        let mut result = Self::ordered(r, h, raw, ingress, now, message)?;
        while !matches!(result, Received::Complete { .. } | Received::Page { .. }) {
            let Some(i) = r
                .pending
                .iter()
                .position(|p| p.as_ref().is_some_and(|p| p.sequence == r.expected))
            else {
                break;
            };
            let p = r.pending[i].take().unwrap();
            result = Self::ordered(r, h, &p.raw[..p.len], ingress, now, message)?;
        }
        if r.pending.iter().all(Option::is_none) {
            r.gap = None;
        } else if matches!(result, Received::Complete { .. } | Received::Page { .. }) {
            return Err(End::Conflict);
        }
        Ok(result)
    }
    fn ordered(
        r: &mut Receiver,
        h: &LinkHandle,
        raw: &[u8],
        ingress: &mut Ingress,
        now: u64,
        message: &mut dyn FnMut(&[u8], State),
    ) -> Result<Received, End> {
        if usize::from(r.expected) >= r.seen.len() {
            return Err(End::Failed);
        }
        let crate::framing::Transport::SyncItem {
            flags,
            next_cursor,
            blob,
            ..
        } = crate::framing::transport(1, raw).map_err(|_| End::Conflict)?
        else {
            return Err(End::Conflict);
        };
        // With at most 15+120 admitted values/link in a 120-second walk,
        // 144 hash slots cover every possible in-order response under ingress.
        r.seen[usize::from(r.expected)] = Sha256::digest(raw).into();
        r.expected += 1;
        if flags == 0 {
            if r.page >= 4 || !fits_session_budget(r.items, r.bytes, blob.len()) {
                return Err(End::Failed);
            }
            let state = ingress
                .admit_stored_sync(h, blob, now)
                .map_err(|_| End::Admission)?
                .map_err(|_| End::Admission)?;
            r.items += 1;
            r.page += 1;
            r.bytes += blob.len() as u32;
            message(blob, state);
            Ok(Received::Processed)
        } else if flags == 3 {
            r.cursor = next_cursor;
            r.page = 0;
            Ok(Received::Page {
                cursor: next_cursor,
            })
        } else {
            Ok(Received::Complete {
                truncated: flags == 7,
            })
        }
    }
    pub fn reservations(&self) -> Reservations {
        Reservations {
            serving: self.serving.iter().filter(|s| s.state.is_some()).count(),
            requesting: self.requesting.iter().flatten().count(),
            gap_objects: self
                .requesting
                .iter()
                .flatten()
                .map(|r| r.pending.iter().flatten().count())
                .sum(),
            gap_bytes_per_session: 4 * size_of::<Option<Pending>>(),
            snapshot_bytes: 5000 * size_of::<u64>() + size_of::<ServeSlot>(),
            allocated_bytes: size_of::<Self>()
                + self.links.capacity() * size_of::<Option<Link>>()
                + self.requesting.capacity() * size_of::<Option<Receiver>>()
                + self.serving.capacity() * size_of::<ServeSlot>()
                + self
                    .serving
                    .iter()
                    .map(|s| s.references.capacity() * size_of::<u64>())
                    .sum::<usize>(),
        }
    }
}
