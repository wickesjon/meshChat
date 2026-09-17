//! Phone Beacon integration. Counts are local aggregates, never delivery claims.
use super::*;
use crate::sync::{
    Cache, CacheState,
    session::{ServeToken, Sessions},
};

#[derive(Debug, Clone, uniffi::Record)]
pub struct BeaconStatus {
    pub active: bool,
    pub infra: bool,
    pub battery_percent: u8,
    pub charging: bool,
    pub battery_exit_count: u64,
    pub uptime_ms: u64,
    pub cached_messages: u32,
    pub cache_bytes: u64,
    pub allocated_cache_bytes: u64,
    pub relay_frame_attempts: u64,
    pub transmitted_bytes: u64,
    pub refused_objects: u64,
    pub queued_objects: u32,
}
pub(super) struct Beacon {
    pub auto: bool,
    pub manual: Option<bool>,
    pub cache: Cache,
    pub sessions: Sessions,
    inflight: Vec<(LinkHandle, ServeToken)>,
    active: bool,
    infra: bool,
    battery: u8,
    charging: bool,
    exits: u64,
    since: u64,
}
impl Beacon {
    pub fn new(instance: u64, now: u64) -> Result<Self, TransportError> {
        Ok(Self {
            auto: false,
            manual: None,
            cache: Cache::new(power::Mode::Normal, now).map_err(|_| TransportError::Invalid)?,
            sessions: Sessions::new(instance, now).map_err(|_| TransportError::Invalid)?,
            inflight: Vec::with_capacity(2),
            active: false,
            infra: false,
            battery: 0,
            charging: false,
            exits: 0,
            since: now,
        })
    }
    pub fn advance(
        &mut self,
        now: u64,
    ) -> Result<Vec<crate::sync::session::Event>, TransportError> {
        self.cache
            .advance(now)
            .map_err(|_| TransportError::Unavailable)?;
        let mut ended = Vec::new();
        self.sessions
            .advance(now, &mut |event| ended.push(event))
            .map_err(|_| TransportError::Unavailable)?;
        Ok(ended)
    }
    pub fn power(
        &mut self,
        params: power::Parameters,
        battery: u8,
        charging: bool,
        now: u64,
    ) -> Result<(), TransportError> {
        self.cache
            .set_mode(params.mode, now)
            .map_err(|_| TransportError::Unavailable)?;
        let active = params.mode == power::Mode::Beacon;
        if active && !self.active {
            self.since = now;
        }
        self.active = active;
        self.infra = params.infra;
        self.battery = battery;
        self.charging = charging;
        if params.beacon_battery_exit {
            self.exits = self.exits.saturating_add(1);
        }
        Ok(())
    }
    pub fn cache_live(&mut self, raw: &[u8], now: u64) {
        let Ok(packet) = codec::parse(raw, codec::Context::Live) else {
            return;
        };
        if packet.header().kind != 1 {
            return;
        }
        let state = if packet.header().flags & 7 == 0 {
            CacheState::Unverified
        } else {
            CacheState::Pending
        };
        // Only already admitted/locally composed CHAT reaches this helper.
        // A pending signature/envelope stays pending; cache never grants trust.
        let _ = self.cache.insert_admitted(raw, state, now);
    }
    pub fn finished(&mut self, event: &relay::ResultEvent, now: u64) {
        if let Some(index) = self
            .inflight
            .iter()
            .position(|(link, _)| *link == event.link)
        {
            let (_, token) = self.inflight.remove(index);
            let _ = self.sessions.served_complete(
                token,
                event.status == relay::Status::NativeComplete,
                now,
                &mut |_| {},
            );
        }
    }
    pub fn disconnect(&mut self, link: &LinkHandle, now: u64) {
        self.inflight.retain(|(old, _)| old != link);
        let _ = self.sessions.disconnect(link, now, &mut |_| {});
    }
}
impl Runtime {
    pub(super) fn serve_cache(&mut self, out: &mut TransportEffects) -> Result<(), TransportError> {
        let links: Vec<_> = self
            .links
            .iter()
            .filter(|l| l.admitted)
            .map(|l| l.handle.clone())
            .collect();
        for link in links {
            if self.beacon.inflight.len() >= 2 {
                break;
            }
            if self.beacon.inflight.iter().any(|(old, _)| *old == link) {
                continue;
            }
            let mut raw = [0; 1035];
            let b = &mut self.beacon;
            let Ok(Some(item)) =
                b.sessions
                    .next_served(&link, &mut b.cache, self.now, &mut raw, &mut |_| {})
            else {
                continue;
            };
            b.inflight.push((link.clone(), item.token));
            // Reserved cookie zero cannot collide with application cookies >=3.
            if self
                .enqueue(
                    &link,
                    &raw[..item.len],
                    relay::Traffic::Transport(1),
                    0,
                    out,
                )
                .is_err()
            {
                self.beacon.inflight.retain(|(old, _)| *old != link);
                let _ =
                    self.beacon
                        .sessions
                        .served_complete(item.token, false, self.now, &mut |_| {});
            }
        }
        Ok(())
    }
}
#[uniffi::export]
impl NativeTransport {
    /// Queues an explicit setting for the next power sample. It never resets
    /// ingress/egress credits, identity, protected history or connection state.
    pub fn configure_beacon(
        &self,
        manual: bool,
        auto_while_charging: bool,
    ) -> Result<(), TransportError> {
        let mut s = self.state.lock().map_err(|_| TransportError::Unavailable)?;
        if s.platform != Platform::Android || s.failed {
            return Err(TransportError::Invalid);
        }
        s.beacon.auto = auto_while_charging;
        s.beacon.manual = Some(manual);
        Ok(())
    }
    pub fn beacon_status(&self, now: u64) -> Result<BeaconStatus, TransportError> {
        let mut s = self.state.lock().map_err(|_| TransportError::Unavailable)?;
        s.clock(now)?;
        let cache = s.beacon.cache.reservations();
        let relay = s.relay.counters();
        Ok(BeaconStatus {
            active: s.beacon.active,
            infra: s.beacon.infra,
            battery_percent: s.beacon.battery,
            charging: s.beacon.charging,
            battery_exit_count: s.beacon.exits,
            uptime_ms: if s.beacon.active {
                now - s.beacon.since
            } else {
                0
            },
            cached_messages: cache.entries as u32,
            cache_bytes: cache.encoded_bytes as u64,
            allocated_cache_bytes: cache.allocated_bytes as u64,
            relay_frame_attempts: relay.forwarded_attempts,
            transmitted_bytes: relay.bytes,
            refused_objects: relay.refused,
            queued_objects: s.relay.reservations().objects as u32,
        })
    }
    /// Applies the authoritative power hint to a locally composed ANNOUNCE
    /// before signing. Never modifies signed bytes or a remote announcement.
    pub fn beacon_announce(&self, mut bytes: Vec<u8>) -> Result<Vec<u8>, TransportError> {
        let packet =
            codec::parse(&bytes, codec::Context::Live).map_err(|_| TransportError::Invalid)?;
        if packet.header().kind != 2 || packet.header().flags != 0 {
            return Err(TransportError::Invalid);
        }
        let s = self.state.lock().map_err(|_| TransportError::Unavailable)?;
        if s.failed {
            return Err(TransportError::Unavailable);
        }
        let tier = if s.beacon.active || s.beacon.charging || s.beacon.battery == 100 {
            3
        } else if s.beacon.battery < 15 {
            0
        } else if s.beacon.battery < 40 {
            1
        } else {
            2
        };
        bytes[32] = tier | if s.beacon.infra { 4 } else { 0 };
        Ok(bytes)
    }
}
