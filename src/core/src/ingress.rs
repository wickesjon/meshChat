//! MC-007 admission and bounded state before reassembly, logical work or crypto.
//!
//! Intake never authenticates: signed/encrypted inputs remain Pending until a
//! protocol owner (such as MC-019 Friends) completes reserved verification work.
//! Native drivers must check their 512-byte intake bound before allocating FFI arrays.
//! The borrowed API below copies only after all frame/byte buckets and staging caps pass.
use crate::{LinkHandle, codec, framing};
use sha2::{Digest, Sha256};
use std::mem::size_of;

const SCALE: u64 = 60_000;
const LAST_TIME: u64 = u64::MAX - 900_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("invalid configuration or capacity")]
    Configuration,
    #[error("unknown or stale connection/token")]
    Stale,
    #[error("monotonic time invalid")]
    Time,
    #[error("bounded resource exhausted")]
    Full,
    #[error("output buffer too small")]
    Output,
}

/// Never represents successful authentication or a UI/history admission decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Unverified,
    Opaque,
    Pending,
    PendingDuplicate,
    Duplicate,
    Control,
    /// Frame/reassembly admission only; MC-015 must validate the SYNC session
    /// and order before admitting its embedded logical CHAT.
    DeferredSync,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Drop {
    Budget,
    Size,
    Staging,
    Malformed,
    RejectedVariant,
    SenderFull,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Incomplete,
    Dropped(Drop),
    /// For SYNC transport, state describes the embedded CHAT; empty markers are Control.
    Complete {
        kind: framing::ObjectKind,
        len: usize,
        state: State,
    },
}

/// Single-use proof of outer admission. Private fields and no Clone/Copy keep
/// raw bytes from manufacturing or reusing admission. The borrowed output is
/// immutable until consumed, so the admitted body cannot be substituted.
///
/// ```compile_fail
/// use meshchat_core::ingress::SyncAdmission;
/// let forged = SyncAdmission {};
/// ```
/// ```compile_fail
/// use meshchat_core::ingress::SyncAdmission;
/// fn twice(token: SyncAdmission<'_>) {
///     let first = token;
///     let second = token;
/// }
/// ```
#[derive(Debug)]
pub struct SyncAdmission<'a> {
    link: LinkHandle,
    now: u64,
    kind: framing::ObjectKind,
    bytes: &'a [u8],
}
impl<'a> SyncAdmission<'a> {
    pub(crate) fn consume(
        self,
        link: &LinkHandle,
        now: u64,
        kind: framing::ObjectKind,
    ) -> Result<&'a [u8], Error> {
        if self.link != *link || self.now != now || self.kind != kind {
            return Err(Error::Stale);
        }
        Ok(self.bytes)
    }
}
#[derive(Debug)]
pub struct Admitted<'a> {
    pub outcome: Outcome,
    pub sync: Option<SyncAdmission<'a>>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Counters {
    pub offered_frames: u64,
    pub offered_bytes: u64,
    pub admitted_frames: u64,
    pub admitted_bytes: u64,
    pub budget_drops: u64,
    pub size_drops: u64,
    pub staging_drops: u64,
    pub malformed: u64,
    pub rejected_duplicates: u64,
    pub accepted_duplicates: u64,
    pub pending_duplicates: u64,
    pub sender_full: u64,
    pub unverified: u64,
    pub pending: u64,
    pub reserved_work_units: u64,
    pub connection_attempts: u64,
    pub peak_staging: usize,
    pub peak_pending: usize,
    pub peak_senders: usize,
    pub peak_accepted: usize,
    pub peak_rejected: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct Reservations {
    pub staging_bytes: usize,
    pub accepted_bytes: usize,
    pub rejected_bytes: usize,
    pub pending_bytes: usize,
    pub sender_bytes: usize,
    pub address_bytes: usize,
    pub manager_bytes: usize,
    pub reassembly: framing::Reservations,
    pub staging: usize,
    pub accepted: usize,
    pub rejected: usize,
    pub pending: usize,
    pub senders: usize,
    pub addresses: usize,
}

