//! Native GATT lifecycle around the existing handshake, intake and scheduler.
//! Transport admission never grants display, identity or delivery authority.
use crate::{
    LinkHandle, SendPath, codec, framing,
    friends::{Friends, Role},
    identity::{IdentityKeySession, PublicIdentity},
    ingress::{Ingress, Outcome, State},
    power::{self, Platform},
    relay::{self, Relay},
    storage::EncryptedStore,
};
#[path = "native_beacon.rs"]
mod beacon;
#[path = "native_messaging.rs"]
pub mod messaging;
pub use beacon::BeaconStatus;
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum TransportError {
    #[error("invalid transport input")]
    Invalid,
    #[error("unknown or stale operation")]
    Stale,
    #[error("transport resource or work limit")]
    Busy,
    #[error("outbound operation refused: {reason}")]
    Refused {
        reason: String,
        effects: TransportEffects,
    },
    #[error("protected identity unavailable")]
    Identity,
    #[error("transport clock or internal state unavailable; close native links")]
    Unavailable,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum TransportRole {
    Central,
    Peripheral,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum TransportTraffic {
    Own,
    Forwarded,
    Local,
}
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum TransportIntake {
    Unverified,
    Opaque,
    Pending,
    Duplicate,
    DeferredSync,
}
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum TransportStatus {
    NativeComplete,
    Failed,
    Expired,
    Cancelled,
    Suppressed,
}
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum TransportEvent {
    Admitted {
        link: LinkHandle,
        transmit_bytes: u16,
        receive_bytes: u16,
    },
    /// Structurally admitted bytes only; signed/encrypted packets remain pending.
    Received {
        link: LinkHandle,
        bytes: Vec<u8>,
        intake: TransportIntake,
    },
    Finished {
        link: LinkHandle,
        cookie: u64,
        status: TransportStatus,
    },
    Closed {
        link: LinkHandle,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TransportSend {
    pub link: LinkHandle,
    pub token: u64,
    pub path: SendPath,
    pub bytes: Vec<u8>,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, uniffi::Record)]
pub struct TransportEffects {
    pub sends: Vec<TransportSend>,
    pub events: Vec<TransportEvent>,
}
#[derive(Debug, Clone, uniffi::Record)]
pub struct TransportConnection {
    pub link: LinkHandle,
    pub effects: TransportEffects,
}
/// Ordinary user settings. Beacon configuration is Android-only and separate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum TransportPowerSetting {
    Auto,
    Normal,
    Saver,
}
#[derive(Debug, Clone, uniffi::Record)]
pub struct TransportPower {
    pub beacon: bool,
    pub infra: bool,
    pub beacon_battery_exit: bool,
    pub saver: bool,
    pub link_limit: u16,
    pub battery_tier: u8,
    pub scan_on_ms: u64,
    pub scan_off_ms: u64,
    pub announce_ms: u64,
    pub cancelled_admissions: Vec<u64>,
    pub effects: TransportEffects,
}
/// Local connection-selection hints, never identity, display or delivery authority.
#[derive(Debug, Clone, uniffi::Record)]
pub struct TransportObservation {
    /// Fresh ANNOUNCE claim, used only for bounded connection preference.
    pub peer_count: Option<u8>,
    pub link: LinkHandle,
    /// Budget-admitted, text-valid clear CHAT/ANNOUNCE, including an explicitly
    /// work-budgeted full-key friend signature check. Opaque/encrypted and
    /// unverified signed bytes never refresh this timestamp. No trust is granted.
    pub last_valid_ms: Option<u64>,
    pub first_valid_ms: Option<u64>,
    /// Missing local samples or expired/absent peer digest means unknown.
    /// Per-mille of local recent IDs absent from the advisory peer digest.
    pub novelty: Option<u16>,
}
struct PendingSend {
    protected: bool,
    pin: Option<crate::friends::SendToken>,
    token: u64,
    attempt: relay::Attempt,
    started: u64,
    cookie: u64,
}
struct Link {
    handle: LinkHandle,
    role: TransportRole,
    born: u64,
    admitted: bool,
    proof: Option<[u8; 66]>,
    proof_queued: bool,
    pending: Option<PendingSend>,
    last_valid_ms: Option<u64>,
    first_valid_ms: Option<u64>,
    digest: Option<(u64, [u8; 256])>,
    native_activity: u64,
    peer_count: Option<(u64, u8)>,
}
struct Admission {
    token: u64,
    born: u64,
}
struct Runtime {
    instance: u64,
    serial: u64,
    now: u64,
    ingress: Ingress,
    friends: Friends,
    messaging: messaging::Messaging,
    relay: Relay,
    links: Vec<Link>,
    admissions: Vec<Admission>,
    failed: bool,
    power: power::Policy,
    platform: Platform,
    link_limit: usize,
    recent: Vec<([u8; 8], u64)>,
    beacon: beacon::Beacon,
}
impl Runtime {
    fn next(&mut self) -> Result<u64, TransportError> {
        self.serial = self
            .serial
            .checked_add(1)
            .ok_or(TransportError::Unavailable)?;
        Ok(self.serial)
    }
    fn index(&self, link: &LinkHandle) -> Result<usize, TransportError> {
        self.links
            .iter()
            .position(|l| l.handle == *link)
            .ok_or(TransportError::Stale)
    }
    fn clock(&mut self, now: u64) -> Result<(), TransportError> {
        if self.failed || now < self.now || now > u64::MAX - 3_600_000 {
            self.failed = true;
            self.friends.clear_session_trust();
            return Err(TransportError::Unavailable);
        }
        self.now = now;
        self.beacon.advance(now)?;
        self.recent.retain(|(_, at)| now - *at < 60_000);
        self.admissions.retain(|a| now < a.born + 30_000);
        self.ingress
            .advance(now)
            .map_err(|_| TransportError::Unavailable)
    }
    fn results(&mut self, events: Vec<relay::ResultEvent>, out: &mut TransportEffects) {
        for event in events {
            if event.cookie == 0 {
                self.beacon.finished(&event, self.now);
                continue;
            }
            self.messaging.finished(&mut self.friends, &event);
            if event.cookie >= 3 {
                out.events.push(TransportEvent::Finished {
                    link: event.link,
                    cookie: event.cookie,
                    status: match event.status {
                        relay::Status::NativeComplete => TransportStatus::NativeComplete,
                        relay::Status::Expired => TransportStatus::Expired,
                        relay::Status::NativeFailed => TransportStatus::Failed,
                        relay::Status::Suppressed => TransportStatus::Suppressed,
                        _ => TransportStatus::Cancelled,
                    },
                });
            }
        }
    }
    fn close(
        &mut self,
        link: &LinkHandle,
        out: &mut TransportEffects,
    ) -> Result<(), TransportError> {
        let index = self.index(link)?;
        let mut events = Vec::new();
        self.relay
            .disconnect(link, self.now, &mut |e| events.push(e))
            .map_err(|_| TransportError::Unavailable)?;
        // Friends may already have invalidated this session on a changed HELLO.
        let _ = self.friends.disconnect(&mut self.ingress, link, self.now);
        self.beacon.disconnect(link, self.now);
        self.messaging.disconnected();
        self.links.remove(index);
        self.results(events, out);
        out.events
            .push(TransportEvent::Closed { link: link.clone() });
        Ok(())
    }
    fn admission_event(
        &mut self,
        index: usize,
        out: &mut TransportEffects,
    ) -> Result<(), TransportError> {
        let link = &mut self.links[index];
        if !link.admitted && self.friends.link_ready(&link.handle) {
            let (tx, rx) = self
                .friends
                .effective_capacities(&link.handle)
                .map_err(|_| TransportError::Stale)?;
            self.beacon
                .sessions
                .register(&link.handle, self.now)
                .map_err(|_| TransportError::Unavailable)?;
            link.admitted = true;
            out.events.push(TransportEvent::Admitted {
                link: link.handle.clone(),
                transmit_bytes: tx,
                receive_bytes: rx,
            });
        }
        Ok(())
    }
    fn consolidate(&mut self, out: &mut TransportEffects) -> Result<(), TransportError> {
        for link in self.friends.duplicate_links_to_close() {
            self.close(&link, out)?;
        }
        Ok(())
    }
    fn remember(&mut self, id: [u8; 8]) {
        if self.recent.iter().any(|(old, _)| *old == id) {
            return;
        }
        if self.recent.len() == 200 {
            self.recent.remove(0);
        }
        self.recent.push((id, self.now));
    }
    /// Activity-only verification does not resolve pending content, write history,
    /// pin identity or confer display authority. The content owner still performs
    /// its own charged acceptance. First signed content and every signed ANNOUNCE
    /// include a full key; missing-key packets remain ineligible for this hint.
    fn signed_activity(&mut self, index: usize, raw: &[u8]) -> bool {
        let Ok(packet) = codec::parse(raw, codec::Context::Live) else {
            return false;
        };
        let signature = match packet.payload() {
            codec::Payload::Chat {
                signature: codec::Signature::Friend(sig),
                ..
            } if packet.header().channel_id != [0x0e, 0x9d, 0x09, 0x74] => sig,
            codec::Payload::Announce {
                signature: Some(sig),
                ..
            } => sig,
            _ => return false,
        };
        let Some(key) = signature
            .public_key
            .and_then(|key| <[u8; 32]>::try_from(key).ok())
        else {
            return false;
        };
        let link = &self.links[index].handle;
        let Ok(Some(permit)) = self.ingress.begin_signature(link, raw, self.now) else {
            return false;
        };
        if self.ingress.finish_work(permit).is_err() {
            return false;
        }
        // MC-008 friend-sign/v1: same immutable-header/body transcript as Friends.
        let body = &raw[26..raw.len() - 64];
        let mut transcript = b"meshfest/friend-sign/v1\0".to_vec();
        transcript.extend_from_slice(&raw[..3]);
        transcript.extend_from_slice(&raw[4..26]);
        transcript.extend_from_slice(&(body.len() as u16).to_be_bytes());
        transcript.extend_from_slice(body);
        Sha256::digest(key)[..8] == packet.header().sender_id
            && crate::friends::verify(&key, &transcript, signature.signature)
            && crate::text::validate_payload(packet.payload()).is_ok()
    }
    fn activity(&mut self, index: usize, raw: &[u8]) {
        let Ok(packet) = codec::parse(raw, codec::Context::Live) else {
            return;
        };
        match packet.payload() {
            codec::Payload::Chat { .. } => {
                self.links[index].last_valid_ms = Some(self.now);
                self.links[index].first_valid_ms.get_or_insert(self.now);
                self.remember(packet.header().message_id);
            }
            codec::Payload::Announce {
                digest, peer_count, ..
            } => {
                self.links[index].peer_count = Some((self.now, peer_count));
                self.links[index].last_valid_ms = Some(self.now);
                self.links[index].first_valid_ms.get_or_insert(self.now);
                self.links[index].digest = digest.try_into().ok().map(|bytes| (self.now, bytes));
            }
            _ => {}
        }
    }
    fn enqueue(
        &mut self,
        link: &LinkHandle,
        bytes: &[u8],
        traffic: relay::Traffic,
        cookie: u64,
        out: &mut TransportEffects,
    ) -> Result<(), TransportError> {
        if matches!(traffic, relay::Traffic::Own) {
            self.beacon.cache_live(bytes, self.now);
        }
        let mut random = [0; 8];
        getrandom::fill(&mut random).map_err(|_| TransportError::Unavailable)?;
        let mut events = Vec::new();
        let result = self.relay.enqueue(
            link,
            bytes,
            relay::Request {
                traffic,
                cookie,
                random: u64::from_le_bytes(random),
            },
            self.now,
            &mut |e| events.push(e),
        );
        self.results(events, out);
        // Refusal can follow expiration/eviction of other objects. Preserve
        // every terminal effect even when this new enqueue does not succeed.
        match result {
            Err(error) => {
                return Err(TransportError::Refused {
                    reason: error.to_string(),
                    effects: out.clone(),
                });
            }
            Ok(None) if cookie >= 3 => out.events.push(TransportEvent::Finished {
                link: link.clone(),
                cookie,
                status: TransportStatus::Suppressed,
            }),
            _ => {}
        }
        Ok(())
    }
}

#[derive(uniffi::Object)]
pub struct NativeTransport {
    state: Mutex<Runtime>,
}
#[uniffi::export]
impl NativeTransport {
    /// Android Normal mode: six links including connections still being set up.
    /// The trusted adapter supplies public metadata from its protected provider.
    /// No unlocked private-key session is retained by this transport owner.
    #[uniffi::constructor]
    pub fn new(
        store: Arc<EncryptedStore>,
        identity: PublicIdentity,
        instance_nonce: u64,
        monotonic_ms: u64,
    ) -> Result<Self, TransportError> {
        Self::create(
            store,
            identity,
            instance_nonce,
            monotonic_ms,
            Platform::Android,
        )
    }
    /// iOS uses four Normal links and readiness, without per-write callbacks.
    #[uniffi::constructor]
    pub fn new_ios(
        store: Arc<EncryptedStore>,
        identity: PublicIdentity,
        instance_nonce: u64,
        monotonic_ms: u64,
    ) -> Result<Self, TransportError> {
        Self::create(store, identity, instance_nonce, monotonic_ms, Platform::Ios)
    }
    /// None retains the current user setting and Auto hysteresis. Mode changes
    /// preserve all node/address credits; cancelled setup permits are returned
    /// so the native owner can release those resources too.
    pub fn update_power(
        &self,
        setting: Option<TransportPowerSetting>,
        battery_percent: u8,
        charging: bool,
        visible_peers: u16,
        now: u64,
    ) -> Result<TransportPower, TransportError> {
        if battery_percent > 100 || visible_peers > 64 {
            return Err(TransportError::Invalid);
        }
        let mut s = self.state.lock().map_err(|_| TransportError::Unavailable)?;
        s.clock(now)?;
        let input = power::Inputs {
            battery_percent,
            charging,
            visible_peers: usize::from(visible_peers),
            auto_beacon: s.beacon.auto,
        };
        let beacon_setting = s.beacon.manual.take().map(|manual| {
            if manual {
                power::Setting::Beacon
            } else {
                power::Setting::Auto
            }
        });
        let selected = beacon_setting.or(setting.map(|value| match value {
            TransportPowerSetting::Auto => power::Setting::Auto,
            TransportPowerSetting::Normal => power::Setting::Normal,
            TransportPowerSetting::Saver => power::Setting::Saver,
        }));
        let params = if let Some(setting) = selected {
            s.power.set(setting, input, now)
        } else {
            s.power.update(input, now)
        }
        .map_err(|_| TransportError::Invalid)?;
        let mut out = TransportEffects::default();
        let mut cancelled = Vec::new();
        while s.links.len() + s.admissions.len() > params.max_links {
            if let Some(admission) = s.admissions.pop() {
                cancelled.push(admission.token);
            } else {
                let link = s
                    .links
                    .last()
                    .ok_or(TransportError::Unavailable)?
                    .handle
                    .clone();
                s.close(&link, &mut out)?;
            }
        }
        let tier = if params.mode == power::Mode::Beacon || charging || battery_percent == 100 {
            3
        } else if battery_percent < 15 {
            0
        } else if battery_percent < 40 {
            1
        } else {
            2
        };
        s.ingress
            .set_link_limit(params.max_links)
            .map_err(|_| TransportError::Unavailable)?;
        s.relay
            .set_power(params.mode, tier, now)
            .map_err(|_| TransportError::Unavailable)?;
        s.beacon.power(params, battery_percent, charging, now)?;
        s.link_limit = params.max_links;
        Ok(TransportPower {
            beacon: params.mode == power::Mode::Beacon,
            infra: params.infra,
            beacon_battery_exit: params.beacon_battery_exit,
            saver: params.mode == power::Mode::Saver,
            link_limit: params.max_links as u16,
            battery_tier: tier,
            scan_on_ms: params.scan_on_ms,
            scan_off_ms: params.scan_off_ms,
            announce_ms: params.announce_ms,
            cancelled_admissions: cancelled,
            effects: out,
        })
    }
    pub fn observations(&self, now: u64) -> Result<Vec<TransportObservation>, TransportError> {
        let mut s = self.state.lock().map_err(|_| TransportError::Unavailable)?;
        s.clock(now)?;
        Ok(s.links
            .iter()
            .map(|link| {
                let novelty = link
                    .digest
                    .as_ref()
                    .filter(|(at, _)| now - *at < 60_000)
                    .filter(|_| !s.recent.is_empty())
                    .map(|(_, bits)| {
                        let absent = s
                            .recent
                            .iter()
                            .filter(|(id, _)| {
                                let mut hash = Sha256::new();
                                hash.update(b"meshfest-digest-v1");
                                hash.update(id);
                                let digest = hash.finalize();
                                let a = u64::from_be_bytes(digest[..8].try_into().unwrap()) % 2048;
                                let b =
                                    u64::from_be_bytes(digest[8..16].try_into().unwrap()) % 2048;
                                !(0..6).all(|i| {
                                    let p = ((a + i * b) % 2048) as usize;
                                    bits[p / 8] & (0x80 >> (p % 8)) != 0
                                })
                            })
                            .count();
                        (absent * 1000 / s.recent.len()) as u16
                    });
                TransportObservation {
                    peer_count: link
                        .peer_count
                        .filter(|(at, _)| now - at < 60_000)
                        .map(|(_, count)| count),
                    link: link.handle.clone(),
                    last_valid_ms: link.last_valid_ms,
                    first_valid_ms: link.first_valid_ms,
                    novelty,
                }
            })
            .collect())
    }
    /// Reserve BEFORE initiating a central connection or accepting a peripheral.
    /// Address is a process-local 16-byte digest, not an authenticated identity.
    pub fn admit_connection(&self, address: Vec<u8>, now: u64) -> Result<u64, TransportError> {
        let address = address.try_into().map_err(|_| TransportError::Invalid)?;
        let mut s = self.state.lock().map_err(|_| TransportError::Unavailable)?;
        s.clock(now)?;
        if s.links.len() + s.admissions.len() >= s.link_limit
            || !s
                .ingress
                .connection_attempt(address, now)
                .map_err(|_| TransportError::Unavailable)?
        {
            return Err(TransportError::Busy);
        }
        let token = s.next()?;
        s.admissions.push(Admission { token, born: now });
        Ok(token)
    }
    pub fn cancel_connection(&self, admission: u64, now: u64) -> Result<(), TransportError> {
        let mut s = self.state.lock().map_err(|_| TransportError::Unavailable)?;
        s.clock(now)?;
        s.admissions.retain(|a| a.token != admission);
        Ok(())
    }
    /// Call only after successful CCCD setup and runtime MTU evidence in BOTH
    /// directions. Unknown (zero), default-small and below-floor limits refuse.
    pub fn native_ready(
        &self,
        admission: u64,
        role: TransportRole,
        transmit_bytes: u16,
        receive_bytes: u16,
        now: u64,
    ) -> Result<TransportConnection, TransportError> {
        let mut s = self.state.lock().map_err(|_| TransportError::Unavailable)?;
        s.clock(now)?;
        let at = s
            .admissions
            .iter()
            .position(|a| a.token == admission)
            .ok_or(TransportError::Stale)?;
        s.admissions.remove(at);
        if !(146..=512).contains(&transmit_bytes) || !(146..=512).contains(&receive_bytes) {
            return Err(TransportError::Invalid);
        }
        let link = LinkHandle {
            instance_nonce: s.instance,
            generation: s.next()?,
        };
        let Runtime {
            friends, ingress, ..
        } = &mut *s;
        let hello = friends
            .start_link(
                ingress,
                link.clone(),
                match role {
                    TransportRole::Central => Role::Central,
                    TransportRole::Peripheral => Role::Peripheral,
                },
                (transmit_bytes, receive_bytes),
                now,
                || {
                    let mut nonce = [0; 16];
                    getrandom::fill(&mut nonce).map_err(|_| crate::friends::Error::Provider)?;
                    Ok(nonce)
                },
            )
            .map_err(|_| TransportError::Identity)?;
        // The fixed floor is a conservative ENCODING ceiling, never a claimed
        // native measurement. Every admitted direction can carry it. It keeps
        // HELLO and subsequent objects on one scheduler with unchanged credits.
        // Peers may still send up to their actual negotiated directional limit.
        if s.relay.register(&link, framing::MIN_CAPACITY, now).is_err() {
            let Runtime {
                friends, ingress, ..
            } = &mut *s;
            let _ = friends.disconnect(ingress, &link, now);
            return Err(TransportError::Busy);
        }
        s.links.push(Link {
            handle: link.clone(),
            role,
            born: now,
            admitted: false,
            proof: None,
            proof_queued: false,
            pending: None,
            last_valid_ms: None,
            first_valid_ms: None,
            digest: None,
            native_activity: now,
            peer_count: None,
        });
        let mut out = TransportEffects::default();
        if let Err(error) = s.enqueue(&link, &hello, relay::Traffic::Transport(2), 1, &mut out) {
            s.close(&link, &mut out)?;
            return Err(match error {
                TransportError::Refused { reason, .. } => TransportError::Refused {
                    reason,
                    effects: out,
                },
                other => other,
            });
        }
        Ok(TransportConnection { link, effects: out })
    }
    /// A fresh short-lived protected provider session is supplied for this call.
    /// The caller invalidates it on return, including errors. Busy can be retried
    /// before the existing proof deadline; no proof renewal is introduced.
    pub fn prepare_proof(
        &self,
        link: LinkHandle,
        provider: Arc<IdentityKeySession>,
        now: u64,
    ) -> Result<TransportEffects, TransportError> {
        let mut s = self.state.lock().map_err(|_| TransportError::Unavailable)?;
        s.clock(now)?;
        let i = s.index(&link)?;
        if s.links[i].proof_queued {
            return Ok(TransportEffects::default());
        }
        let Runtime {
            friends, ingress, ..
        } = &mut *s;
        let proof = friends
            .local_proof(ingress, &provider, &link, now)
            .map_err(|_| TransportError::Identity)?
            .ok_or(TransportError::Busy)?;
        let mut out = TransportEffects::default();
        s.enqueue(&link, &proof, relay::Traffic::Transport(3), 2, &mut out)?;
        s.links[i].proof = Some(proof);
        s.links[i].proof_queued = true;
        Ok(out)
    }
    pub fn receive(
        &self,
        link: LinkHandle,
        reported_bytes: u64,
        bytes: Vec<u8>,
        now: u64,
    ) -> Result<TransportEffects, TransportError> {
        let mut s = self.state.lock().map_err(|_| TransportError::Unavailable)?;
        s.clock(now)?;
        let i = s.index(&link)?;
        let mut out = TransportEffects::default();
        s.links[i].native_activity = now;
        // This Rust dispatch only selects an owner. Existing ingress/framing
        // still charges and validates every byte before control interpretation.
        let control = !s.links[i].admitted
            || (bytes.first() == Some(&2) && matches!(bytes.get(4), Some(2 | 3)));
        let Runtime {
            friends, ingress, ..
        } = &mut *s;
        if control {
            if friends
                .receive(ingress, &link, reported_bytes, &bytes, now)
                .is_err()
            {
                s.close(&link, &mut out)?;
            } else {
                s.admission_event(i, &mut out)?;
            }
        } else {
            let capacity = friends
                .receive_capacity(&link)
                .map_err(|_| TransportError::Stale)?;
            let mut output = [0; 1035];
            // Oversize native values still enter the charged refusal path. All
            // other values use the same ingress plus an unforgeable SYNC receipt.
            let (result, sync) = if bytes.len() > capacity {
                (
                    ingress
                        .receive_friend(
                            &link,
                            (reported_bytes, &bytes),
                            now,
                            &mut output,
                            (false, capacity),
                        )
                        .map_err(|_| TransportError::Stale)?,
                    None,
                )
            } else {
                let admitted = ingress
                    .receive_deferred_sync(&link, reported_bytes, &bytes, now, &mut output)
                    .map_err(|_| TransportError::Stale)?;
                (admitted.outcome, admitted.sync)
            };
            if let Some(admission) = sync {
                if let Outcome::Complete {
                    kind: framing::ObjectKind::Logical,
                    len,
                    state,
                } = result
                {
                    let b = &mut s.beacon;
                    let _ = b.sessions.accept_admitted_request(
                        &link,
                        admission,
                        &mut b.cache,
                        now,
                        &mut |_| {},
                    );
                    // Direct requests never enter mesh relay/digest/history
                    // state. Preserve only the existing diagnostic event.
                    out.events.push(TransportEvent::Received {
                        link,
                        bytes: output[..len].to_vec(),
                        intake: if state == State::Duplicate {
                            TransportIntake::Duplicate
                        } else {
                            TransportIntake::Unverified
                        },
                    });
                    return Ok(out);
                }
                // Requesting/history presentation is owned separately; an
                // unsolicited stored wrapper never becomes live CHAT here.
            }
            if let Outcome::Complete { len, state, .. } = result {
                if state != State::DeferredSync && state != State::Control {
                    let now = s.now;
                    s.relay
                        .observe(Some(&link), &output[..len], now)
                        .map_err(|_| TransportError::Unavailable)?;
                }
                if matches!(state, State::Unverified | State::Pending | State::Opaque) {
                    s.beacon.cache_live(&output[..len], now);
                }
                let clear = matches!(state, State::Unverified | State::Duplicate)
                    && output.get(2).is_some_and(|flags| flags & 7 == 0);
                let signed = matches!(state, State::Pending | State::PendingDuplicate)
                    && s.signed_activity(i, &output[..len]);
                if clear || signed {
                    s.activity(i, &output[..len]);
                }
                let intake = match state {
                    State::Unverified => TransportIntake::Unverified,
                    State::Opaque => TransportIntake::Opaque,
                    State::Pending | State::PendingDuplicate => TransportIntake::Pending,
                    State::Duplicate => TransportIntake::Duplicate,
                    State::DeferredSync => TransportIntake::DeferredSync,
                    State::Control => return Ok(out),
                };
                out.events.push(TransportEvent::Received {
                    link,
                    bytes: output[..len].to_vec(),
                    intake,
                });
            }
        }
        s.consolidate(&mut out)?;
        Ok(out)
    }
    /// Trusted protocol-owner egress. Caller must already have applied the
    /// corresponding content/trust/SYNC policy. This method grants none of it.
    pub fn enqueue(
        &self,
        link: LinkHandle,
        bytes: Vec<u8>,
        traffic: TransportTraffic,
        cookie: u64,
        now: u64,
    ) -> Result<TransportEffects, TransportError> {
        if bytes.len() > 1024 || cookie < 3 {
            return Err(TransportError::Invalid);
        }
        let mut s = self.state.lock().map_err(|_| TransportError::Unavailable)?;
        s.clock(now)?;
        if !s.links[s.index(&link)?].admitted {
            return Err(TransportError::Stale);
        }
        let mut out = TransportEffects::default();
        if matches!(traffic, TransportTraffic::Own) {
            s.relay
                .observe(None, &bytes, now)
                .map_err(|_| TransportError::Invalid)?;
        }
        s.enqueue(
            &link,
            &bytes,
            match traffic {
                TransportTraffic::Own => relay::Traffic::Own,
                TransportTraffic::Forwarded => relay::Traffic::Forwarded,
                TransportTraffic::Local => relay::Traffic::Local,
            },
            cookie,
            &mut out,
        )?;
        if let Ok(packet) = codec::parse(&bytes, codec::Context::Live) {
            if packet.header().kind == 1 {
                s.remember(packet.header().message_id);
            }
        }
        Ok(out)
    }
    /// A trusted SYNC owner supplies a structurally valid item/marker after its
    /// session, history and authorization checks. Native completion is not SYNC
    /// completion. Incoming wrappers remain DeferredSync for that owner.
    pub fn enqueue_sync(
        &self,
        link: LinkHandle,
        bytes: Vec<u8>,
        cookie: u64,
        now: u64,
    ) -> Result<TransportEffects, TransportError> {
        if bytes.len() > 1035 || cookie < 3 {
            return Err(TransportError::Invalid);
        }
        let mut s = self.state.lock().map_err(|_| TransportError::Unavailable)?;
        s.clock(now)?;
        if !s.links[s.index(&link)?].admitted {
            return Err(TransportError::Stale);
        }
        let mut out = TransportEffects::default();
        s.enqueue(
            &link,
            &bytes,
            relay::Traffic::Transport(1),
            cookie,
            &mut out,
        )?;
        Ok(out)
    }
    /// One bounded batch, at most one outstanding GATT value per live link.
    /// Commands must be submitted immediately on the serialized native owner.
    pub fn tick(&self, now: u64) -> Result<TransportEffects, TransportError> {
        let mut s = self.state.lock().map_err(|_| TransportError::Unavailable)?;
        s.clock(now)?;
        let mut out = TransportEffects::default();
        let expired: Vec<_> = s
            .links
            .iter()
            .filter(|l| {
                (!l.admitted && now > l.born + 10_000)
                    || (s.platform == Platform::Android
                        && l.pending.as_ref().is_some_and(|p| now >= p.started + 5_000))
                    || (s.platform == Platform::Ios && now >= l.native_activity + 15_000)
            })
            .map(|l| l.handle.clone())
            .collect();
        for link in expired {
            s.close(&link, &mut out)?;
        }
        s.serve_cache(&mut out)?;
        for _ in 0..8 {
            let mut raw = [0; 512];
            let mut events = Vec::new();
            let send = s
                .relay
                .poll(now, &mut raw, &mut |e| events.push(e))
                .map_err(|_| TransportError::Unavailable)?;
            s.results(events, &mut out);
            let Some(send) = send else {
                break;
            };
            let index = s.index(&send.link)?;
            let token = s.next()?;
            let (protected, pin) = s.messaging.egress_guard(&send.link, send.cookie);
            let link = &mut s.links[index];
            link.pending = Some(PendingSend {
                protected,
                pin,
                token,
                attempt: send.attempt,
                started: now,
                cookie: send.cookie,
            });
            out.sends.push(TransportSend {
                link: send.link,
                token,
                path: match link.role {
                    TransportRole::Central => SendPath::Write,
                    TransportRole::Peripheral => SendPath::Notify,
                },
                bytes: raw[..send.len].to_vec(),
            });
        }
        Ok(out)
    }
    /// Current native readiness level. Checking a property is not a liveness event.
    pub fn set_writable(
        &self,
        link: LinkHandle,
        writable: bool,
        now: u64,
    ) -> Result<(), TransportError> {
        let mut s = self.state.lock().map_err(|_| TransportError::Unavailable)?;
        s.clock(now)?;
        s.index(&link)?;
        if s.platform != Platform::Ios {
            return Err(TransportError::Invalid);
        }
        s.relay
            .ready(&link, writable, now)
            .map_err(|_| TransportError::Stale)
    }
    /// Only an actual CoreBluetooth readiness callback refreshes native liveness.
    pub fn readiness(&self, link: LinkHandle, now: u64) -> Result<(), TransportError> {
        let mut s = self.state.lock().map_err(|_| TransportError::Unavailable)?;
        s.clock(now)?;
        let i = s.index(&link)?;
        if s.platform != Platform::Ios {
            return Err(TransportError::Invalid);
        }
        s.links[i].native_activity = now;
        s.relay
            .ready(&link, true, now)
            .map_err(|_| TransportError::Stale)
    }
    /// An actual iOS notification submission returned false. Keep the same
    /// object/fragment, retain its deadline and paid credits, and await readiness.
    pub fn backpressure(
        &self,
        link: LinkHandle,
        token: u64,
        now: u64,
    ) -> Result<TransportEffects, TransportError> {
        let mut s = self.state.lock().map_err(|_| TransportError::Unavailable)?;
        s.clock(now)?;
        let i = s.index(&link)?;
        if s.platform != Platform::Ios {
            return Err(TransportError::Invalid);
        }
        let pending = s.links[i].pending.as_ref().ok_or(TransportError::Stale)?;
        if pending.token != token {
            return Err(TransportError::Stale);
        }
        let attempt = pending.attempt;
        let cookie = pending.cookie;
        let mut events = Vec::new();
        let result = s.relay.backpressure(attempt, now, &mut |e| events.push(e));
        s.links[i].pending = None;
        let mut out = TransportEffects::default();
        let expired = events
            .iter()
            .any(|e| e.link == link && e.cookie == cookie && e.status == relay::Status::Expired);
        s.results(events, &mut out);
        if result.is_err() && !expired {
            s.close(&link, &mut out)?;
        }
        Ok(out)
    }
    /// Report actual submission refusal or callback completion exactly once.
    /// A missing/ambiguous callback closes the connection instead of retrying
    /// on an operation whose native completion could still arrive later.
    pub fn complete(
        &self,
        link: LinkHandle,
        token: u64,
        success: bool,
        now: u64,
    ) -> Result<TransportEffects, TransportError> {
        let mut s = self.state.lock().map_err(|_| TransportError::Unavailable)?;
        s.clock(now)?;
        let i = s.index(&link)?;
        let pending = s.links[i].pending.as_ref().ok_or(TransportError::Stale)?;
        if pending.token != token {
            return Err(TransportError::Stale);
        }
        let mut out = TransportEffects::default();
        if s.platform == Platform::Android && now >= pending.started + 5_000 {
            s.close(&link, &mut out)?;
            return Ok(out);
        }
        let pending = s.links[i].pending.take().unwrap();
        let mut events = Vec::new();
        if s.relay
            .complete(pending.attempt, success, now, &mut |e| events.push(e))
            .is_err()
        {
            // Native submission can cross the absolute object deadline. Retain
            // the scheduler's terminal event rather than losing it as Stale.
            let expired = events.iter().any(|e| {
                e.link == link && e.cookie == pending.cookie && e.status == relay::Status::Expired
            });
            s.results(events, &mut out);
            if !expired {
                s.close(&link, &mut out)?;
            }
            return Ok(out);
        }
        if success && pending.cookie == 1 {
            if s.friends.hello_transmitted(&link, now).is_err() {
                s.close(&link, &mut out)?;
            } else {
                s.admission_event(i, &mut out)?;
            }
        } else if success && pending.cookie == 2 {
            let proof = s.links[i].proof.ok_or(TransportError::Stale)?;
            if s.friends.proof_transmitted(&link, &proof, now).is_err() {
                s.close(&link, &mut out)?;
            }
        } else if !success
            && events
                .iter()
                .any(|e| e.cookie <= 2 && e.status == relay::Status::NativeFailed)
        {
            s.close(&link, &mut out)?;
        }
        s.results(events, &mut out);
        s.consolidate(&mut out)?;
        Ok(out)
    }
    /// Any capacity change, permission loss, radio disable, lock or native
    /// disconnect tears down the generation. Reconnect needs fresh admission.
    pub fn disconnect(
        &self,
        link: LinkHandle,
        now: u64,
    ) -> Result<TransportEffects, TransportError> {
        let mut s = self.state.lock().map_err(|_| TransportError::Unavailable)?;
        s.clock(now)?;
        let mut out = TransportEffects::default();
        s.close(&link, &mut out)?;
        Ok(out)
    }
}

impl NativeTransport {
    fn create(
        store: Arc<EncryptedStore>,
        identity: PublicIdentity,
        instance_nonce: u64,
        monotonic_ms: u64,
        platform: Platform,
    ) -> Result<Self, TransportError> {
        let limit = power::parameters(platform, power::Mode::Normal, false, false).max_links;
        let ingress = Ingress::new(instance_nonce, limit, monotonic_ms)
            .map_err(|_| TransportError::Invalid)?;
        let friends =
            Friends::open(&store, &identity, monotonic_ms).map_err(|_| TransportError::Identity)?;
        let relay = Relay::new(instance_nonce, platform, true, monotonic_ms)
            .map_err(|_| TransportError::Invalid)?;
        Ok(Self {
            state: Mutex::new(Runtime {
                instance: instance_nonce,
                serial: 0,
                now: monotonic_ms,
                ingress,
                friends,
                messaging: messaging::Messaging::new(identity)?,
                relay,
                links: Vec::with_capacity(limit),
                admissions: Vec::with_capacity(limit),
                failed: false,
                power: power::Policy::new(platform, monotonic_ms),
                platform,
                link_limit: limit,
                recent: Vec::with_capacity(200),
                beacon: beacon::Beacon::new(instance_nonce, monotonic_ms)?,
            }),
        })
    }
}
