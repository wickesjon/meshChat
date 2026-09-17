//! Deterministic link boundary and bounded logical codec. No radio or persistent state.
#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::sync::Mutex;

uniffi::setup_scaffolding!();

pub mod channel;
pub mod codec;
pub mod dm;
pub mod framing;
pub mod friends;
pub mod identity;
pub mod ingress;
pub mod links;
pub mod native_channels;
pub mod native_transport;
pub mod organizer;
pub mod power;
pub mod relay;
pub mod storage;
pub mod sync;
pub mod text;

// Temporary MC-005 feasibility surface; absent from normal application builds.
#[cfg(feature = "security-probe")]
pub mod security_probe;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum CoreError {
    #[error("invalid configuration")]
    InvalidConfiguration,
    #[error("link limit reached")]
    LinkLimit,
    #[error("unknown or stale link")]
    UnknownLink,
    #[error("invalid directional capacity")]
    InvalidCapacity,
    #[error("empty or oversized value")]
    InvalidValue,
    #[error("monotonic time moved backwards")]
    TimeRegression,
    #[error("generation space exhausted")]
    GenerationExhausted,
    #[error("core unavailable after internal failure")]
    Unavailable,
}

/// Boundary limits supplied by the caller; these are not MC-007 mesh budgets.
#[derive(Debug, Clone, uniffi::Record)]
pub struct Limits {
    pub max_links: u16,
    pub max_value_bytes: u16,
}