// Credits are measured in 1/60000 units. All MC-007 refill periods divide a minute.
#[derive(Debug, Clone, Copy)]
struct Bucket {
    credit: u64,
    at: u64,
    capacity: u32,
    per_minute: u32,
}
impl Bucket {
    fn new(capacity: u32, per_minute: u32, at: u64) -> Self {
        Self {
            credit: u64::from(capacity) * SCALE,
            at,
            capacity,
            per_minute,
        }
    }
    fn available(&mut self, now: u64, cost: u64) -> bool {
        let added = u128::from(now - self.at) * u128::from(self.per_minute);
        self.credit = (u128::from(self.credit) + added)
            .min(u128::from(self.capacity) * u128::from(SCALE)) as u64;
        self.at = now;
        u128::from(cost) * u128::from(SCALE) <= u128::from(self.credit)
    }
    fn debit(&mut self, cost: u64) {
        self.credit -= cost * SCALE;
    }
}
fn charge(now: u64, costs: &mut [(&mut Bucket, u64)]) -> bool {
    // Refill every bucket even on failure, but debit none unless all can pay.
    let mut allowed = true;
    for (bucket, cost) in costs.iter_mut() {
        allowed &= bucket.available(now, *cost);
    }
    if allowed {
        for (bucket, cost) in costs {
            bucket.debit(*cost);
        }
    }
    allowed
}
fn pool<T>(len: usize) -> Vec<Option<T>> {
    std::iter::repeat_with(|| None).take(len).collect()
}
fn count<T>(pool: &[Option<T>]) -> usize {
    pool.iter().filter(|x| x.is_some()).count()
}
fn reserved<T>(pool: &Vec<Option<T>>) -> usize {
    pool.capacity() * size_of::<Option<T>>() + size_of::<Vec<Option<T>>>()
}
fn digest(body: &[u8], logical: bool) -> [u8; 32] {
    let mut hash = Sha256::new();
    if logical {
        // TTL is mutable; every other immutable byte participates. Never dedup by claimed ID.
        hash.update(&body[..3]);
        hash.update(&body[4..]);
    } else {
        hash.update(body);
    }
    hash.finalize().into()
}

struct Link {
    generation: u64,
    capacity: usize,
    partition: usize,
    frames: Bucket,
    bytes: Bucket,
    unknown: Bucket,
    controls: [Bucket; 4],
    sessions: [Bucket; 2],
    work: Bucket,
}
struct Partition {
    generation: u64,
    retired: Option<u64>,
}
struct Accepted {
    hash: [u8; 32],
    expires: u64,
    arrived: u64,
}
struct Rejected {
    generation: u64,
    hash: [u8; 32],
    expires: u64,
}
struct Pending {
    generation: u64,
    hash: [u8; 32],
    expires: u64,
    len: u16,
    bytes: [u8; 1024],
}
struct Sender {
    id: [u8; 8],
    seen: u64,
    buckets: [Bucket; 6],
}
struct Address {
    id: [u8; 16],
    seen: u64,
    bucket: Bucket,
}
struct Staged {
    sequence: u64,
    generation: u64,
    len: u16,
    bytes: [u8; 512],
}
struct Work {
    sequence: u64,
    generation: u64,
    signature_hash: Option<[u8; 32]>,
    expires: u64,
}

/// Scoped, non-forgeable handle to already charged staged input; replay is rejected.
#[derive(Debug, Clone, Copy)]
pub struct Intake {
    instance: u64,
    sequence: u64,
}
/// Work reservation grants units and a concurrency slot, never trust. No cloning.
#[derive(Debug)]
pub struct WorkPermit {
    instance: u64,
    sequence: u64,
}

pub struct Ingress {
    instance: u64,
    now: u64,
    sequence: u64,
    highest_generation: u64,
    max_links: usize,
    public_channels: [[u8; 4]; 2],
    links: [Option<Link>; 8],
    partitions: [Option<Partition>; 8],
    staging: Vec<Option<Staged>>,
    accepted: Vec<Option<Accepted>>,
    rejected: Vec<Option<Rejected>>,
    pending: Vec<Option<Pending>>,
    senders: Vec<Option<Sender>>,
    addresses: Vec<Option<Address>>,
    work: [Option<Work>; 2],
    frames: Bucket,
    bytes: Bucket,
    unknown: Bucket,
    sessions: [Bucket; 2],
    crypto: Bucket,
    connections: Bucket,
    reassembly: framing::Reassembler,
    counters: Counters,
}

