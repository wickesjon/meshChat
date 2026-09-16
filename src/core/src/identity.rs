//! Short-lived software curve session inside the native protected-key provider.
//! Native adapters alone unwrap/import material and invalidate each session in
//! a finally/defer block. The application receives public metadata and opaque
//! native identity handles, never this session or an exportable private seed.
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};
use zeroize::{Zeroize, Zeroizing};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum IdentityError {
    #[error("invalid identity input")]
    InvalidInput,
    #[error("identity key session unavailable")]
    Unavailable,
    #[error("invalid agreement peer")]
    InvalidPeer,
}
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PublicIdentity {
    pub generation: Vec<u8>,
    pub signing_key: Vec<u8>,
    pub agreement_key: Vec<u8>,
    pub sender_id: Vec<u8>,
    pub agreement_hint: Vec<u8>,
}
#[derive(uniffi::Object)]
pub struct IdentityKeySession {
    material: Mutex<Option<Zeroizing<[u8; 64]>>>,
    generation: [u8; 16],
}
fn signing(material: &[u8; 64]) -> SigningKey {
    let seed = Zeroizing::new(<[u8; 32]>::try_from(&material[..32]).unwrap());
    SigningKey::from_bytes(&seed)
}
fn agreement(material: &[u8; 64]) -> x25519_dalek::StaticSecret {
    let seed = Zeroizing::new(<[u8; 32]>::try_from(&material[32..]).unwrap());
    x25519_dalek::StaticSecret::from(*seed)
}
#[uniffi::export]
impl IdentityKeySession {
    /// Trusted native-provider ingress, not a seed-export operation. The native
    /// provider creates/unwraps this temporary material under its lock policy;
    /// application-facing operations accept only its opaque identity handle.
    /// UniFFI/JVM/Swift may leave unavoidable transient copies; no guarantee of
    /// hardware curve execution or complete runtime memory erasure is made.
    #[uniffi::constructor]
    pub fn import_unlocked(
        mut material: Vec<u8>,
        generation: Vec<u8>,
    ) -> Result<Arc<Self>, IdentityError> {
        let generation = <[u8; 16]>::try_from(generation.as_slice());
        let copied = <[u8; 64]>::try_from(material.as_slice()).map(Zeroizing::new);
        material.zeroize();
        let generation = generation.map_err(|_| IdentityError::InvalidInput)?;
        if generation == [0; 16] {
            return Err(IdentityError::InvalidInput);
        }
        Ok(Arc::new(Self {
            material: Mutex::new(Some(copied.map_err(|_| IdentityError::InvalidInput)?)),
            generation,
        }))
    }
    pub fn public_identity(&self) -> Result<PublicIdentity, IdentityError> {
        let lock = self
            .material
            .lock()
            .map_err(|_| IdentityError::Unavailable)?;
        let material = lock.as_ref().ok_or(IdentityError::Unavailable)?;
        let ed = signing(material).verifying_key().to_bytes();
        let x = x25519_dalek::PublicKey::from(&agreement(material)).to_bytes();
        Ok(PublicIdentity {
            generation: self.generation.to_vec(),
            signing_key: ed.to_vec(),
            agreement_key: x.to_vec(),
            sender_id: Sha256::digest(ed)[..8].to_vec(),
            agreement_hint: Sha256::digest(x)[..8].to_vec(),
        })
    }
    /// Bounded primitive. Owning protocol code supplies the MC-008 transcript
    /// and reserves actual work units before invoking the native provider.
    pub fn sign(&self, message: Vec<u8>) -> Result<Vec<u8>, IdentityError> {
        let message = Zeroizing::new(message);
        if message.len() > 2048 {
            return Err(IdentityError::InvalidInput);
        }
        let lock = self
            .material
            .lock()
            .map_err(|_| IdentityError::Unavailable)?;
        let material = lock.as_ref().ok_or(IdentityError::Unavailable)?;
        Ok(signing(material).sign(&message).to_bytes().to_vec())
    }
    pub fn agree(&self, peer: Vec<u8>) -> Result<Vec<u8>, IdentityError> {
        let peer =
            <[u8; 32]>::try_from(peer.as_slice()).map_err(|_| IdentityError::InvalidInput)?;
        // MC-008 requires canonical little-endian u < 2^255-19, without
        // accepting alternate encodings via RFC7748 masking/reduction.
        let mut prime = [255; 32];
        prime[0] = 237;
        prime[31] = 127;
        if peer.iter().rev().cmp(prime.iter().rev()) != std::cmp::Ordering::Less {
            return Err(IdentityError::InvalidPeer);
        }
        let lock = self
            .material
            .lock()
            .map_err(|_| IdentityError::Unavailable)?;
        let material = lock.as_ref().ok_or(IdentityError::Unavailable)?;
        let shared = agreement(material).diffie_hellman(&x25519_dalek::PublicKey::from(peer));
        if !shared.was_contributory() {
            return Err(IdentityError::InvalidPeer);
        }
        Ok(shared.as_bytes().to_vec())
    }
    /// Explicitly clears Rust-owned seed storage even if a native wrapper keeps
    /// its generated object reference alive. Idempotent; all later uses refuse.
    pub fn invalidate(&self) -> Result<(), IdentityError> {
        self.material
            .lock()
            .map_err(|_| IdentityError::Unavailable)?
            .take();
        Ok(())
    }
}

impl IdentityKeySession {
    pub(crate) fn matches_generation(&self, generation: &[u8; 16]) -> bool {
        self.generation == *generation && self.material.lock().is_ok_and(|m| m.is_some())
    }
}