/// A random, nonzero instance nonce plus a never-reused local generation.
/// This is a lifecycle token, not peer identity or authentication.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct LinkHandle {
    pub instance_nonce: u64,
    pub generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct Capacities {
    pub write_bytes: u16,
    pub notify_bytes: u16,
    pub receive_bytes: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum SendPath {
    Write,
    Notify,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum PowerState {
    Foreground,
    Background,
    LowPower,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum DriverEvent {
    Disconnected {
        link: LinkHandle,
    },
    CapacitiesChanged {
        link: LinkHandle,
        capacities: Capacities,
    },
    InboundBytes {
        link: LinkHandle,
        bytes: Vec<u8>,
    },
    TimeAdvanced {
        monotonic_ms: u64,
    },
    PowerChanged {
        state: PowerState,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SendCommand {
    pub link: LinkHandle,
    pub path: SendPath,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum UiEvent {
    LinkConnected {
        link: LinkHandle,
    },
    LinkDisconnected {
        link: LinkHandle,
    },
    /// Diagnostic count only. Bytes have not been parsed or authenticated.
    InboundObserved {
        link: LinkHandle,
        byte_count: u16,
    },
    PowerChanged {
        state: PowerState,
    },
}

/// Each call returns at most one command or event and retains no payload queue.
#[derive(Debug, Clone, PartialEq, Eq, Default, uniffi::Record)]
pub struct Effects {
    pub sends: Vec<SendCommand>,
    pub ui_events: Vec<UiEvent>,
}

#[derive(Debug, Clone, thiserror::Error, uniffi::Error)]
pub enum KeyError {
    #[error("protected key unavailable")]
    Unavailable,
    #[error("operation rejected")]
    Rejected,
}

/// Reserved adapter boundary: opaque platform-owned handle, never private key bytes.
/// MC-005/008/017 select supported operations, algorithms and handle lifecycle.
/// No production key operation is invoked by this foundation.
#[uniffi::export(callback_interface)]
pub trait PlatformKeyProvider: Send + Sync {
    fn public_key(&self, handle: String) -> Result<Vec<u8>, KeyError>;
    fn sign(&self, handle: String, transcript: Vec<u8>) -> Result<Vec<u8>, KeyError>;
}

struct State {
    next_generation: u64,
    monotonic_ms: u64,
    power: PowerState,
    links: BTreeMap<u64, Capacities>,
}

#[derive(uniffi::Object)]
pub struct Core {
    limits: Limits,
    instance_nonce: u64,
    state: Mutex<State>,
}

impl Core {
    fn validate_capacities(&self, capacities: &Capacities) -> Result<(), CoreError> {
        if capacities.receive_bytes == 0
            || (capacities.write_bytes == 0 && capacities.notify_bytes == 0)
            || [
                capacities.write_bytes,
                capacities.notify_bytes,
                capacities.receive_bytes,
            ]
            .iter()
            .any(|size| *size > self.limits.max_value_bytes)
        {
            return Err(CoreError::InvalidCapacity);
        }
        Ok(())
    }

    fn link<'a>(&self, state: &'a State, link: &LinkHandle) -> Result<&'a Capacities, CoreError> {
        if link.instance_nonce != self.instance_nonce {
            return Err(CoreError::UnknownLink);
        }
        state
            .links
            .get(&link.generation)
            .ok_or(CoreError::UnknownLink)
    }

    fn validate_value(bytes: &[u8], capacity: u16) -> Result<(), CoreError> {
        if bytes.is_empty() || bytes.len() > usize::from(capacity) {
            return Err(CoreError::InvalidValue);
        }
        Ok(())
    }
}

#[uniffi::export]
impl Core {
    /// Inject a nonce from the platform RNG (fixed in deterministic tests) and
    /// a monotonic clock reading. Core never consults ambient time or entropy.
    #[uniffi::constructor]
    pub fn new(limits: Limits, instance_nonce: u64, monotonic_ms: u64) -> Result<Self, CoreError> {
        if limits.max_links == 0 || limits.max_value_bytes == 0 || instance_nonce == 0 {
            return Err(CoreError::InvalidConfiguration);
        }
        Ok(Self {
            limits,
            instance_nonce,
            state: Mutex::new(State {
                next_generation: 1,
                monotonic_ms,
                power: PowerState::Foreground,
                links: BTreeMap::new(),
            }),
        })
    }

    /// Driver reports one established connection; returned event contains its token.
    pub fn connected(&self, capacities: Capacities) -> Result<Effects, CoreError> {
        self.validate_capacities(&capacities)?;
        let mut state = self.state.lock().map_err(|_| CoreError::Unavailable)?;
        if state.links.len() >= usize::from(self.limits.max_links) {
            return Err(CoreError::LinkLimit);
        }
        let generation = state.next_generation;
        let next = generation
            .checked_add(1)
            .ok_or(CoreError::GenerationExhausted)?;
        state.links.insert(generation, capacities);
        state.next_generation = next;
        Ok(Effects {
            ui_events: vec![UiEvent::LinkConnected {
                link: LinkHandle {
                    instance_nonce: self.instance_nonce,
                    generation,
                },
            }],
            ..Effects::default()
        })
    }

    pub fn handle_event(&self, event: DriverEvent) -> Result<Effects, CoreError> {
        let mut state = self.state.lock().map_err(|_| CoreError::Unavailable)?;
        let mut effects = Effects::default();
        match event {
            DriverEvent::Disconnected { link } => {
                self.link(&state, &link)?;
                state.links.remove(&link.generation);
                effects.ui_events.push(UiEvent::LinkDisconnected { link });
            }
            DriverEvent::CapacitiesChanged { link, capacities } => {
                self.link(&state, &link)?;
                self.validate_capacities(&capacities)?;
                state.links.insert(link.generation, capacities);
            }
            DriverEvent::InboundBytes { link, bytes } => {
                let capacity = self.link(&state, &link)?.receive_bytes;
                Self::validate_value(&bytes, capacity)?;
                let byte_count = u16::try_from(bytes.len()).map_err(|_| CoreError::InvalidValue)?;
                effects
                    .ui_events
                    .push(UiEvent::InboundObserved { link, byte_count });
            }
            DriverEvent::TimeAdvanced { monotonic_ms } => {
                if monotonic_ms < state.monotonic_ms {
                    return Err(CoreError::TimeRegression);
                }
                state.monotonic_ms = monotonic_ms;
            }
            DriverEvent::PowerChanged { state: power } => {
                state.power = power.clone();
                effects
                    .ui_events
                    .push(UiEvent::PowerChanged { state: power });
            }
        }
        Ok(effects)
    }

    /// Admit a caller-provided value for the driver. This is not wire validation,
    /// radio delivery or a protocol echo. Drivers must discard commands after
    /// disconnect and serialize event/command dispatch with their link lifecycle.
    pub fn prepare_send(
        &self,
        link: LinkHandle,
        path: SendPath,
        bytes: Vec<u8>,
    ) -> Result<Effects, CoreError> {
        let state = self.state.lock().map_err(|_| CoreError::Unavailable)?;
        let capacities = self.link(&state, &link)?;
        let capacity = match path {
            SendPath::Write => capacities.write_bytes,
            SendPath::Notify => capacities.notify_bytes,
        };
        Self::validate_value(&bytes, capacity)?;
        Ok(Effects {
            sends: vec![SendCommand { link, path, bytes }],
            ui_events: vec![],
        })
    }
}