impl Ingress {
    /// max_links is the current platform/mode ceiling, never greater than eight.
    pub fn new(instance: u64, max_links: usize, now: u64) -> Result<Self, Error> {
        if instance == 0 || !(1..=8).contains(&max_links) || now > LAST_TIME {
            return Err(Error::Configuration);
        }
        let value = Self {
            instance,
            now,
            sequence: 0,
            highest_generation: 0,
            max_links,
            public_channels: [
                crate::channel::Public::General,
                crate::channel::Public::Confessions,
            ]
            .map(|p| crate::channel::Channel::Public(p).id()),
            links: std::array::from_fn(|_| None),
            partitions: std::array::from_fn(|_| None),
            staging: pool(16),
            accepted: pool(4096),
            rejected: pool(512),
            pending: pool(32),
            senders: pool(4096),
            addresses: pool(256),
            work: [None, None],
            frames: Bucket::new(60, 480, now),
            bytes: Bucket::new(64 * 1024, 8 * 1024 * 60, now),
            unknown: Bucket::new(20, 60, now),
            sessions: [Bucket::new(2, 2, now); 2],
            crypto: Bucket::new(40, 2400, now),
            connections: Bucket::new(6, 6, now),
            reassembly: framing::Reassembler::new(instance, 8, now)
                .map_err(|_| Error::Configuration)?,
            counters: Counters::default(),
        };
        let r = value.reservations();
        if r.staging_bytes > 16 * 1024
            || r.accepted_bytes > 384 * 1024
            || r.rejected_bytes > 64 * 1024
            || r.pending_bytes > 48 * 1024
            || r.sender_bytes > 1024 * 1024
            || r.address_bytes > 32 * 1024
            || 8 * size_of::<Option<Pending>>() > 12 * 1024
        {
            return Err(Error::Configuration);
        }
        Ok(value)
    }
    fn link(&self, handle: &LinkHandle) -> Result<usize, Error> {
        if handle.instance_nonce != self.instance {
            return Err(Error::Stale);
        }
        self.links
            .iter()
            .position(|l| {
                l.as_ref()
                    .is_some_and(|l| l.generation == handle.generation)
            })
            .ok_or(Error::Stale)
    }
    fn next(&mut self) -> Result<u64, Error> {
        self.sequence = self.sequence.checked_add(1).ok_or(Error::Full)?;
        Ok(self.sequence)
    }
    fn clock(&mut self, now: u64) -> Result<(), Error> {
        if now < self.now || now > LAST_TIME {
            return Err(Error::Time);
        }
        self.now = now;
        Ok(())
    }
    pub fn advance(&mut self, now: u64) -> Result<(), Error> {
        self.clock(now)?;
        self.reassembly.advance(now).map_err(|_| Error::Time)?;
        for item in &mut self.accepted {
            if item.as_ref().is_some_and(|x| x.expires <= now) {
                *item = None;
            }
        }
        for item in &mut self.rejected {
            if item.as_ref().is_some_and(|x| x.expires <= now) {
                *item = None;
            }
        }
        for item in &mut self.pending {
            if item.as_ref().is_some_and(|x| x.expires <= now) {
                *item = None;
            }
        }
        for item in &mut self.work {
            if item.as_ref().is_some_and(|x| x.expires <= now) {
                *item = None;
            }
        }
        for item in &mut self.senders {
            if item.as_ref().is_some_and(|x| now - x.seen >= 300_000) {
                *item = None;
            }
        }
        for item in &mut self.addresses {
            if item.as_ref().is_some_and(|x| now - x.seen >= 300_000) {
                *item = None;
            }
        }
        Ok(())
    }
    /// Driver connection-attempt admission. A rotated native address still consumes node credit.
    pub fn connection_attempt(&mut self, address: [u8; 16], now: u64) -> Result<bool, Error> {
        self.advance(now)?;
        let index = self
            .addresses
            .iter()
            .position(|x| x.as_ref().is_some_and(|x| x.id == address))
            .or_else(|| self.addresses.iter().position(Option::is_none));
        let Some(index) = index else {
            return Ok(false);
        };
        // No allocation/table entry is created if the node cannot pay.
        if !self.connections.available(now, 1) {
            return Ok(false);
        }
        let item = self.addresses[index].get_or_insert_with(|| Address {
            id: address,
            seen: now,
            bucket: Bucket::new(3, 3, now),
        });
        item.seen = now;
        let ok = charge(
            now,
            &mut [(&mut self.connections, 1), (&mut item.bucket, 1)],
        );
        if ok {
            self.counters.connection_attempts = self.counters.connection_attempts.saturating_add(1);
        }
        Ok(ok)
    }
    /// An established/admitted link, including handshaking links. No node-credit reset.
    pub fn register(
        &mut self,
        handle: &LinkHandle,
        capacity: usize,
        now: u64,
    ) -> Result<(), Error> {
        self.advance(now)?;
        if handle.instance_nonce != self.instance || handle.generation <= self.highest_generation {
            return Err(Error::Stale);
        }
        if count(&self.links) >= self.max_links {
            return Err(Error::Full);
        }
        if !(framing::MIN_CAPACITY..=framing::MAX_CAPACITY).contains(&capacity) {
            return Err(Error::Configuration);
        }
        let partition = self
            .partitions
            .iter()
            .position(Option::is_none)
            .or_else(|| {
                self.partitions
                    .iter()
                    .enumerate()
                    .filter_map(|(i, p)| p.as_ref()?.retired.map(|at| (i, at)))
                    .min_by_key(|(_, at)| *at)
                    .map(|(i, _)| i)
            })
            .ok_or(Error::Full)?;
        self.reassembly
            .register(handle, capacity)
            .map_err(|_| Error::Configuration)?;
        for entry in &mut self.accepted[partition * 512..(partition + 1) * 512] {
            *entry = None;
        }
        self.partitions[partition] = Some(Partition {
            generation: handle.generation,
            retired: None,
        });
        let free = self
            .links
            .iter_mut()
            .find(|x| x.is_none())
            .ok_or(Error::Full)?;
        *free = Some(Link {
            generation: handle.generation,
            capacity,
            partition,
            frames: Bucket::new(15, 60, now),
            bytes: Bucket::new(24 * 1024, 2 * 1024 * 60, now),
            unknown: Bucket::new(5, 12, now),
            controls: [
                Bucket::new(2, 2, now),
                Bucket::new(1, 12, now),
                Bucket::new(2, 2, now),
                Bucket::new(2, 2, now),
            ],
            sessions: [Bucket::new(1, 1, now); 2],
            work: Bucket::new(20, 1200, now),
        });
        self.highest_generation = handle.generation;
        Ok(())
    }
    pub fn disconnect(&mut self, handle: &LinkHandle, now: u64) -> Result<(), Error> {
        self.advance(now)?;
        let index = self.link(handle)?;
        let link = self.links[index].take().ok_or(Error::Stale)?;
        self.partitions[link.partition]
            .as_mut()
            .ok_or(Error::Stale)?
            .retired = Some(now);
        for item in &mut self.staging {
            if item
                .as_ref()
                .is_some_and(|x| x.generation == handle.generation)
            {
                *item = None;
            }
        }
        for item in &mut self.rejected {
            if item
                .as_ref()
                .is_some_and(|x| x.generation == handle.generation)
            {
                *item = None;
            }
        }
        for item in &mut self.pending {
            if item
                .as_ref()
                .is_some_and(|x| x.generation == handle.generation)
            {
                *item = None;
            }
        }
        self.reassembly.disconnect(handle).map_err(|_| Error::Stale)
    }
    /// Caller tears down excess links explicitly before lowering the ceiling; no implicit credit grant.
    pub fn set_link_limit(&mut self, maximum: usize) -> Result<(), Error> {
        if !(1..=8).contains(&maximum) || count(&self.links) > maximum {
            return Err(Error::Configuration);
        }
        self.max_links = maximum;
        Ok(())
    }
    /// Any changed native receive capacity invalidates the transfer generation,
    /// including an increase. Re-admit a fresh generation; node credit is retained.
    pub fn capacity_changed(
        &mut self,
        handle: &LinkHandle,
        observed: usize,
        now: u64,
    ) -> Result<bool, Error> {
        self.advance(now)?;
        let index = self.link(handle)?;
        if self.links[index].as_ref().unwrap().capacity == observed {
            return Ok(false);
        }
        self.disconnect(handle, now)?;
        Ok(true)
    }
    /// Reserve a new locally requested session. Incoming cursor-zero requests charge serving
    /// admission automatically; the future session consumer must validate continuations.
    pub fn new_requesting_session(&mut self, handle: &LinkHandle, now: u64) -> Result<bool, Error> {
        self.advance(now)?;
        let index = self.link(handle)?;
        let role = 0;
        Ok(charge(
            now,
            &mut [
                (&mut self.sessions[role], 1),
                (&mut self.links[index].as_mut().unwrap().sessions[role], 1),
            ],
        ))
    }
    /// Reserve a complete known crypto bundle before any operation. No refund on failure.
    pub fn begin_work(
        &mut self,
        handle: &LinkHandle,
        units: u8,
        now: u64,
    ) -> Result<Option<WorkPermit>, Error> {
        self.advance(now)?;
        let index = self.link(handle)?;
        if units == 0 || units > 20 {
            return Err(Error::Configuration);
        }
        let Some(slot) = self.work.iter().position(Option::is_none) else {
            return Ok(None);
        };
        if !charge(
            now,
            &mut [
                (&mut self.crypto, u64::from(units)),
                (
                    &mut self.links[index].as_mut().unwrap().work,
                    u64::from(units),
                ),
            ],
        ) {
            return Ok(None);
        }
        let sequence = self.next()?;
        self.work[slot] = Some(Work {
            sequence,
            generation: handle.generation,
            signature_hash: None,
            expires: now + 30_000,
        });
        self.counters.reserved_work_units = self
            .counters
            .reserved_work_units
            .saturating_add(u64::from(units));
        Ok(Some(WorkPermit {
            instance: self.instance,
            sequence,
        }))
    }
    /// Explicit local QR/import work shares the node bucket and both job slots.
    /// Generation zero is reserved here; transport registration forbids it.
    pub fn begin_local_work(&mut self, units: u8, now: u64) -> Result<Option<WorkPermit>, Error> {
        self.advance(now)?;
        if units == 0 || units > 20 {
            return Err(Error::Configuration);
        }
        let Some(slot) = self.work.iter().position(Option::is_none) else {
            return Ok(None);
        };
        if !charge(now, &mut [(&mut self.crypto, u64::from(units))]) {
            return Ok(None);
        }
        let sequence = self.next()?;
        self.work[slot] = Some(Work {
            sequence,
            generation: 0,
            signature_hash: None,
            expires: now + 30_000,
        });
        self.counters.reserved_work_units = self
            .counters
            .reserved_work_units
            .saturating_add(u64::from(units));
        Ok(Some(WorkPermit {
            instance: self.instance,
            sequence,
        }))
    }
    pub fn finish_work(&mut self, permit: WorkPermit) -> Result<(), Error> {
        if permit.instance != self.instance {
            return Err(Error::Stale);
        }
        let slot = self
            .work
            .iter_mut()
            .find(|x| x.as_ref().is_some_and(|x| x.sequence == permit.sequence))
            .ok_or(Error::Stale)?;
        let generation = slot.take().unwrap().generation;
        if generation == 0
            || self
                .links
                .iter()
                .flatten()
                .any(|x| x.generation == generation)
        {
            Ok(())
        } else {
            Err(Error::Stale)
        }
    }
    /// Only a verifier inside the core can claim charged, still-pending bytes.
    /// Identical concurrent arrivals share one attempt; missing-key/budget cases
    /// retain the original bounded pending record for a later retry.
    pub(crate) fn begin_signature(
        &mut self,
        handle: &LinkHandle,
        body: &[u8],
        now: u64,
    ) -> Result<Option<WorkPermit>, Error> {
        self.begin_authentication(handle, body, 1, now)
    }
    pub(crate) fn pending_deadline(&self, handle: &LinkHandle, body: &[u8]) -> Option<u64> {
        let hash = digest(body, true);
        self.pending
            .iter()
            .flatten()
            .find(|p| p.generation == handle.generation && p.hash == hash)
            .map(|p| p.expires)
    }
    pub(crate) fn begin_authentication(
        &mut self,
        handle: &LinkHandle,
        body: &[u8],
        units: u8,
        now: u64,
    ) -> Result<Option<WorkPermit>, Error> {
        self.advance(now)?;
        self.link(handle)?;
        if body.len() < codec::HEADER_LEN {
            return Err(Error::Stale);
        }
        let hash = digest(body, true);
        if !self
            .pending
            .iter()
            .flatten()
            .any(|p| p.generation == handle.generation && p.hash == hash)
            || self
                .work
                .iter()
                .flatten()
                .any(|w| w.signature_hash == Some(hash))
        {
            return Ok(None);
        }
        let permit = self.begin_work(handle, units, now)?;
        if let Some(permit) = &permit {
            let expires = self
                .pending
                .iter()
                .flatten()
                .find(|p| p.generation == handle.generation && p.hash == hash)
                .ok_or(Error::Stale)?
                .expires;
            let work = self
                .work
                .iter_mut()
                .flatten()
                .find(|w| w.sequence == permit.sequence)
                .ok_or(Error::Stale)?;
            work.signature_hash = Some(hash);
            work.expires = expires;
        }
        Ok(permit)
    }
    pub(crate) fn resolve_signature(
        &mut self,
        handle: &LinkHandle,
        body: &[u8],
        accepted: bool,
    ) -> Result<(), Error> {
        let index = self.link(handle)?;
        let hash = digest(body, true);
        for item in &mut self.pending {
            if item.as_ref().is_some_and(|p| p.hash == hash) {
                *item = None;
            }
        }
        if !accepted {
            self.reject(handle.generation, hash);
            return Ok(());
        }
        let partition = self.links[index].as_ref().unwrap().partition;
        let entries = &mut self.accepted[partition * 512..(partition + 1) * 512];
        let slot = entries.iter().position(Option::is_none).unwrap_or_else(|| {
            entries
                .iter()
                .enumerate()
                .min_by_key(|(_, x)| x.as_ref().unwrap().arrived)
                .unwrap()
                .0
        });
        entries[slot] = Some(Accepted {
            hash,
            expires: self.now + 900_000,
            arrived: self.now,
        });
        self.counters.peak_accepted = self.counters.peak_accepted.max(count(&self.accepted));
        Ok(())
    }
    /// Budget the reported native size before checking/copying the supplied bounded slice.
    pub fn enqueue(
        &mut self,
        handle: &LinkHandle,
        reported: u64,
        raw: &[u8],
        now: u64,
    ) -> Result<Result<Intake, Drop>, Error> {
        self.clock(now)?;
        let index = self.link(handle)?;
        let charged_bytes = reported.max(raw.len() as u64);
        self.counters.offered_frames = self.counters.offered_frames.saturating_add(1);
        self.counters.offered_bytes = self.counters.offered_bytes.saturating_add(charged_bytes);
        let link = self.links[index].as_mut().unwrap();
        if !charge(
            now,
            &mut [
                (&mut self.frames, 1),
                (&mut self.bytes, charged_bytes),
                (&mut link.frames, 1),
                (&mut link.bytes, charged_bytes),
            ],
        ) {
            self.counters.budget_drops = self.counters.budget_drops.saturating_add(1);
            return Ok(Err(Drop::Budget));
        }
        if reported == 0 || reported > 512 || reported != raw.len() as u64 {
            self.counters.size_drops = self.counters.size_drops.saturating_add(1);
            return Ok(Err(Drop::Size));
        }
        let free = self.staging.iter().position(Option::is_none);
        if free.is_none()
            || self
                .staging
                .iter()
                .flatten()
                .filter(|x| x.generation == handle.generation)
                .count()
                >= 2
        {
            self.counters.staging_drops = self.counters.staging_drops.saturating_add(1);
            return Ok(Err(Drop::Staging));
        }
        let sequence = self.next()?;
        let mut staged = Staged {
            sequence,
            generation: handle.generation,
            len: raw.len() as u16,
            bytes: [0; 512],
        };
        staged.bytes[..raw.len()].copy_from_slice(raw);
        self.staging[free.unwrap()] = Some(staged);
        self.counters.admitted_frames = self.counters.admitted_frames.saturating_add(1);
        self.counters.admitted_bytes = self.counters.admitted_bytes.saturating_add(reported);
        self.counters.peak_staging = self.counters.peak_staging.max(count(&self.staging));
        Ok(Ok(Intake {
            instance: self.instance,
            sequence,
        }))
    }
    pub fn receive(
        &mut self,
        handle: &LinkHandle,
        reported: u64,
        raw: &[u8],
        now: u64,
        output: &mut [u8],
    ) -> Result<Outcome, Error> {
        // Caller buffer errors cannot strand an admitted staging reservation.
        if output.len() < 1035 {
            return Err(Error::Output);
        }
        match self.enqueue(handle, reported, raw, now)? {
            Ok(token) => self.process(token, now, output),
            Err(reason) => Ok(Outcome::Dropped(reason)),
        }
    }
    /// MC-015 receive path: correlate and order a SYNC object before the inner
    /// packet can affect logical dedup, sender budgets or pending crypto state.
    /// All outer frame/byte charges and reassembly limits are unchanged.
    pub fn receive_deferred_sync<'a>(
        &mut self,
        handle: &LinkHandle,
        reported: u64,
        raw: &[u8],
        now: u64,
        output: &'a mut [u8],
    ) -> Result<Admitted<'a>, Error> {
        if output.len() < 1035 {
            return Err(Error::Output);
        }
        let outcome = match self.enqueue(handle, reported, raw, now)? {
            Ok(token) => self.process_inner(token, now, output, true, None)?,
            Err(reason) => Outcome::Dropped(reason),
        };
        let sync = match outcome {
            Outcome::Complete { kind, len, .. }
                if kind == framing::ObjectKind::Transport(1)
                    || (kind == framing::ObjectKind::Logical && output[1] == 3) =>
            {
                Some(SyncAdmission {
                    link: handle.clone(),
                    now,
                    kind,
                    bytes: &output[..len],
                })
            }
            _ => None,
        };
        Ok(Admitted { outcome, sync })
    }
    /// Called once in sequence by MC-015 after deferred outer admission and an
    /// active-session check. No native frame is charged a second time. Ordinary
    /// sender/dedup/pending gates still apply, and no authenticated state exists.
    pub(crate) fn admit_stored_sync(
        &mut self,
        handle: &LinkHandle,
        blob: &[u8],
        now: u64,
    ) -> Result<Result<State, Drop>, Error> {
        self.advance(now)?;
        let index = self.link(handle)?;
        Ok(self.logical(index, blob, codec::Context::StoredChat))
    }
    pub fn process(
        &mut self,
        token: Intake,
        now: u64,
        output: &mut [u8],
    ) -> Result<Outcome, Error> {
        self.process_inner(token, now, output, false, None)
    }
    pub(crate) fn receive_friend(
        &mut self,
        handle: &LinkHandle,
        input: (u64, &[u8]),
        now: u64,
        output: &mut [u8],
        gate: (bool, usize),
    ) -> Result<Outcome, Error> {
        if output.len() < 1035 {
            return Err(Error::Output);
        }
        match self.enqueue(handle, input.0, input.1, now)? {
            Ok(token) => self.process_inner(token, now, output, true, Some(gate)),
            Err(reason) => Ok(Outcome::Dropped(reason)),
        }
    }
    fn process_inner(
        &mut self,
        token: Intake,
        now: u64,
        output: &mut [u8],
        defer_sync: bool,
        friend_gate: Option<(bool, usize)>,
    ) -> Result<Outcome, Error> {
        if output.len() < 1035 {
            return Err(Error::Output);
        }
        self.advance(now)?;
        if token.instance != self.instance {
            return Err(Error::Stale);
        }
        let slot = self
            .staging
            .iter_mut()
            .find(|x| x.as_ref().is_some_and(|x| x.sequence == token.sequence))
            .ok_or(Error::Stale)?;
        let staged = slot.take().ok_or(Error::Stale)?;
        let handle = LinkHandle {
            instance_nonce: self.instance,
            generation: staged.generation,
        };
        let index = self.link(&handle)?;
        let raw = &staged.bytes[..usize::from(staged.len)];
        if let Some((hello_only, receive_limit)) = friend_gate {
            if raw.len() > receive_limit {
                return Ok(Outcome::Dropped(Drop::Size));
            }
            if hello_only
                && !matches!(
                    framing::parse_frame(raw, receive_limit),
                    Ok(framing::Frame::WholeTransport { kind: 2, .. })
                )
            {
                return Ok(Outcome::Dropped(Drop::Malformed));
            }
        }
        let hash = digest(raw, false);
        if self
            .rejected
            .iter()
            .flatten()
            .any(|x| x.generation == staged.generation && x.hash == hash)
        {
            self.counters.rejected_duplicates = self.counters.rejected_duplicates.saturating_add(1);
            return Ok(Outcome::Dropped(Drop::RejectedVariant));
        }
        let completed = match self.reassembly.ingest_admitted(&handle, raw, now, output) {
            Err(_) => {
                // Known fragment envelopes use the reassembler's original group deadline.
                if !matches!(raw.first(), Some(1 | 3)) {
                    self.reject(staged.generation, hash);
                }
                self.counters.malformed = self.counters.malformed.saturating_add(1);
                return Ok(Outcome::Dropped(Drop::Malformed));
            }
            Ok(None) => return Ok(Outcome::Incomplete),
            Ok(Some(done)) => done,
        };
        let body = &output[..completed.len];
        let state = match completed.kind {
            framing::ObjectKind::Logical => self.logical(index, body, codec::Context::Live),
            framing::ObjectKind::Transport(1) if defer_sync => Ok(State::DeferredSync),
            framing::ObjectKind::Transport(1) => match framing::transport(1, body) {
                Ok(framing::Transport::SyncItem { blob, .. }) if !blob.is_empty() => {
                    self.logical(index, blob, codec::Context::StoredChat)
                }
                Ok(_) => Ok(State::Control),
                Err(_) => Err(Drop::Malformed),
            },
            framing::ObjectKind::Transport(3) => Ok(State::Pending),
            framing::ObjectKind::Transport(_) => Ok(State::Control),
        };
        match state {
            Ok(state) => Ok(Outcome::Complete {
                kind: completed.kind,
                len: completed.len,
                state,
            }),
            Err(reason) => Ok(Outcome::Dropped(reason)),
        }
    }
    fn reject(&mut self, generation: u64, hash: [u8; 32]) {
        let matching = self.rejected.iter().enumerate().filter_map(|(i, x)| {
            x.as_ref()
                .filter(|x| x.generation == generation)
                .map(|x| (i, x.expires))
        });
        let slot = if matching.clone().count() >= 64 {
            matching.min_by_key(|x| x.1).unwrap().0
        } else {
            self.rejected
                .iter()
                .position(Option::is_none)
                .unwrap_or_else(|| {
                    self.rejected
                        .iter()
                        .enumerate()
                        .min_by_key(|(_, x)| x.as_ref().unwrap().expires)
                        .unwrap()
                        .0
                })
        };
        self.rejected[slot] = Some(Rejected {
            generation,
            hash,
            expires: self.now + 30_000,
        });
        self.counters.peak_rejected = self.counters.peak_rejected.max(count(&self.rejected));
    }
    fn logical(
        &mut self,
        index: usize,
        body: &[u8],
        context: codec::Context,
    ) -> Result<State, Drop> {
        let packet = codec::parse(body, context).map_err(|_| Drop::Malformed)?;
        let header = packet.header();
        let hash = digest(body, true);
        if self.rejected.iter().flatten().any(|x| {
            x.generation == self.links[index].as_ref().unwrap().generation && x.hash == hash
        }) {
            self.counters.rejected_duplicates = self.counters.rejected_duplicates.saturating_add(1);
            return Err(Drop::RejectedVariant);
        }
        if self.accepted.iter().flatten().any(|x| x.hash == hash) {
            self.counters.accepted_duplicates = self.counters.accepted_duplicates.saturating_add(1);
            return Ok(State::Duplicate);
        }
        if self.pending.iter().flatten().any(|x| x.hash == hash) {
            self.counters.pending_duplicates = self.counters.pending_duplicates.saturating_add(1);
            return Ok(State::PendingDuplicate);
        }
        let link = self.links[index].as_mut().unwrap();
        let unknown = matches!(packet.payload(), codec::Payload::Unknown(_));
        let control = match header.kind {
            2 => Some(0),
            7 => Some(1),
            5 => Some(2),
            8 => Some(3),
            _ => None,
        };
        let allowed = if unknown {
            charge(
                self.now,
                &mut [(&mut self.unknown, 1), (&mut link.unknown, 1)],
            )
        } else if let Some(which) = control {
            charge(self.now, &mut [(&mut link.controls[which], 1)])
        } else if matches!(
            packet.payload(),
            codec::Payload::SyncRequest { cursor: 0, .. }
        ) {
            charge(
                self.now,
                &mut [(&mut self.sessions[1], 1), (&mut link.sessions[1], 1)],
            )
        } else {
            true
        };
        if !allowed {
            self.counters.budget_drops = self.counters.budget_drops.saturating_add(1);
            return Err(Drop::Budget);
        }
        if header.kind == 1 || header.kind == 6 {
            let sender = self
                .senders
                .iter()
                .position(|x| x.as_ref().is_some_and(|x| x.id == header.sender_id))
                .or_else(|| self.senders.iter().position(Option::is_none));
            let Some(sender) = sender else {
                self.counters.sender_full = self.counters.sender_full.saturating_add(1);
                return Err(Drop::SenderFull);
            };
            let item = self.senders[sender].get_or_insert_with(|| Sender {
                id: header.sender_id,
                seen: self.now,
                buckets: [(5, 5), (2, 2), (10, 10), (15, 15), (20, 20), (30, 30)]
                    .map(|(capacity, rate)| Bucket::new(capacity, rate, self.now)),
            });
            item.seen = self.now;
            let class = if header.kind == 6 {
                5
            } else if header.channel_id == codec::EVENT_CHANNEL {
                if header.flags & 7 == 6 { 2 } else { 1 }
            } else if self.public_channels.contains(&header.channel_id) {
                0
            } else {
                3
            };
            let allowed = if class == 5 {
                charge(self.now, &mut [(&mut item.buckets[5], 1)])
            } else {
                let (classes, rest) = item.buckets.split_at_mut(4);
                charge(self.now, &mut [(&mut classes[class], 1), (&mut rest[0], 1)])
            };
            self.counters.peak_senders = self.counters.peak_senders.max(count(&self.senders));
            if !allowed {
                self.counters.budget_drops = self.counters.budget_drops.saturating_add(1);
                return Err(Drop::Budget);
            }
        }
        let pending = !unknown && (header.flags & 7 != 0 || header.kind == 8);
        let generation = self.links[index].as_ref().unwrap().generation;
        if pending {
            let matching = self.pending.iter().enumerate().filter_map(|(i, x)| {
                x.as_ref()
                    .filter(|x| x.generation == generation)
                    .map(|x| (i, x.expires))
            });
            let slot = if matching.clone().count() >= 8 {
                matching.min_by_key(|x| x.1).unwrap().0
            } else {
                self.pending
                    .iter()
                    .position(Option::is_none)
                    .unwrap_or_else(|| {
                        self.pending
                            .iter()
                            .enumerate()
                            .min_by_key(|(_, x)| x.as_ref().unwrap().expires)
                            .unwrap()
                            .0
                    })
            };
            let mut item = Pending {
                generation,
                hash,
                expires: self.now + 30_000,
                len: body.len() as u16,
                bytes: [0; 1024],
            };
            item.bytes[..body.len()].copy_from_slice(body);
            self.pending[slot] = Some(item);
            self.counters.pending = self.counters.pending.saturating_add(1);
            self.counters.peak_pending = self.counters.peak_pending.max(count(&self.pending));
            return Ok(State::Pending);
        }
        // Clear unsigned content must satisfy text rejection policy before any
        // accepted effect/dedup. Signed text is checked by its authenticating
        // owner; encrypted and unknown payloads remain opaque at this layer.
        crate::text::validate_payload(packet.payload()).map_err(|_| Drop::Malformed)?;
        // Only accepted unverified/opaque data here; never a trusted partition.
        let partition = self.links[index].as_ref().unwrap().partition;
        let entries = &mut self.accepted[partition * 512..(partition + 1) * 512];
        let slot = entries.iter().position(Option::is_none).unwrap_or_else(|| {
            entries
                .iter()
                .enumerate()
                .min_by_key(|(_, x)| x.as_ref().unwrap().arrived)
                .unwrap()
                .0
        });
        entries[slot] = Some(Accepted {
            hash,
            expires: self.now + 900_000,
            arrived: self.now,
        });
        self.counters.unverified = self.counters.unverified.saturating_add(1);
        self.counters.peak_accepted = self.counters.peak_accepted.max(count(&self.accepted));
        Ok(if unknown {
            State::Opaque
        } else {
            State::Unverified
        })
    }
    /// Inspect one pending record for a later verifier without creating an authenticated result.
    pub fn pending_bytes(&self, position: usize) -> Option<&[u8]> {
        let item = self.pending.get(position)?.as_ref()?;
        Some(&item.bytes[..usize::from(item.len)])
    }
    pub fn counters(&self) -> Counters {
        self.counters
    }
    pub fn reassembly_counters(&self) -> framing::Counters {
        self.reassembly.counters()
    }
    pub fn reservations(&self) -> Reservations {
        Reservations {
            staging_bytes: reserved(&self.staging),
            accepted_bytes: reserved(&self.accepted),
            rejected_bytes: reserved(&self.rejected),
            pending_bytes: reserved(&self.pending),
            sender_bytes: reserved(&self.senders),
            address_bytes: reserved(&self.addresses),
            manager_bytes: size_of::<Self>()
                - size_of::<framing::Reassembler>()
                - 6 * size_of::<Vec<()>>(),
            reassembly: self.reassembly.reservations(),
            staging: count(&self.staging),
            accepted: count(&self.accepted),
            rejected: count(&self.rejected),
            pending: count(&self.pending),
            senders: count(&self.senders),
            addresses: count(&self.addresses),
        }
    }
    pub fn partition_generation(&self, partition: usize) -> Option<u64> {
        self.partitions
            .get(partition)?
            .as_ref()
            .map(|x| x.generation)
    }
}
