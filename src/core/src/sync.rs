//! Bounded forward cache and SYNC state. No authenticated state is invented here.
pub mod session;
use crate::{
    codec::{self, Context},
    power::Mode,
};
use sha2::{Digest, Sha256};
use std::mem::size_of;
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("invalid object or context")]
    Invalid,
    #[error("invalid monotonic time")]
    Time,
    #[error("bounded resource exhausted")]
    Full,
    #[error("sequence exhausted")]
    Sequence,
    #[error("expired or evicted cache object")]
    Missing,
}
/// This milestone has no production cryptographic verifier. Cache membership
/// preserves the distinction between clear unverified and opaque pending data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheState {
    Unverified,
    Pending,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Insert {
    Added(u64),
    Existing(u64),
    Expired,
}
struct Record {
    hash: [u8; 32],
    message: [u8; 8],
    sequence: u64,
    arrival: u64,
    deadline: u64,
    touched: u64,
    offset: usize,
    len: usize,
    state: CacheState,
}
#[derive(Debug, Clone, Copy)]
pub struct CacheReservations {
    pub entries: usize,
    pub encoded_bytes: usize,
    pub allocated_bytes: usize,
    pub peak_encoded: usize,
    pub peak_entries: usize,
}
pub struct Cache {
    mode: Mode,
    now: u64,
    sequence: u64,
    records: Vec<Option<Record>>,
    arena: Vec<u8>,
    used: usize,
    peak_encoded: usize,
    peak_entries: usize,
}
fn pool<T>(count: usize) -> Vec<Option<T>> {
    std::iter::repeat_with(|| None).take(count).collect()
}
fn hash(body: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(&body[..3]);
    h.update(&body[4..]);
    h.finalize().into()
}
fn limits(mode: Mode) -> (usize, usize, u64) {
    if mode == Mode::Beacon {
        (5000, 5 * 1024 * 1024, 3_600_000)
    } else {
        (500, 256 * 1024, 900_000)
    }
}
fn bloom_positions(id: &[u8; 8]) -> [usize; 6] {
    let mut h = Sha256::new();
    h.update(b"meshfest-bloom-v1");
    h.update(id);
    let h = h.finalize();
    let a = u64::from_be_bytes(h[..8].try_into().unwrap());
    let b = u64::from_be_bytes(h[8..16].try_into().unwrap());
    std::array::from_fn(|i| ((u128::from(a) + i as u128 * u128::from(b)) % 4096) as usize)
}
pub fn bloom_contains(filter: &[u8; 512], id: &[u8; 8]) -> bool {
    bloom_positions(id)
        .iter()
        .all(|&p| filter[p / 8] & (0x80 >> (p % 8)) != 0)
}
impl Cache {
    pub fn new(mode: Mode, now: u64) -> Result<Self, Error> {
        if now > u64::MAX - 3_600_000 {
            return Err(Error::Time);
        }
        let (entries, bytes, _) = limits(mode);
        Ok(Self {
            mode,
            now,
            sequence: 0,
            records: pool(entries),
            arena: vec![0; bytes],
            used: 0,
            peak_encoded: 0,
            peak_entries: 0,
        })
    }
    fn time(&mut self, now: u64) -> Result<(), Error> {
        if now < self.now || now > u64::MAX - 3_600_000 {
            return Err(Error::Time);
        }
        self.now = now;
        Ok(())
    }
    pub fn advance(&mut self, now: u64) -> Result<(), Error> {
        self.time(now)?;
        let mut changed = false;
        for r in self.records.iter_mut().flatten() {
            if r.len > 0 && r.deadline <= now {
                changed = true;
                self.arena[r.offset..r.offset + r.len].fill(0);
                r.len = 0;
            }
        }
        // Retain bounded metadata for expired records until ordinary LRU pressure
        // replaces it. Re-access/replay cannot resurrect a tracked expired record.
        if changed {
            self.compact();
        }
        Ok(())
    }
    fn compact(&mut self) {
        // Sequence references, not slot offsets, escape this cache. In-place
        // unstable sorting needs no auxiliary heap/index/payload allocation.
        self.records.sort_unstable_by_key(|r| {
            r.as_ref()
                .map(|r| (r.len == 0, r.offset))
                .unwrap_or((true, usize::MAX))
        });
        let mut write = 0;
        for r in self.records.iter_mut().flatten().filter(|r| r.len > 0) {
            self.arena.copy_within(r.offset..r.offset + r.len, write);
            r.offset = write;
            write += r.len;
        }
        if write < self.used {
            self.arena[write..self.used].fill(0);
        }
        self.used = write;
    }
    fn evict(&mut self, i: usize) {
        if let Some(r) = self.records[i].take() {
            self.arena[r.offset..r.offset + r.len].fill(0);
        }
    }
    pub fn insert_admitted(
        &mut self,
        body: &[u8],
        state: CacheState,
        now: u64,
    ) -> Result<Insert, Error> {
        self.advance(now)?;
        let p = codec::parse(body, Context::StoredChat).map_err(|_| Error::Invalid)?;
        let expected = if p.header().flags & 7 == 0 {
            CacheState::Unverified
        } else {
            CacheState::Pending
        };
        if state != expected {
            return Err(Error::Invalid);
        }
        let hash = hash(body);
        if let Some(r) = self.records.iter_mut().flatten().find(|r| r.hash == hash) {
            r.touched = now;
            return Ok(if r.len == 0 {
                Insert::Expired
            } else {
                Insert::Existing(r.sequence)
            });
        }
        while self.used + body.len() > self.arena.len() || self.records.iter().all(Option::is_some)
        {
            let i = self
                .records
                .iter()
                .enumerate()
                .filter_map(|(i, r)| r.as_ref().map(|r| (r.len != 0, r.touched, r.sequence, i)))
                .min()
                .ok_or(Error::Full)?
                .3;
            self.evict(i);
            self.compact();
        }
        self.sequence = self.sequence.checked_add(1).ok_or(Error::Sequence)?;
        let slot = self
            .records
            .iter()
            .position(Option::is_none)
            .ok_or(Error::Full)?;
        self.arena[self.used..self.used + body.len()].copy_from_slice(body);
        // Keep the received stored TTL exactly, including local-only zero. The
        // relay codec alone clamps/decrements if a later live forward is allowed.
        self.records[slot] = Some(Record {
            hash,
            message: p.header().message_id,
            sequence: self.sequence,
            arrival: now,
            deadline: now + limits(self.mode).2,
            touched: now,
            offset: self.used,
            len: body.len(),
            state,
        });
        self.used += body.len();
        self.peak_encoded = self.peak_encoded.max(self.used);
        self.peak_entries = self
            .peak_entries
            .max(self.records.iter().flatten().filter(|r| r.len > 0).count());
        Ok(Insert::Added(self.sequence))
    }
    pub fn copy(
        &mut self,
        sequence: u64,
        now: u64,
        out: &mut [u8],
    ) -> Result<(usize, CacheState), Error> {
        self.advance(now)?;
        let r = self
            .records
            .iter_mut()
            .flatten()
            .find(|r| r.sequence == sequence && r.len > 0)
            .ok_or(Error::Missing)?;
        if out.len() < r.len {
            return Err(Error::Full);
        }
        out[..r.len].copy_from_slice(&self.arena[r.offset..r.offset + r.len]);
        r.touched = now;
        Ok((r.len, r.state))
    }
    /// Bounded newest-first sequence references only; no retained payload clone.
    pub fn snapshot(&mut self, now: u64, out: &mut [u64]) -> Result<usize, Error> {
        self.advance(now)?;
        let count = self.records.iter().flatten().filter(|r| r.len > 0).count();
        if out.len() < count {
            return Err(Error::Full);
        }
        for (slot, r) in out
            .iter_mut()
            .zip(self.records.iter().flatten().filter(|r| r.len > 0))
        {
            *slot = r.sequence;
        }
        out[..count].sort_unstable_by(|a, b| b.cmp(a));
        Ok(count)
    }
    pub fn bloom(&mut self, now: u64) -> Result<(u16, [u8; 512]), Error> {
        self.advance(now)?;
        let mut bits = [0; 512];
        let mut count = 0;
        for r in self.records.iter().flatten().filter(|r| r.len > 0) {
            for p in bloom_positions(&r.message) {
                bits[p / 8] |= 0x80 >> (p % 8);
            }
            count += 1;
        }
        Ok((count, bits))
    }
    pub fn reservations(&self) -> CacheReservations {
        CacheReservations {
            entries: self.records.iter().flatten().filter(|r| r.len > 0).count(),
            encoded_bytes: self.used,
            allocated_bytes: self.records.capacity() * size_of::<Option<Record>>()
                + self.arena.capacity()
                + size_of::<Self>(),
            peak_encoded: self.peak_encoded,
            peak_entries: self.peak_entries,
        }
    }
    /// Retention remains anchored to first arrival. Shrinking discards expired/LRU
    /// data before resizing; increasing cannot resurrect expired cached payloads.
    pub fn set_mode(&mut self, mode: Mode, now: u64) -> Result<(), Error> {
        self.advance(now)?;
        let (count, bytes, age) = limits(mode);
        for r in self.records.iter_mut().flatten() {
            if r.len > 0 {
                r.deadline = r.arrival + age;
            }
        }
        self.advance(now)?;
        while self.records.iter().flatten().count() > count || self.used > bytes {
            let i = self
                .records
                .iter()
                .enumerate()
                .filter_map(|(i, r)| r.as_ref().map(|r| (r.len != 0, r.touched, r.sequence, i)))
                .min()
                .unwrap()
                .3;
            self.evict(i);
            self.compact();
        }
        // Allocate the replacement layout within the new mode budget. No payload
        // duplicate is retained: shrink the existing arena before shrinking metadata.
        self.arena.resize(bytes, 0);
        self.arena.shrink_to_fit();
        let mut records = pool(count);
        for (out, r) in records
            .iter_mut()
            .zip(self.records.iter_mut().filter(|r| r.is_some()))
        {
            *out = r.take();
        }
        self.records = records;
        self.mode = mode;
        Ok(())
    }
}
