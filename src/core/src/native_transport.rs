//! Native GATT lifecycle around the existing handshake, intake and scheduler.
//! Transport admission never grants display, identity or delivery authority.
use crate::{
    LinkHandle, SendPath, framing,
    friends::{Friends, Role},
    identity::{IdentityKeySession, PublicIdentity},
    ingress::{Ingress, Outcome, State},
    power::Platform,
    relay::{self, Relay},
    storage::EncryptedStore,
};
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
struct PendingSend {
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
    relay: Relay,
    links: Vec<Link>,
    admissions: Vec<Admission>,
    failed: bool,
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
        if self.failed || now < self.now || now > u64::MAX - 900_000 {
            self.failed = true;
            self.friends.clear_session_trust();
            return Err(TransportError::Unavailable);
        }
        self.now = now;
        self.admissions.retain(|a| now < a.born + 30_000);
        self.ingress
            .advance(now)
            .map_err(|_| TransportError::Unavailable)
    }
    fn results(events: Vec<relay::ResultEvent>, out: &mut TransportEffects) {
        for event in events {
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
        self.links.remove(index);
        Self::results(events, out);
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
            link.admitted = true;
            out.events.push(TransportEvent::Admitted {
                link: link.handle.clone(),
                transmit_bytes: tx,
                receive_bytes: rx,
            });
        }
        Ok(())
    }
    fn enqueue(
        &mut self,
        link: &LinkHandle,
        bytes: &[u8],
        traffic: relay::Traffic,
        cookie: u64,
        out: &mut TransportEffects,
    ) -> Result<(), TransportError> {
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
        Self::results(events, out);
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
        let ingress =
            Ingress::new(instance_nonce, 6, monotonic_ms).map_err(|_| TransportError::Invalid)?;
        let friends =
            Friends::open(&store, &identity, monotonic_ms).map_err(|_| TransportError::Identity)?;
        let relay = Relay::new(instance_nonce, Platform::Android, true, monotonic_ms)
            .map_err(|_| TransportError::Invalid)?;
        Ok(Self {
            state: Mutex::new(Runtime {
                instance: instance_nonce,
                serial: 0,
                now: monotonic_ms,
                ingress,
                friends,
                relay,
                links: Vec::with_capacity(6),
                admissions: Vec::with_capacity(6),
                failed: false,
            }),
        })
    }
    /// Reserve BEFORE initiating a central connection or accepting a peripheral.
    /// Address is a process-local 16-byte digest, not an authenticated identity.
    pub fn admit_connection(&self, address: Vec<u8>, now: u64) -> Result<u64, TransportError> {
        let address = address.try_into().map_err(|_| TransportError::Invalid)?;
        let mut s = self.state.lock().map_err(|_| TransportError::Unavailable)?;
        s.clock(now)?;
        if s.links.len() + s.admissions.len() >= 6
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
        });
        let mut out = TransportEffects::default();
        if let Err(error) = s.enqueue(&link, &hello, relay::Traffic::Transport(2), 1, &mut out) {
            s.close(&link, &mut out)?;
            return Err(error);
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
            let result = ingress
                .receive_friend(
                    &link,
                    (reported_bytes, &bytes),
                    now,
                    &mut output,
                    (false, capacity),
                )
                .map_err(|_| TransportError::Stale)?;
            if let Outcome::Complete { len, state, .. } = result {
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
                    || l.pending.as_ref().is_some_and(|p| now >= p.started + 5_000)
            })
            .map(|l| l.handle.clone())
            .collect();
        for link in expired {
            s.close(&link, &mut out)?;
        }
        for _ in 0..6 {
            let mut raw = [0; 512];
            let mut events = Vec::new();
            let send = s
                .relay
                .poll(now, &mut raw, &mut |e| events.push(e))
                .map_err(|_| TransportError::Unavailable)?;
            Runtime::results(events, &mut out);
            let Some(send) = send else {
                break;
            };
            let index = s.index(&send.link)?;
            let token = s.next()?;
            let link = &mut s.links[index];
            link.pending = Some(PendingSend {
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
        if now >= pending.started + 5_000 {
            s.close(&link, &mut out)?;
            return Ok(out);
        }
        let pending = s.links[i].pending.take().unwrap();
        let mut events = Vec::new();
        s.relay
            .complete(pending.attempt, success, now, &mut |e| events.push(e))
            .map_err(|_| TransportError::Stale)?;
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
        Runtime::results(events, &mut out);
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
