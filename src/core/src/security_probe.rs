//! Test-only software curve operations. Seeds are application-memory material,
//! never represented as non-exportable hardware keys or production identities.
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use zeroize::{Zeroize, Zeroizing};

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum ProbeError {
    #[error("invalid probe input")]
    InvalidInput,
    #[error("operating system randomness unavailable")]
    RandomUnavailable,
    #[error("non-contributory peer key")]
    InvalidPeer,
}

fn seed(mut bytes: Vec<u8>) -> Result<Zeroizing<[u8; 32]>, ProbeError> {
    let result = <[u8; 32]>::try_from(bytes.as_slice())
        .map(Zeroizing::new)
        .map_err(|_| ProbeError::InvalidInput);
    bytes.zeroize();
    result
}

/// Native wrappers encrypt this test-only material before persisting it:
/// Ed25519 seed, X25519 seed, and database key, each 32 bytes.
/// The FFI copies are not guaranteed to be zeroized; never use real identities.
#[uniffi::export]
pub fn probe_random_material() -> Result<Vec<u8>, ProbeError> {
    let mut bytes = Zeroizing::new([0_u8; 96]);
    getrandom::fill(bytes.as_mut()).map_err(|_| ProbeError::RandomUnavailable)?;
    Ok(bytes.to_vec())
}

#[uniffi::export]
pub fn probe_signing_public(secret: Vec<u8>) -> Result<Vec<u8>, ProbeError> {
    let bytes = seed(secret)?;
    Ok(SigningKey::from_bytes(&bytes)
        .verifying_key()
        .to_bytes()
        .to_vec())
}

#[uniffi::export]
pub fn probe_sign(secret: Vec<u8>, message: Vec<u8>) -> Result<Vec<u8>, ProbeError> {
    let bytes = seed(secret)?;
    if message.len() > 1024 {
        return Err(ProbeError::InvalidInput);
    }
    Ok(SigningKey::from_bytes(&bytes)
        .sign(&message)
        .to_bytes()
        .to_vec())
}

#[uniffi::export]
pub fn probe_verify(public: Vec<u8>, message: Vec<u8>, signature: Vec<u8>) -> bool {
    if message.len() > 1024 {
        return false;
    }
    let Ok(public) = <[u8; 32]>::try_from(public.as_slice()) else {
        return false;
    };
    let Ok(key) = VerifyingKey::from_bytes(&public) else {
        return false;
    };
    let Ok(signature) = Signature::from_slice(&signature) else {
        return false;
    };
    key.verify_strict(&message, &signature).is_ok()
}

#[uniffi::export]
pub fn probe_agreement_public(secret: Vec<u8>) -> Result<Vec<u8>, ProbeError> {
    let bytes = seed(secret)?;
    let key = x25519_dalek::StaticSecret::from(*bytes);
    Ok(x25519_dalek::PublicKey::from(&key).as_bytes().to_vec())
}

/// Compare to a native provider's result without exporting the core's secret.
#[uniffi::export]
pub fn probe_agreement_matches(
    secret: Vec<u8>,
    peer: Vec<u8>,
    expected: Vec<u8>,
) -> Result<bool, ProbeError> {
    let bytes = seed(secret)?;
    let expected = seed(expected)?;
    let peer = <[u8; 32]>::try_from(peer.as_slice()).map_err(|_| ProbeError::InvalidInput)?;
    let key = x25519_dalek::StaticSecret::from(*bytes);
    let shared = key.diffie_hellman(&x25519_dalek::PublicKey::from(peer));
    if !shared.was_contributory() {
        return Err(ProbeError::InvalidPeer);
    }
    // Both inputs are synthetic probe material, but avoid an early-exit comparison.
    Ok(shared
        .as_bytes()
        .iter()
        .zip(expected.iter())
        .fold(0_u8, |d, (a, b)| d | (a ^ b))
        == 0)
}
