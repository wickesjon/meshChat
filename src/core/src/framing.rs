//! MC-006/007/008 outer framing. No authentication, admission buckets or native I/O.
//! `Reassembler::ingest_admitted` must be called only after frame/byte admission.

use crate::LinkHandle;
use crate::codec::{self, Context};

pub const MIN_CAPACITY: usize = 146;
pub const MAX_CAPACITY: usize = 512;
pub const GROUP_LIFETIME_MS: u64 = 30_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("unsupported directional capacity")]
    Capacity,
    #[error("malformed frame or object")]
    Invalid,
    #[error("unknown or stale connection generation")]
    Link,
    #[error("resource limit")]
    Limit,
    #[error("invalid monotonic time")]
    Time,
    #[error("fragment conflict or rejected group")]
    Rejected,
}

fn capacity(c: usize) -> Result<(), Error> {
    if (MIN_CAPACITY..=MAX_CAPACITY).contains(&c) {
        Ok(())
    } else {
        Err(Error::Capacity)
    }
}
fn be16(b: &[u8]) -> Result<u16, Error> {
    Ok(u16::from_be_bytes(
        b.get(..2)
            .ok_or(Error::Invalid)?
            .try_into()
            .map_err(|_| Error::Invalid)?,
    ))
}
fn be32(b: &[u8]) -> Result<u32, Error> {
    Ok(u32::from_be_bytes(
        b.get(..4)
            .ok_or(Error::Invalid)?
            .try_into()
            .map_err(|_| Error::Invalid)?,
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport<'a> {
    SyncItem {
        session: u16,
        sequence: u16,
        flags: u8,
        next_cursor: u32,
        blob: &'a [u8],
    },
    Hello(&'a [u8]),
    LinkProof(&'a [u8]),
}

/// Structural validation only. Session, HELLO role/nonce uniqueness, self-key,
/// proof freshness and authentication checks belong to their respective consumers.
pub fn transport(kind: u8, body: &[u8]) -> Result<Transport<'_>, Error> {
    match kind {
        1 if (11..=1035).contains(&body.len()) => {
            let session = be16(body)?;
            let sequence = be16(&body[2..])?;
            let flags = body[4];
            let next_cursor = be32(&body[5..])?;
            let len = usize::from(be16(&body[9..])?);
            let blob = &body[11..];
            if blob.len() != len {
                return Err(Error::Invalid);
            }
            match (flags, next_cursor, len) {
                (0, 0, 26..=1024) => {
                    codec::parse(blob, Context::StoredChat).map_err(|_| Error::Invalid)?;
                }
                (3, 1..=u32::MAX, 0) | (5 | 7, 0, 0) => {}
                _ => return Err(Error::Invalid),
            }
            Ok(Transport::SyncItem {
                session,
                sequence,
                flags,
                next_cursor,
                blob,
            })
        }
        2 if body.len() == 54 => {
            if body[0] != 1 || body[1] > 1 || body[6..22].iter().all(|b| *b == 0) {
                return Err(Error::Invalid);
            }
            capacity(usize::from(be16(&body[2..])?))?;
            capacity(usize::from(be16(&body[4..])?))?;
            Ok(Transport::Hello(body))
        }
        3 if body.len() == 66 && body[0] == 1 && body[1] <= 1 => Ok(Transport::LinkProof(body)),
        _ => Err(Error::Invalid),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Frame<'a> {
    Logical(&'a [u8]),
    WholeTransport { kind: u8, body: &'a [u8] },
    Fragment(Fragment<'a>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupId {
    Logical { message: [u8; 8], group: u16 },
    Transport { kind: u8, transfer: u16 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fragment<'a> {
    pub id: GroupId,
    pub index: u8,
    pub count: u8,
    pub total: u16,
    pub slice: &'a [u8],
}

fn outer(raw: &[u8], limit: usize) -> Result<(u8, &[u8]), Error> {
    if raw.len() < 4
        || raw.len() > limit
        || raw[1] != 0
        || usize::from(be16(&raw[2..])?) != raw.len() - 4
    {
        return Err(Error::Invalid);
    }
    Ok((raw[0], &raw[4..]))
}

/// Bootstrap permits only whole HELLO within a known native receive limit.
pub fn bootstrap_hello(raw: &[u8], native_receive_limit: usize) -> Result<&[u8], Error> {
    let (kind, body) = outer(raw, native_receive_limit.min(MAX_CAPACITY))?;
    if kind != 2 || body.first() != Some(&2) {
        return Err(Error::Invalid);
    }
    match transport(2, &body[1..])? {
        Transport::Hello(b) => Ok(b),
        _ => Err(Error::Invalid),
    }
}

pub fn parse_frame(raw: &[u8], receive_capacity: usize) -> Result<Frame<'_>, Error> {
    capacity(receive_capacity)?;
    let (kind, body) = outer(raw, receive_capacity)?;
    match kind {
        0 => {
            codec::parse(body, Context::Live).map_err(|_| Error::Invalid)?;
            Ok(Frame::Logical(body))
        }
        2 => {
            let (&kind, body) = body.split_first().ok_or(Error::Invalid)?;
            transport(kind, body)?;
            Ok(Frame::WholeTransport { kind, body })
        }
        1 | 3 => {
            let id = fragment_id(raw).ok_or(Error::Invalid)?;
            let (index, count, total, slice, maximum, max_count, minimum, ceiling) = if kind == 1 {
                (
                    body[10],
                    body[11],
                    be16(&body[12..])?,
                    &body[14..],
                    receive_capacity - 18,
                    8,
                    26,
                    1024,
                )
            } else {
                if body[7..12].iter().any(|b| *b != 0) {
                    return Err(Error::Invalid);
                }
                (
                    body[3],
                    body[4],
                    be16(&body[5..])?,
                    &body[12..],
                    receive_capacity - 16,
                    16,
                    11,
                    1035,
                )
            };
            if count == 0
                || count > max_count
                || index >= count
                || slice.is_empty()
                || !(minimum..=ceiling).contains(&total)
                || total < u16::from(count)
                || usize::from(total) > usize::from(count) * maximum
                || slice.len() > usize::from(total)
            {
                return Err(Error::Invalid);
            }
            Ok(Frame::Fragment(Fragment {
                id,
                index,
                count,
                total,
                slice,
            }))
        }
        _ => Err(Error::Invalid),
    }
}

// Extract only complete, known fragment envelope keys; even invalid metadata can
// be rejected without poisoning accepted-message identity. Never allocate here.
fn fragment_id(raw: &[u8]) -> Option<GroupId> {
    match raw.first()? {
        1 if raw.len() >= 18 => Some(GroupId::Logical {
            message: raw[4..12].try_into().ok()?,
            group: be16(&raw[12..]).ok()?,
        }),
        3 if raw.len() >= 16 && raw[4] == 1 => Some(GroupId::Transport {
            kind: 1,
            transfer: be16(&raw[5..]).ok()?,
        }),
        _ => None,
    }
}

/// Borrowed, stateless per-object encoder. Each frame is written on demand rather
/// than allocating an outbound frame queue. Capacity is fixed for the object.
pub struct Encoder<'a> {
    body: &'a [u8],
    capacity: usize,
    id: GroupId,
    whole: bool,
    count: usize,
}
impl<'a> Encoder<'a> {
    pub fn logical(body: &'a [u8], transmit_capacity: usize, group: u16) -> Result<Self, Error> {
        capacity(transmit_capacity)?;
        let packet = codec::parse(body, Context::Live).map_err(|_| Error::Invalid)?;
        Ok(Self::new(
            body,
            transmit_capacity,
            GroupId::Logical {
                message: packet.header().message_id,
                group,
            },
        ))
    }
    pub fn transport(
        kind: u8,
        body: &'a [u8],
        transmit_capacity: usize,
        transfer: u16,
    ) -> Result<Self, Error> {
        capacity(transmit_capacity)?;
        transport(kind, body)?;
        Ok(Self::new(
            body,
            transmit_capacity,
            GroupId::Transport { kind, transfer },
        ))
    }
    fn new(body: &'a [u8], capacity: usize, id: GroupId) -> Self {
        let logical = matches!(id, GroupId::Logical { .. });
        let whole = body.len() + if logical { 4 } else { 5 } <= capacity;
        let slice = capacity - if logical { 18 } else { 16 };
        let count = if whole { 1 } else { body.len().div_ceil(slice) };
        Self {
            body,
            capacity,
            id,
            whole,
            count,
        }
    }
    pub fn frame_count(&self) -> usize {
        self.count
    }
    pub fn frame(&self, index: usize, output: &mut [u8]) -> Result<usize, Error> {
        if index >= self.count {
            return Err(Error::Invalid);
        }
        let logical = matches!(self.id, GroupId::Logical { .. });
        let overhead = if self.whole {
            if logical { 4 } else { 5 }
        } else if logical {
            18
        } else {
            16
        };
        let maximum = self.capacity - overhead;
        let start = if self.whole { 0 } else { index * maximum };
        let slice = &self.body[start..self.body.len().min(start + maximum)];
        let len = overhead + slice.len();
        let out = output.get_mut(..len).ok_or(Error::Capacity)?;
        out.fill(0);
        out[0] = match (logical, self.whole) {
            (true, true) => 0,
            (true, false) => 1,
            (false, true) => 2,
            (false, false) => 3,
        };
        out[2..4].copy_from_slice(&((len - 4) as u16).to_be_bytes());
        match self.id {
            GroupId::Logical { message, group } if !self.whole => {
                out[4..12].copy_from_slice(&message);
                out[12..14].copy_from_slice(&group.to_be_bytes());
                out[14] = index as u8;
                out[15] = self.count as u8;
                out[16..18].copy_from_slice(&(self.body.len() as u16).to_be_bytes());
            }
            GroupId::Transport { kind, transfer } => {
                out[4] = kind;
                if !self.whole {
                    out[5..7].copy_from_slice(&transfer.to_be_bytes());
                    out[7] = index as u8;
                    out[8] = self.count as u8;
                    out[9..11].copy_from_slice(&(self.body.len() as u16).to_be_bytes());
                }
            }
            _ => {}
        }
        out[overhead..].copy_from_slice(slice);
        Ok(len)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectKind {
    Logical,
    Transport(u8),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Completed {
    pub kind: ObjectKind,
    pub len: usize,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Counters {
    pub frames: u64,
    pub bytes: u64,
    pub malformed: u64,
    pub rejected: u64,
    pub duplicates: u64,
    pub evicted: u64,
    pub completed: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reservations {
    /// Fixed pool capacities include slot metadata and each Vec control block.
    pub logical_bytes: usize,
    pub transport_bytes: usize,
    pub rejected_bytes: usize,
    pub manager_bytes: usize,
    pub logical_groups: usize,
    pub transport_groups: usize,
    pub rejected_groups: usize,
    pub peak_logical_groups: usize,
    pub peak_transport_groups: usize,
    pub peak_rejected_groups: usize,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkReservations {
    pub logical_bytes: usize,
    pub transport_bytes: usize,
    pub rejected_bytes: usize,
    pub logical_groups: usize,
    pub transport_groups: usize,
    pub rejected_groups: usize,
}
#[derive(Clone, Copy, PartialEq, Eq)]
struct Key {
    generation: u64,
    id: GroupId,
}
struct Group<const N: usize> {
    key: Key,
    expires: u64,
    started: u64,
    count: u8,
    total: u16,
    used: u16,
    offsets: [u16; 16],
    lengths: [u16; 16],
    data: [u8; N],
}
impl<const N: usize> Group<N> {
    fn new(key: Key, fragment: Fragment<'_>, now: u64) -> Self {
        Self {
            key,
            expires: now + GROUP_LIFETIME_MS,
            started: now,
            count: fragment.count,
            total: fragment.total,
            used: 0,
            offsets: [0; 16],
            lengths: [0; 16],
            data: [0; N],
        }
    }
    /// false means a byte-identical duplicate, true a new slice.
    fn add(&mut self, fragment: Fragment<'_>) -> Result<bool, Error> {
        if fragment.count != self.count || fragment.total != self.total {
            return Err(Error::Rejected);
        }
        let i = usize::from(fragment.index);
        let offset = usize::from(self.offsets[i]);
        let length = usize::from(self.lengths[i]);
        if length != 0 {
            return if self.data.get(offset..offset + length) == Some(fragment.slice) {
                Ok(false)
            } else {
                Err(Error::Rejected)
            };
        }
        let start = usize::from(self.used);
        let end = start
            .checked_add(fragment.slice.len())
            .ok_or(Error::Rejected)?;
        if end > usize::from(self.total) {
            return Err(Error::Rejected);
        }
        self.data
            .get_mut(start..end)
            .ok_or(Error::Rejected)?
            .copy_from_slice(fragment.slice);
        self.offsets[i] = self.used;
        self.lengths[i] = fragment.slice.len() as u16;
        self.used = end as u16;
        Ok(true)
    }
    fn complete(&self) -> bool {
        self.lengths[..usize::from(self.count)]
            .iter()
            .all(|n| *n > 0)
    }
    fn assemble(&self, output: &mut [u8]) -> Result<Completed, Error> {
        if self.used != self.total {
            return Err(Error::Rejected);
        }
        let len = usize::from(self.total);
        let out = output.get_mut(..len).ok_or(Error::Capacity)?;
        let mut end = 0;
        for i in 0..usize::from(self.count) {
            let offset = usize::from(self.offsets[i]);
            let n = usize::from(self.lengths[i]);
            out[end..end + n].copy_from_slice(&self.data[offset..offset + n]);
            end += n;
        }
        let kind = match self.key.id {
            GroupId::Logical { message, .. } => {
                let p = codec::parse(out, Context::Live).map_err(|_| Error::Rejected)?;
                if p.header().message_id != message {
                    return Err(Error::Rejected);
                }
                ObjectKind::Logical
            }
            GroupId::Transport { kind, .. } => {
                transport(kind, out).map_err(|_| Error::Rejected)?;
                ObjectKind::Transport(kind)
            }
        };
        Ok(Completed { kind, len })
    }
}
#[derive(Clone, Copy)]
struct Rejected {
    key: Key,
    expires: u64,
}
#[derive(Clone, Copy)]
struct Link {
    generation: u64,
    capacity: usize,
}

fn pool<T>(size: usize) -> Vec<Option<T>> {
    let mut pool = Vec::with_capacity(size);
    pool.resize_with(size, || None);
    pool
}
fn reserved<T>(pool: &Vec<Option<T>>) -> usize {
    pool.capacity() * std::mem::size_of::<Option<T>>() + std::mem::size_of::<Vec<Option<T>>>()
}
fn count<T>(pool: &[Option<T>]) -> usize {
    pool.iter().filter(|s| s.is_some()).count()
}

/// Fixed-capacity pools: incoming fragments cannot grow allocations. Fresh link
/// generations must be registered in increasing order for this process instance.
/// Callers keep node admission state outside this object; reconnect never refills it.
pub struct Reassembler {
    instance: u64,
    highest_generation: u64,
    max_links: usize,
    links: [Option<Link>; 8],
    now: u64,
    logical: Vec<Option<Group<1024>>>,
    transport: Vec<Option<Group<1035>>>,
    rejected: Vec<Option<Rejected>>,
    counters: Counters,
    peaks: [usize; 3],
}
impl Reassembler {
    pub fn new(instance: u64, max_links: usize, now: u64) -> Result<Self, Error> {
        if instance == 0 || !(1..=8).contains(&max_links) {
            return Err(Error::Limit);
        }
        if now > u64::MAX - GROUP_LIFETIME_MS {
            return Err(Error::Time);
        }
        let value = Self {
            instance,
            highest_generation: 0,
            max_links,
            links: [None; 8],
            now,
            logical: pool(64),
            transport: pool(32),
            rejected: pool(512),
            counters: Counters::default(),
            peaks: [0; 3],
        };
        let r = value.reservations();
        if r.logical_bytes > 96 * 1024
            || r.transport_bytes > 64 * 1024
            || r.rejected_bytes > 64 * 1024
            || 8 * std::mem::size_of::<Option<Group<1024>>>() > 12 * 1024
            || 4 * std::mem::size_of::<Option<Group<1035>>>() > 8 * 1024
            || 64 * std::mem::size_of::<Option<Rejected>>() > 8 * 1024
        {
            return Err(Error::Limit);
        }
        Ok(value)
    }
    pub fn register(&mut self, handle: &LinkHandle, receive_capacity: usize) -> Result<(), Error> {
        capacity(receive_capacity)?;
        if handle.instance_nonce != self.instance || handle.generation == 0 {
            return Err(Error::Link);
        }
        if let Some(link) = self
            .links
            .iter()
            .flatten()
            .find(|l| l.generation == handle.generation)
        {
            return if link.capacity == receive_capacity {
                Ok(())
            } else {
                Err(Error::Capacity)
            };
        }
        if handle.generation <= self.highest_generation {
            return Err(Error::Link);
        }
        if self.links.iter().flatten().count() >= self.max_links {
            return Err(Error::Limit);
        }
        let slot = self
            .links
            .iter_mut()
            .find(|l| l.is_none())
            .ok_or(Error::Limit)?;
        *slot = Some(Link {
            generation: handle.generation,
            capacity: receive_capacity,
        });
        self.highest_generation = handle.generation;
        Ok(())
    }
    fn link(&self, handle: &LinkHandle) -> Result<Link, Error> {
        if handle.instance_nonce != self.instance {
            return Err(Error::Link);
        }
        self.links
            .iter()
            .flatten()
            .find(|l| l.generation == handle.generation)
            .copied()
            .ok_or(Error::Link)
    }
    pub fn disconnect(&mut self, handle: &LinkHandle) -> Result<(), Error> {
        self.link(handle)?;
        let generation = handle.generation;
        for slot in &mut self.links {
            if slot.is_some_and(|l| l.generation == generation) {
                *slot = None;
            }
        }
        for slot in &mut self.logical {
            if slot
                .as_ref()
                .is_some_and(|g| g.key.generation == generation)
            {
                *slot = None;
            }
        }
        for slot in &mut self.transport {
            if slot
                .as_ref()
                .is_some_and(|g| g.key.generation == generation)
            {
                *slot = None;
            }
        }
        for slot in &mut self.rejected {
            if slot.is_some_and(|g| g.key.generation == generation) {
                *slot = None;
            }
        }
        Ok(())
    }
    /// A changed native capacity invalidates the admitted transfer context, even
    /// on an increase. Re-admit with a fresh connection generation before reuse.
    pub fn capacity_changed(
        &mut self,
        handle: &LinkHandle,
        observed: usize,
    ) -> Result<bool, Error> {
        if self.link(handle)?.capacity == observed {
            return Ok(false);
        }
        self.disconnect(handle)?;
        Ok(true)
    }
    pub fn advance(&mut self, now: u64) -> Result<(), Error> {
        if now < self.now || now > u64::MAX - GROUP_LIFETIME_MS {
            return Err(Error::Time);
        }
        self.now = now;
        for slot in &mut self.logical {
            if slot.as_ref().is_some_and(|g| g.expires <= now) {
                *slot = None;
            }
        }
        for slot in &mut self.transport {
            if slot.as_ref().is_some_and(|g| g.expires <= now) {
                *slot = None;
            }
        }
        for slot in &mut self.rejected {
            if slot.is_some_and(|g| g.expires <= now) {
                *slot = None;
            }
        }
        Ok(())
    }
    pub fn counters(&self) -> Counters {
        self.counters
    }
    pub fn reservations(&self) -> Reservations {
        Reservations {
            logical_bytes: reserved(&self.logical),
            transport_bytes: reserved(&self.transport),
            rejected_bytes: reserved(&self.rejected),
            // Pool Vec control blocks are already charged above, so avoid counting twice.
            manager_bytes: std::mem::size_of::<Self>() - 3 * std::mem::size_of::<Vec<()>>(),
            logical_groups: count(&self.logical),
            transport_groups: count(&self.transport),
            rejected_groups: count(&self.rejected),
            peak_logical_groups: self.peaks[0],
            peak_transport_groups: self.peaks[1],
            peak_rejected_groups: self.peaks[2],
        }
    }
    pub fn link_reservations(&self, handle: &LinkHandle) -> Result<LinkReservations, Error> {
        self.link(handle)?;
        let generation = handle.generation;
        let logical_groups = self
            .logical
            .iter()
            .flatten()
            .filter(|g| g.key.generation == generation)
            .count();
        let transport_groups = self
            .transport
            .iter()
            .flatten()
            .filter(|g| g.key.generation == generation)
            .count();
        let rejected_groups = self
            .rejected
            .iter()
            .flatten()
            .filter(|g| g.key.generation == generation)
            .count();
        Ok(LinkReservations {
            logical_groups,
            transport_groups,
            rejected_groups,
            logical_bytes: logical_groups * std::mem::size_of::<Option<Group<1024>>>(),
            transport_bytes: transport_groups * std::mem::size_of::<Option<Group<1035>>>(),
            rejected_bytes: rejected_groups * std::mem::size_of::<Option<Rejected>>(),
        })
    }
    fn peak(&mut self) {
        self.peaks[0] = self.peaks[0].max(count(&self.logical));
        self.peaks[1] = self.peaks[1].max(count(&self.transport));
        self.peaks[2] = self.peaks[2].max(count(&self.rejected));
    }
    fn reject(&mut self, key: Key, expires: u64) {
        if self.rejected.iter().flatten().any(|r| r.key == key) {
            return;
        }
        let at_link = self
            .rejected
            .iter()
            .flatten()
            .filter(|r| r.key.generation == key.generation)
            .count();
        let index = if at_link >= 64 {
            self.rejected
                .iter()
                .enumerate()
                .filter_map(|(i, r)| {
                    r.filter(|r| r.key.generation == key.generation)
                        .map(|r| (i, r.expires))
                })
                .min_by_key(|p| p.1)
                .map(|p| p.0)
        } else {
            self.rejected.iter().position(|r| r.is_none()).or_else(|| {
                self.rejected
                    .iter()
                    .enumerate()
                    .filter_map(|(i, r)| r.map(|r| (i, r.expires)))
                    .min_by_key(|p| p.1)
                    .map(|p| p.0)
            })
        };
        if let Some(i) = index {
            self.rejected[i] = Some(Rejected { key, expires });
        }
        self.peak();
    }
    fn abort(&mut self, key: Key) -> u64 {
        for slot in &mut self.logical {
            if slot.as_ref().is_some_and(|g| g.key == key) {
                return slot.take().expect("matching slot").expires;
            }
        }
        for slot in &mut self.transport {
            if slot.as_ref().is_some_and(|g| g.key == key) {
                return slot.take().expect("matching slot").expires;
            }
        }
        self.now + GROUP_LIFETIME_MS
    }
    /// All returned output bytes are valid only for `Ok(Some(completed))`.
    /// A fixed 1,035-byte output buffer is required before any group mutation.
    /// Rate admission, native intake staging and authentication are separate gates.
    pub fn ingest_admitted(
        &mut self,
        handle: &LinkHandle,
        raw: &[u8],
        now: u64,
        output: &mut [u8],
    ) -> Result<Option<Completed>, Error> {
        if output.len() < 1035 {
            return Err(Error::Capacity);
        }
        let link = self.link(handle)?;
        self.advance(now)?;
        self.counters.frames = self.counters.frames.saturating_add(1);
        self.counters.bytes = self
            .counters
            .bytes
            .saturating_add(u64::try_from(raw.len()).unwrap_or(u64::MAX));
        // Oversized native values never get copied or reserve tracking state.
        if raw.len() > MAX_CAPACITY {
            self.counters.malformed = self.counters.malformed.saturating_add(1);
            return Err(Error::Invalid);
        }
        let frame = match parse_frame(raw, link.capacity) {
            Ok(f) => f,
            Err(e) => {
                self.counters.malformed = self.counters.malformed.saturating_add(1);
                if let Some(id) = fragment_id(raw) {
                    let key = Key {
                        generation: handle.generation,
                        id,
                    };
                    let expires = self.abort(key);
                    self.reject(key, expires);
                }
                return Err(e);
            }
        };
        let result = match frame {
            Frame::Logical(body) => {
                output[..body.len()].copy_from_slice(body);
                Some(Completed {
                    kind: ObjectKind::Logical,
                    len: body.len(),
                })
            }
            Frame::WholeTransport { kind, body } => {
                output[..body.len()].copy_from_slice(body);
                Some(Completed {
                    kind: ObjectKind::Transport(kind),
                    len: body.len(),
                })
            }
            Frame::Fragment(fragment) => {
                let key = Key {
                    generation: handle.generation,
                    id: fragment.id,
                };
                if self.rejected.iter().flatten().any(|r| r.key == key) {
                    self.counters.rejected = self.counters.rejected.saturating_add(1);
                    return Err(Error::Rejected);
                }
                let result = if matches!(fragment.id, GroupId::Logical { .. }) {
                    receive(&mut self.logical, 8, key, fragment, now, output)
                } else {
                    receive(&mut self.transport, 4, key, fragment, now, output)
                };
                if let Some((old, expires)) = result.evicted {
                    self.counters.evicted = self.counters.evicted.saturating_add(1);
                    self.reject(old, expires);
                }
                if result.duplicate {
                    self.counters.duplicates = self.counters.duplicates.saturating_add(1);
                }
                self.peak();
                match result.value {
                    Ok(value) => value,
                    Err((e, expires)) => {
                        self.counters.rejected = self.counters.rejected.saturating_add(1);
                        self.reject(key, expires);
                        return Err(e);
                    }
                }
            }
        };
        if result.is_some() {
            self.counters.completed = self.counters.completed.saturating_add(1);
        }
        Ok(result)
    }
}
struct Received {
    value: Result<Option<Completed>, (Error, u64)>,
    evicted: Option<(Key, u64)>,
    duplicate: bool,
}
fn receive<const N: usize>(
    pool: &mut [Option<Group<N>>],
    per_link: usize,
    key: Key,
    fragment: Fragment<'_>,
    now: u64,
    output: &mut [u8],
) -> Received {
    let mut evicted = None;
    let index = if let Some(i) = pool
        .iter()
        .position(|g| g.as_ref().is_some_and(|g| g.key == key))
    {
        i
    } else {
        let at_link = pool
            .iter()
            .flatten()
            .filter(|g| g.key.generation == key.generation)
            .count();
        let i = if at_link >= per_link {
            pool.iter()
                .enumerate()
                .filter_map(|(i, g)| {
                    g.as_ref()
                        .filter(|g| g.key.generation == key.generation)
                        .map(|g| (i, g.started))
                })
                .min_by_key(|p| p.1)
                .map(|p| p.0)
        } else {
            pool.iter().position(|g| g.is_none()).or_else(|| {
                pool.iter()
                    .enumerate()
                    .filter_map(|(i, g)| g.as_ref().map(|g| (i, g.started)))
                    .min_by_key(|p| p.1)
                    .map(|p| p.0)
            })
        }
        .expect("nonempty fixed pool or link group exists");
        if let Some(old) = pool[i].take() {
            evicted = Some((old.key, old.expires));
        }
        pool[i] = Some(Group::new(key, fragment, now));
        i
    };
    let group = pool[index].as_mut().expect("inserted or matched slot");
    let expires = group.expires;
    let added = group.add(fragment);
    let mut duplicate = false;
    let value = match added {
        Ok(false) => {
            duplicate = true;
            Ok(None)
        }
        Ok(true) if group.complete() => group.assemble(output).map(Some),
        Ok(true) => Ok(None),
        Err(e) => Err(e),
    };
    if matches!(value, Ok(Some(_)) | Err(_)) {
        pool[index] = None;
    }
    Received {
        value: value.map_err(|e| (e, expires)),
        evicted,
        duplicate,
    }
}
