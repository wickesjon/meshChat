//! HPKE stays inside the short-lived protected identity session.
use super::*;
use crate::dm::{Error, aad, info};
use hmac::{Hmac, KeyInit, Mac};
use hpke::rand_core::{TryCryptoRng, TryRng};
use hpke::{Deserializable, Kem as KemTrait, Serializable};
use std::convert::Infallible;
use subtle::ConstantTimeEq;
type Kem = hpke::kem::X25519HkdfSha256;
type Aead = hpke::aead::ChaCha20Poly1305;
type Kdf = hpke::kdf::HkdfSha256;

// The pinned KEM requests one 32-byte fill. Unexpected usage poisons all output;
// the infallible API receives dummy bytes only to finish without an entropy panic.
// No encapsulation/ciphertext from a poisoned adapter may escape.
struct OneUse {
    bytes: Zeroizing<[u8; 32]>,
    used: bool,
    failed: bool,
}
impl TryRng for OneUse {
    type Error = Infallible;
    fn try_next_u32(&mut self) -> Result<u32, Infallible> {
        self.failed = true;
        Ok(0)
    }
    fn try_next_u64(&mut self) -> Result<u64, Infallible> {
        self.failed = true;
        Ok(0)
    }
    fn try_fill_bytes(&mut self, out: &mut [u8]) -> Result<(), Infallible> {
        if self.used || out.len() != 32 {
            self.failed = true;
            out.fill(0);
        } else {
            out.copy_from_slice(self.bytes.as_ref());
            self.used = true;
            self.bytes.zeroize();
        }
        Ok(())
    }
}
impl TryCryptoRng for OneUse {}
fn key(raw: &[u8]) -> Result<<Kem as KemTrait>::PublicKey, Error> {
    let mut p = [255; 32];
    p[0] = 237;
    p[31] = 127;
    if raw.len() != 32 || !raw.iter().rev().cmp(p.iter().rev()).is_lt() {
        return Err(Error::Invalid);
    }
    <Kem as KemTrait>::PublicKey::from_bytes(raw).map_err(|_| Error::Invalid)
}
fn tag(shared: &[u8], epoch: u64) -> [u8; 4] {
    let mut mac = <Hmac<sha2_11::Sha256> as KeyInit>::new_from_slice(shared)
        .expect("HMAC permits any key length");
    mac.update(b"meshfest-dmtag-v1");
    mac.update(&epoch.to_be_bytes());
    mac.finalize().into_bytes()[..4].try_into().unwrap()
}
impl IdentityKeySession {
    pub(crate) fn dm_seal(
        &self,
        own: &[u8; 64],
        peer: &[u8; 64],
        header: &mut [u8; 26],
        plain: &[u8],
        epoch: u64,
    ) -> Result<Vec<u8>, Error> {
        self.dm_seal_with_entropy(own, peer, header, plain, epoch, |seed| {
            getrandom::fill(seed).map_err(|_| Error::Entropy)
        })
    }
    #[allow(clippy::too_many_arguments)]
    fn dm_seal_with_entropy(
        &self,
        own: &[u8; 64],
        peer: &[u8; 64],
        header: &mut [u8; 26],
        plain: &[u8],
        epoch: u64,
        entropy: impl FnOnce(&mut [u8; 32]) -> Result<(), Error>,
    ) -> Result<Vec<u8>, Error> {
        let mut seed = Zeroizing::new([0; 32]);
        entropy(&mut seed)?;
        self.dm_seal_buffered(own, peer, header, plain, epoch, seed)
    }
    fn dm_seal_buffered(
        &self,
        own: &[u8; 64],
        peer: &[u8; 64],
        header: &mut [u8; 26],
        plain: &[u8],
        epoch: u64,
        seed: Zeroizing<[u8; 32]>,
    ) -> Result<Vec<u8>, Error> {
        let recipient = key(&peer[32..])?;
        let public = key(&own[32..])?;
        let lock = self.material.lock().map_err(|_| Error::Provider)?;
        let material = lock.as_ref().ok_or(Error::Provider)?;
        let shared = agreement(material).diffie_hellman(&x25519_dalek::PublicKey::from(
            <[u8; 32]>::try_from(&peer[32..]).unwrap(),
        ));
        if !shared.was_contributory() {
            return Err(Error::Invalid);
        }
        header[20..24].copy_from_slice(&tag(shared.as_bytes(), epoch));
        let secret = <Kem as KemTrait>::PrivateKey::from_bytes(&material[32..])
            .map_err(|_| Error::Provider)?;
        let mut rng = OneUse {
            bytes: seed,
            used: false,
            failed: false,
        };
        let setup = hpke::setup_sender_with_rng::<Aead, Kdf, Kem>(
            &hpke::OpModeS::Auth((secret, public)),
            &recipient,
            &info(own, peer),
            &mut rng,
        );
        if !rng.used || rng.failed {
            return Err(Error::Entropy);
        }
        let (enc, mut context) = setup.map_err(|_| Error::Authentication)?;
        let mut raw = header.to_vec();
        raw.push(1);
        raw.extend_from_slice(&Sha256::digest(&own[32..])[..8]);
        raw.extend_from_slice(&enc.to_bytes());
        let ciphertext = context
            .seal(plain, &aad(&raw))
            .map_err(|_| Error::Authentication)?;
        raw.extend_from_slice(&ciphertext);
        Ok(raw)
    }
    pub(crate) fn dm_open(
        &self,
        own: &[u8; 64],
        peer: &[u8; 64],
        raw: &[u8],
        epoch: u64,
    ) -> Result<Zeroizing<Vec<u8>>, Error> {
        let sender = key(&peer[32..])?;
        key(&raw[35..67])?;
        let enc =
            <Kem as KemTrait>::EncappedKey::from_bytes(&raw[35..67]).map_err(|_| Error::Invalid)?;
        let lock = self.material.lock().map_err(|_| Error::Provider)?;
        let material = lock.as_ref().ok_or(Error::Provider)?;
        let shared = agreement(material).diffie_hellman(&x25519_dalek::PublicKey::from(
            <[u8; 32]>::try_from(&peer[32..]).unwrap(),
        ));
        if !shared.was_contributory() {
            return Err(Error::Invalid);
        }
        let mut matched = subtle::Choice::from(0);
        for candidate in [epoch.checked_sub(1), Some(epoch), epoch.checked_add(1)]
            .into_iter()
            .flatten()
        {
            matched |= tag(shared.as_bytes(), candidate).ct_eq(&raw[20..24]);
        }
        if !bool::from(matched) {
            return Err(Error::Tag);
        }
        let secret = <Kem as KemTrait>::PrivateKey::from_bytes(&material[32..])
            .map_err(|_| Error::Provider)?;
        let mut context = hpke::setup_receiver::<Aead, Kdf, Kem>(
            &hpke::OpModeR::Auth(sender),
            &secret,
            &enc,
            &info(peer, own),
        )
        .map_err(|_| Error::Authentication)?;
        // Supply our own zeroizing output so partial plaintext is cleared even
        // when authentication fails; the allocating convenience API does not.
        let ciphertext = &raw[67..raw.len() - 16];
        let auth_tag = hpke::aead::AeadTag::<Aead>::from_bytes(&raw[raw.len() - 16..])
            .map_err(|_| Error::Invalid)?;
        let mut plain = Zeroizing::new(vec![0; ciphertext.len()]);
        context
            .open_inout_detached(
                hpke::inout::InOutBuf::new(ciphertext, plain.as_mut_slice())
                    .map_err(|_| Error::Invalid)?,
                &aad(&raw[..67]),
                &auth_tag,
            )
            .map_err(|_| Error::Authentication)?;
        Ok(plain)
    }
}
#[cfg(test)]
#[path = "../../../../tests/integration/dm/provider.rs"]
mod tests;
