//! MC-014 outbound scheduling and advisory, per-egress coverage.
//! Call only after ingress/type/trust policy admits an object for content-neutral relay.
//! Neither enqueue nor native completion means remote delivery or authentication.
use crate::{
    LinkHandle,
    codec::{self, Context, Payload},
    framing::Encoder,
    power::{Mode, Platform, parameters},
};
use sha2::{Digest, Sha256};
use std::mem::size_of;
const QUANTA: [i16; 5] = [8, 16, 8, 4, 8];
const DEADLINE: u64 = 30_000;
const SCALE: u64 = 3_000;
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("invalid monotonic time")]
    Time,
    #[error("unknown/stale link or send completion")]
    Stale,
    #[error("invalid object/configuration")]
    Invalid,
    #[error("bounded queue full; object not sent")]
    Full,
    #[error("disconnect excess links before lowering the mode ceiling")]
    Links,
    #[error("sequence exhausted; reconnect required")]
    Sequence,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Traffic {
    Own,
    Forwarded,
    Local,
    Transport(u8),
}
#[derive(Debug, Clone, Copy)]
pub struct Request {
    pub traffic: Traffic,
    pub cookie: u64,
    pub random: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Control,
    Own,
    Chat,
    Reaction,
    Sync,
    Announce,
}
impl Kind {
    fn class(self) -> usize {
        match self {
            Self::Control => 0,
            Self::Own => 1,
            Self::Chat => 2,
            Self::Reaction => 3,
            Self::Sync | Self::Announce => 4,
        }
    }
    fn forwarding(self) -> bool {
        matches!(self, Self::Chat | Self::Reaction | Self::Sync)
    }
    fn relayed(self) -> bool {
        matches!(self, Self::Chat | Self::Reaction)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    NativeComplete,
    Suppressed,
    Expired,
    NativeFailed,
    Disconnected,
    Evicted,
    Coalesced,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultEvent {
    pub link: LinkHandle,
    pub cookie: u64,
    pub kind: Kind,
    pub status: Status,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Attempt {
    instance: u64,
    generation: u64,
    serial: u64,
}
#[derive(Debug)]
pub struct Send {
    pub attempt: Attempt,
    pub link: LinkHandle,
    pub cookie: u64,
    pub kind: Kind,
    pub len: usize,
    pub fragment: u8,
    pub frames: u8,
    pub retry: bool,
    pub ttl: Option<u8>,
}
#[derive(Debug, Default, Clone, Copy)]
pub struct Counters {
    pub attempts: u64,
    pub bytes: u64,
    pub retries: u64,
    pub forwarded_attempts: u64,
    pub suppressed: u64,
    pub failures: u64,
    pub refused: u64,
    pub coalesced: u64,
    pub peak_node_objects: usize,
    pub peak_link_objects: usize,
    pub peak_recent: usize,
    pub reaction_refused: u64,
    pub attempts_by_tier: [u64; 4],
    pub forwarded_by_mode: [u64; 3],
}
#[derive(Debug, Clone, Copy)]
pub struct Reservations {
    pub objects: usize,
    pub outbound_bytes: usize,
    pub outbound_bytes_per_link: usize,
    pub recent_bytes: usize,
    pub target_bytes: usize,
    pub manager_bytes: usize,
    pub links_bytes: usize,
}
#[derive(Clone, Copy)]
struct Bucket {
    credit: u64,
    capacity: u32,
    rate: u32,
    at: u64,
}
impl Bucket {
    fn new(capacity: u32, rate: u32, at: u64) -> Self {
        Self {
            credit: u64::from(capacity) * SCALE,
            capacity,
            rate,
            at,
        }
    }
    fn refill(&mut self, now: u64) {
        self.credit = (u128::from(self.credit) + u128::from(now - self.at) * u128::from(self.rate))
            .min(u128::from(self.capacity) * u128::from(SCALE)) as u64;
        self.at = now;
    }
    fn configure(&mut self, now: u64, capacity: u32, rate: u32) {
        self.refill(now);
        self.capacity = capacity;
        self.rate = rate;
        self.credit = self.credit.min(u64::from(capacity) * SCALE);
    }
}
fn charge(now: u64, costs: &mut [(&mut Bucket, u64)]) -> bool {
    for (b, _) in costs.iter_mut() {
        b.refill(now);
    }
    if costs
        .iter()
        .any(|(b, n)| u128::from(b.credit) < u128::from(*n) * u128::from(SCALE))
    {
        return false;
    }
    for (b, n) in costs {
        b.credit -= *n * SCALE;
    }
    true
}
struct Link {
    handle: LinkHandle,
    capacity: usize,
    frame: Bucket,
    bytes: Bucket,
    next_send: u64,
    ready: bool,
    active: Option<usize>,
    deficits: [i16; 5],
    cursor: usize,
    last_sync: bool,
    round_added: bool,
    transfer: u16,
    digest: [u8; 256],
    digest_until: u64,
}
struct Object {
    cookie: u64,
    serial: u64,
    link: usize,
    kind: Kind,
    transport: Option<u8>,
    body: [u8; 1035],
    len: usize,
    group: u16,
    frames: u8,
    index: u8,
    retry: bool,
    queued: u64,
    ready: u64,
    started: Option<u64>,
    pending: Option<u64>,
    hash: [u8; 32],
    message: [u8; 8],
}
struct Recent {
    hash: [u8; 32],
    message: [u8; 8],
    until: u64,
    peers: u8,
    reaction_counted: bool,
}
struct Target {
    id: [u8; 8],
    until: u64,
    count: u8,
}
pub struct Relay {
    instance: u64,
    generation: u64,
    serial: u64,
    attempt: u64,
    now: u64,
    platform: Platform,
    mode: Mode,
    tier: u8,
    suppression: bool,
    links: Vec<Option<Link>>,
    objects: Vec<Option<Object>>,
    recent: Vec<Option<Recent>>,
    targets: Vec<Option<Target>>,
    node_frame: Bucket,
    node_bytes: Bucket,
    forwarding: Bucket,
    cursor: usize,
    counters: Counters,
}
fn pool<T>(n: usize) -> Vec<Option<T>> {
    std::iter::repeat_with(|| None).take(n).collect()
}
fn hash(body: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(&body[..3]);
    h.update(&body[4..]);
    h.finalize().into()
}
fn positions(message: &[u8; 8]) -> [usize; 6] {
    let mut h = Sha256::new();
    h.update(b"meshfest-digest-v1");
    h.update(message);
    let h = h.finalize();
    let a = u64::from_be_bytes(h[..8].try_into().unwrap());
    let b = u64::from_be_bytes(h[8..16].try_into().unwrap());
    std::array::from_fn(|i| ((u128::from(a) + i as u128 * u128::from(b)) % 2048) as usize)
}
impl Relay {
    /// `suppression=false` is the identical-budget unsuppressed measurement baseline.
    pub fn new(
        instance: u64,
        platform: Platform,
        suppression: bool,
        now: u64,
    ) -> Result<Self, Error> {
        if instance == 0 || now > u64::MAX - 900_000 {
            return Err(Error::Invalid);
        }
        Ok(Self {
            instance,
            generation: 0,
            serial: 0,
            attempt: 0,
            now,
            platform,
            mode: Mode::Normal,
            tier: 1,
            suppression,
            links: pool(8),
            objects: pool(128),
            recent: pool(200),
            targets: pool(512),
            node_frame: Bucket::new(60, 24, now),
            node_bytes: Bucket::new(65536, 24576, now),
            forwarding: Bucket::new(120, 6, now),
            cursor: 0,
            counters: Counters::default(),
        })
    }
    fn time(&mut self, now: u64) -> Result<(), Error> {
        if now < self.now || now > u64::MAX - 900_000 {
            return Err(Error::Time);
        }
        self.now = now;
        Ok(())
    }
    fn link(&self, handle: &LinkHandle) -> Result<usize, Error> {
        self.links
            .iter()
            .position(|x| x.as_ref().is_some_and(|l| &l.handle == handle))
            .ok_or(Error::Stale)
    }
    pub fn register(
        &mut self,
        handle: &LinkHandle,
        capacity: usize,
        now: u64,
    ) -> Result<(), Error> {
        self.time(now)?;
        if handle.instance_nonce != self.instance || handle.generation <= self.generation {
            return Err(Error::Stale);
        }
        if !(146..=512).contains(&capacity) {
            return Err(Error::Invalid);
        }
        if self.links.iter().flatten().count()
            >= parameters(self.platform, self.mode, false, false).max_links
        {
            return Err(Error::Links);
        }
        let i = self
            .links
            .iter()
            .position(Option::is_none)
            .ok_or(Error::Links)?;
        self.links[i] = Some(Link {
            handle: handle.clone(),
            capacity,
            frame: Bucket::new(15, 3, now),
            bytes: Bucket::new(24576, 6144, now),
            next_send: now,
            ready: true,
            active: None,
            deficits: [0; 5],
            cursor: 0,
            last_sync: false,
            round_added: false,
            transfer: 0,
            digest: [0; 256],
            digest_until: 0,
        });
        self.generation = handle.generation;
        Ok(())
    }
    pub fn set_power(&mut self, mode: Mode, tier: u8, now: u64) -> Result<(), Error> {
        self.time(now)?;
        if tier > 3 || (mode == Mode::Beacon && self.platform == Platform::Ios) {
            return Err(Error::Invalid);
        }
        if self.links.iter().flatten().count()
            > parameters(self.platform, mode, false, false).max_links
        {
            return Err(Error::Links);
        }
        // Beacon bypasses only this bucket; still advance it so re-entry never grants credit.
        self.forwarding.configure(
            now,
            if mode == Mode::Saver { 40 } else { 120 },
            if mode == Mode::Saver { 2 } else { 6 },
        );
        self.mode = mode;
        self.tier = tier;
        Ok(())
    }
    pub fn ready(&mut self, handle: &LinkHandle, ready: bool, now: u64) -> Result<(), Error> {
        self.time(now)?;
        let i = self.link(handle)?;
        self.links[i].as_mut().unwrap().ready = ready;
        Ok(())
    }
    fn finish(&mut self, i: usize, status: Status, emit: &mut impl FnMut(ResultEvent)) {
        let o = self.objects[i].take().unwrap();
        let l = self.links[o.link].as_mut().unwrap();
        if l.active == Some(i) {
            l.active = None;
        }
        match status {
            Status::Suppressed => {
                self.counters.suppressed = self.counters.suppressed.saturating_add(1)
            }
            Status::Coalesced => {
                self.counters.coalesced = self.counters.coalesced.saturating_add(1)
            }
            Status::NativeComplete => {}
            _ => self.counters.failures = self.counters.failures.saturating_add(1),
        }
        emit(ResultEvent {
            link: l.handle.clone(),
            cookie: o.cookie,
            kind: o.kind,
            status,
        });
    }
    /// Any directional capacity change requires teardown and a fresh scoped generation.
    pub fn disconnect(
        &mut self,
        handle: &LinkHandle,
        now: u64,
        emit: &mut impl FnMut(ResultEvent),
    ) -> Result<(), Error> {
        self.time(now)?;
        let l = self.link(handle)?;
        for i in 0..self.objects.len() {
            if self.objects[i].as_ref().is_some_and(|o| o.link == l) {
                self.finish(i, Status::Disconnected, emit);
            }
        }
        self.links[l] = None;
        for r in self.recent.iter_mut().flatten() {
            r.peers &= !(1 << l);
        }
        Ok(())
    }
    /// Observe only complete, structurally valid, ingress-admitted content, including
    /// duplicate copies. Returns whether this exact recent variant is new locally.
    /// `None` records an admitted local origin. This never inserts trusted dedup.
    pub fn observe(
        &mut self,
        handle: Option<&LinkHandle>,
        body: &[u8],
        now: u64,
    ) -> Result<bool, Error> {
        self.time(now)?;
        let bit = match handle {
            Some(h) => 1 << self.link(h)?,
            None => 0,
        };
        let packet = codec::parse(body, Context::Live).map_err(|_| Error::Invalid)?;
        if let (Some(h), Payload::Announce { digest, .. }) = (handle, packet.payload()) {
            let i = self.link(h)?;
            let l = self.links[i].as_mut().unwrap();
            l.digest.fill(0);
            l.digest[..digest.len()].copy_from_slice(digest);
            l.digest_until = now + 60_000;
        }
        let hash = hash(body);
        self.expire_recent(now);
        if let Some(r) = self.recent.iter_mut().flatten().find(|r| r.hash == hash) {
            r.peers |= bit;
            return Ok(false);
        }
        let i = self
            .recent
            .iter()
            .position(Option::is_none)
            .unwrap_or_else(|| {
                self.recent
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, r)| r.as_ref().unwrap().until)
                    .unwrap()
                    .0
            });
        self.recent[i] = Some(Recent {
            hash,
            message: packet.header().message_id,
            until: now + 60_000,
            peers: bit,
            reaction_counted: false,
        });
        self.counters.peak_recent = self
            .counters
            .peak_recent
            .max(self.recent.iter().flatten().count());
        Ok(true)
    }
    fn expire_recent(&mut self, now: u64) {
        for r in &mut self.recent {
            if r.as_ref().is_some_and(|r| r.until <= now) {
                *r = None;
            }
        }
    }
    /// Advisory recent-ID filter; no trust/display decision may depend on it.
    pub fn digest(&mut self, now: u64) -> Result<[u8; 256], Error> {
        self.time(now)?;
        self.expire_recent(now);
        let mut out = [0; 256];
        for r in self.recent.iter().flatten() {
            for b in positions(&r.message) {
                out[b / 8] |= 0x80 >> (b % 8);
            }
        }
        Ok(out)
    }
    fn covered(&self, o: &Object, now: u64) -> bool {
        if !self.suppression || !o.kind.relayed() {
            return false;
        }
        let l = self.links[o.link].as_ref().unwrap();
        self.recent
            .iter()
            .flatten()
            .any(|r| r.until > now && r.hash == o.hash && r.peers & (1 << o.link) != 0)
            || (l.digest_until > now
                && positions(&o.message)
                    .iter()
                    .all(|&b| l.digest[b / 8] & (0x80 >> (b % 8)) != 0))
    }
    pub fn advance(&mut self, now: u64, emit: &mut impl FnMut(ResultEvent)) -> Result<(), Error> {
        self.time(now)?;
        self.expire_recent(now);
        for i in 0..self.objects.len() {
            if let Some(o) = &self.objects[i] {
                if now >= o.started.unwrap_or(o.queued) + DEADLINE {
                    self.finish(i, Status::Expired, emit);
                } else if o.started.is_none() && self.covered(o, now) {
                    self.finish(i, Status::Suppressed, emit);
                }
            }
        }
        Ok(())
    }
    /// Forwarded takes the RECEIVED bytes; the codec clamps/decrements TTL exactly
    /// once here. Local is restricted to direct controls and link announcements.
    /// Terminal results are synchronously emitted: the driver must surface own
    /// failures and abort served walks on eviction/expiry. No unbounded event queue.
    pub fn enqueue(
        &mut self,
        handle: &LinkHandle,
        body: &[u8],
        request: Request,
        now: u64,
        emit: &mut impl FnMut(ResultEvent),
    ) -> Result<Option<u64>, Error> {
        let Request {
            traffic,
            cookie,
            random,
        } = request;
        self.advance(now, emit)?;
        let l = self.link(handle)?;
        let mut encoded = [0; 1035];
        let (kind, transport, len, message, hash) = match traffic {
            Traffic::Transport(t) => {
                crate::framing::transport(t, body).map_err(|_| Error::Invalid)?;
                encoded[..body.len()].copy_from_slice(body);
                (
                    if t == 1 { Kind::Sync } else { Kind::Control },
                    Some(t),
                    body.len(),
                    [0; 8],
                    [0; 32],
                )
            }
            _ => {
                let p = codec::parse(body, Context::Live).map_err(|_| Error::Invalid)?;
                let kind = match traffic {
                    Traffic::Forwarded => {
                        if p.header().kind == 6 {
                            Kind::Reaction
                        } else {
                            Kind::Chat
                        }
                    }
                    Traffic::Own => {
                        if matches!(p.header().kind, 2 | 3 | 7) {
                            return Err(Error::Invalid);
                        }
                        Kind::Own
                    }
                    Traffic::Local => match p.header().kind {
                        2 => Kind::Announce,
                        3 | 7 => Kind::Control,
                        _ => return Err(Error::Invalid),
                    },
                    _ => unreachable!(),
                };
                let len = if traffic == Traffic::Forwarded {
                    match p.forward_to(&mut encoded).map_err(|_| Error::Invalid)? {
                        Some(n) => n,
                        None => return Ok(None),
                    }
                } else {
                    encoded[..body.len()].copy_from_slice(body);
                    body.len()
                };
                (kind, None, len, p.header().message_id, hash(body))
            }
        };
        let link = self.links[l].as_mut().unwrap();
        let group = link.transfer.checked_add(1).ok_or(Error::Sequence)?;
        let frames = match transport {
            Some(t) => Encoder::transport(t, &encoded[..len], link.capacity, group),
            None => Encoder::logical(&encoded[..len], link.capacity, group),
        }
        .map_err(|_| Error::Invalid)?
        .frame_count();
        let target = if kind == Kind::Reaction {
            match codec::parse(body, Context::Live)
                .map_err(|_| Error::Invalid)?
                .payload()
            {
                Payload::Reaction { target, .. } => Some(<[u8; 8]>::try_from(target).unwrap()),
                _ => None,
            }
        } else {
            None
        };
        let target_slot = if let Some(id) = target {
            for t in &mut self.targets {
                if t.as_ref().is_some_and(|t| t.until <= now) {
                    *t = None;
                }
            }
            self.expire_recent(now);
            if self
                .recent
                .iter()
                .flatten()
                .any(|r| r.hash == hash && r.reaction_counted)
            {
                None
            } else {
                let existing = self
                    .targets
                    .iter()
                    .position(|t| t.as_ref().is_some_and(|t| t.id == id));
                if existing.is_some_and(|i| self.targets[i].as_ref().unwrap().count >= 30) {
                    self.counters.reaction_refused =
                        self.counters.reaction_refused.saturating_add(1);
                    return Err(Error::Full);
                }
                Some((
                    existing
                        .or_else(|| self.targets.iter().position(Option::is_none))
                        .ok_or(Error::Full)?,
                    id,
                ))
            }
        } else {
            None
        };
        // Coalesce only unstarted ANNOUNCE; started fragments always finish intact.
        if kind == Kind::Announce {
            for i in 0..128 {
                if self.objects[i]
                    .as_ref()
                    .is_some_and(|o| o.link == l && o.kind == Kind::Announce && o.started.is_none())
                {
                    self.finish(i, Status::Coalesced, emit);
                }
            }
        }
        let count_link = self
            .objects
            .iter()
            .flatten()
            .filter(|o| o.link == l)
            .count();
        if count_link >= 32 || self.objects.iter().all(Option::is_some) {
            let only_link = count_link >= 32;
            let victim = self
                .objects
                .iter()
                .enumerate()
                .filter_map(|(i, x)| {
                    x.as_ref()
                        .filter(|o| o.started.is_none() && (!only_link || o.link == l))
                        .and_then(|o| match o.kind {
                            Kind::Reaction => Some((0, o.serial, i)),
                            Kind::Chat => Some((1, o.serial, i)),
                            Kind::Sync => Some((2, o.serial, i)),
                            _ => None,
                        })
                })
                .min();
            if let Some((_, _, i)) = victim {
                self.finish(i, Status::Evicted, emit);
            } else {
                self.counters.refused = self.counters.refused.saturating_add(1);
                return Err(Error::Full);
            }
        }
        let i = self
            .objects
            .iter()
            .position(Option::is_none)
            .ok_or(Error::Full)?;
        self.serial = self.serial.checked_add(1).ok_or(Error::Sequence)?;
        let (low, high) = match (self.mode, self.tier) {
            (Mode::Beacon, _) => (10, 40),
            (Mode::Saver, _) | (_, 0) => (300, 700),
            (_, 3) => (40, 150),
            _ => (80, 400),
        };
        let hold = if kind.relayed() && self.suppression {
            low + random % (high - low + 1)
        } else {
            0
        };
        if let Some((index, id)) = target_slot {
            let target = self.targets[index].get_or_insert(Target {
                id,
                until: now + 900_000,
                count: 0,
            });
            target.count += 1;
            // Share one reservation across all egress copies of this exact variant.
            // Eviction/drop never refunds it; missing recent state is conservative.
            let _ = self.observe(None, body, now)?;
            self.recent
                .iter_mut()
                .flatten()
                .find(|r| r.hash == hash)
                .unwrap()
                .reaction_counted = true;
        }
        self.objects[i] = Some(Object {
            cookie,
            serial: self.serial,
            link: l,
            kind,
            transport,
            body: encoded,
            len,
            group,
            frames: frames as u8,
            index: 0,
            retry: false,
            queued: now,
            ready: now + hold,
            started: None,
            pending: None,
            hash,
            message,
        });
        self.links[l].as_mut().unwrap().transfer = group;
        self.counters.peak_node_objects = self
            .counters
            .peak_node_objects
            .max(self.objects.iter().flatten().count());
        self.counters.peak_link_objects = self.counters.peak_link_objects.max(
            self.objects
                .iter()
                .flatten()
                .filter(|o| o.link == l)
                .count(),
        );
        Ok(Some(self.serial))
    }
    fn pick(&mut self, l: usize, now: u64) -> Option<usize> {
        if let Some(i) = self.links[l].as_ref().unwrap().active {
            return Some(i);
        }
        for _ in 0..25 {
            let link = self.links[l].as_mut().unwrap();
            let class = link.cursor;

            let first = |kind: Option<Kind>| {
                self.objects
                    .iter()
                    .enumerate()
                    .filter_map(|(i, o)| {
                        o.as_ref()
                            .filter(|o| {
                                o.link == l
                                    && o.kind.class() == class
                                    && kind.is_none_or(|k| o.kind == k)
                            })
                            .map(|o| (o.serial, i))
                    })
                    .min()
                    .map(|(_, i)| i)
            };
            let candidate = if class == 4 {
                let a = first(Some(if link.last_sync {
                    Kind::Announce
                } else {
                    Kind::Sync
                }));
                let b = first(Some(if link.last_sync {
                    Kind::Sync
                } else {
                    Kind::Announce
                }));
                a.or(b)
            } else {
                first(None)
            };
            let Some(i) = candidate else {
                link.deficits[class] = 0;
                link.cursor = (class + 1) % 5;
                link.round_added = false;
                continue;
            };
            if !link.round_added {
                link.deficits[class] = (link.deficits[class] + QUANTA[class]).min(32);
                link.round_added = true;
            }
            let o = self.objects[i].as_ref().unwrap();
            if o.ready <= now && i16::from(o.frames) <= link.deficits[class] {
                link.deficits[class] -= i16::from(o.frames);
                link.active = Some(i);
                if class == 4 {
                    link.last_sync = o.kind == Kind::Sync;
                }
                return Some(i);
            }
            link.cursor = (class + 1) % 5;
            link.round_added = false;
        }
        None
    }
    /// Reserves all actual-attempt credits immediately before the driver calls the
    /// native API. The driver must call complete once, including native failures.
    pub fn poll(
        &mut self,
        now: u64,
        out: &mut [u8],
        emit: &mut impl FnMut(ResultEvent),
    ) -> Result<Option<Send>, Error> {
        if out.len() < 512 {
            return Err(Error::Invalid);
        }
        self.advance(now, emit)?;
        for offset in 0..8 {
            let l = (self.cursor + offset) % 8;
            let Some(link) = &self.links[l] else {
                continue;
            };
            if !link.ready || now < link.next_send {
                continue;
            }
            let Some(i) = self.pick(l, now) else {
                continue;
            };
            let o = self.objects[i].as_mut().unwrap();
            if o.pending.is_some() {
                continue;
            }
            let link = self.links[l].as_mut().unwrap();
            let encoder = match o.transport {
                Some(t) => Encoder::transport(t, &o.body[..o.len], link.capacity, o.group),
                None => Encoder::logical(&o.body[..o.len], link.capacity, o.group),
            }
            .map_err(|_| Error::Invalid)?;
            let len = encoder
                .frame(usize::from(o.index), out)
                .map_err(|_| Error::Invalid)?;
            let mut costs = [
                (&mut link.frame, 1),
                (&mut link.bytes, len as u64),
                (&mut self.node_frame, 1),
                (&mut self.node_bytes, len as u64),
                (
                    &mut self.forwarding,
                    u64::from(o.kind.forwarding() && self.mode != Mode::Beacon),
                ),
            ];
            if !charge(now, &mut costs) {
                continue;
            }
            self.attempt = self.attempt.checked_add(1).ok_or(Error::Sequence)?;
            o.pending = Some(self.attempt);
            o.started.get_or_insert(now);
            link.next_send = now + 1000;
            if o.retry {
                link.deficits[o.kind.class()] -= 1;
                self.counters.retries = self.counters.retries.saturating_add(1);
            }
            self.counters.attempts = self.counters.attempts.saturating_add(1);
            self.counters.bytes = self.counters.bytes.saturating_add(len as u64);
            self.counters.attempts_by_tier[self.tier as usize] =
                self.counters.attempts_by_tier[self.tier as usize].saturating_add(1);
            if o.kind.forwarding() {
                self.counters.forwarded_attempts =
                    self.counters.forwarded_attempts.saturating_add(1);
                let m = match self.mode {
                    Mode::Normal => 0,
                    Mode::Saver => 1,
                    Mode::Beacon => 2,
                };
                self.counters.forwarded_by_mode[m] =
                    self.counters.forwarded_by_mode[m].saturating_add(1);
            }
            self.cursor = (l + 1) % 8;
            return Ok(Some(Send {
                attempt: Attempt {
                    instance: self.instance,
                    generation: link.handle.generation,
                    serial: self.attempt,
                },
                link: link.handle.clone(),
                cookie: o.cookie,
                kind: o.kind,
                len,
                fragment: o.index,
                frames: o.frames,
                retry: o.retry,
                ttl: o.transport.is_none().then_some(o.body[3]),
            }));
        }
        Ok(None)
    }
    pub fn complete(
        &mut self,
        attempt: Attempt,
        success: bool,
        now: u64,
        emit: &mut impl FnMut(ResultEvent),
    ) -> Result<(), Error> {
        self.advance(now, emit)?;
        if attempt.instance != self.instance {
            return Err(Error::Stale);
        }
        let i = self
            .objects
            .iter()
            .position(|x| {
                x.as_ref().is_some_and(|o| {
                    o.pending == Some(attempt.serial)
                        && self.links[o.link].as_ref().unwrap().handle.generation
                            == attempt.generation
                })
            })
            .ok_or(Error::Stale)?;
        let o = self.objects[i].as_mut().unwrap();
        o.pending = None;
        if success {
            o.index += 1;
            o.retry = false;
            if o.index == o.frames {
                self.finish(i, Status::NativeComplete, emit);
            }
        } else if o.retry {
            self.finish(i, Status::NativeFailed, emit);
        } else {
            o.retry = true;
        }
        Ok(())
    }
    pub fn reservations(&self) -> Reservations {
        Reservations {
            objects: self.objects.iter().flatten().count(),
            outbound_bytes: self.objects.capacity() * size_of::<Option<Object>>()
                + size_of::<Vec<Option<Object>>>(),
            outbound_bytes_per_link: 32 * size_of::<Option<Object>>(),
            recent_bytes: self.recent.capacity() * size_of::<Option<Recent>>()
                + size_of::<Vec<Option<Recent>>>(),
            target_bytes: self.targets.capacity() * size_of::<Option<Target>>()
                + size_of::<Vec<Option<Target>>>(),
            manager_bytes: size_of::<Self>() - 4 * size_of::<Vec<Option<Object>>>(),
            links_bytes: self.links.capacity() * size_of::<Option<Link>>()
                + size_of::<Vec<Option<Link>>>(),
        }
    }
    pub fn counters(&self) -> Counters {
        self.counters
    }
    pub fn next_wake(&self, now: u64) -> Option<u64> {
        self.objects
            .iter()
            .flatten()
            .map(|o| {
                let l = self.links[o.link].as_ref().unwrap();
                let deadline = o.started.unwrap_or(o.queued) + DEADLINE;
                let service = if o.pending.is_some() || !l.ready {
                    deadline
                } else if o.ready.max(l.next_send) > now {
                    o.ready.max(l.next_send)
                } else {
                    now.saturating_add(1000)
                };
                service.min(deadline).min(now.saturating_add(1000))
            })
            .min()
    }
}
