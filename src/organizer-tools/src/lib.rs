//! Offline issuance only. No network, radio, authority adoption or secret logging.
use chacha20poly1305::{
    ChaCha20Poly1305, KeyInit,
    aead::{Aead, Payload},
};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use meshchat_core::{
    codec,
    links::{self, UnconfirmedProposal},
    text,
};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

const VAULT: &[u8] = b"meshchat/offline-root/v1\0";
#[derive(Debug, Clone, Copy)]
pub struct Invalid;
impl std::fmt::Display for Invalid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Invalid input or unavailable offline operation")
    }
}
impl std::error::Error for Invalid {}
type Result<T> = std::result::Result<T, Invalid>;

pub fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut out, b| {
        write!(out, "{b:02x}").unwrap();
        out
    })
}
pub fn unhex(raw: &str) -> Result<Zeroizing<Vec<u8>>> {
    if raw.len() > 4096 || raw.len() % 2 != 0 || !raw.is_ascii() {
        return Err(Invalid);
    }
    raw.as_bytes()
        .chunks_exact(2)
        .map(|p| {
            u8::from_str_radix(std::str::from_utf8(p).map_err(|_| Invalid)?, 16)
                .map_err(|_| Invalid)
        })
        .collect::<Result<Vec<_>>>()
        .map(Zeroizing::new)
}
pub fn base64url(raw: &[u8]) -> Zeroizing<String> {
    const ABC: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = Zeroizing::new(String::new());
    let (mut acc, mut bits) = (0u32, 0);
    for b in raw {
        acc = (acc << 8) | u32::from(*b);
        bits += 8;
        while bits >= 6 {
            bits -= 6;
            out.push(ABC[((acc >> bits) & 63) as usize] as char);
        }
    }
    if bits > 0 {
        out.push(ABC[((acc << (6 - bits)) & 63) as usize] as char);
    }
    out
}
fn segment(raw: &str) -> String {
    raw.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
                char::from(b).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}
fn random<const N: usize>() -> Result<Zeroizing<[u8; N]>> {
    let mut out = Zeroizing::new([0; N]);
    getrandom::fill(out.as_mut()).map_err(|_| Invalid)?;
    Ok(out)
}
fn id(key: &[u8]) -> [u8; 8] {
    Sha256::digest(key)[..8].try_into().unwrap()
}
fn verify(key: &[u8], message: &[u8], signature: &[u8]) -> bool {
    // Match the core's canonical compressed-coordinate boundary as well as
    // dalek's strict signature check; offline validation grants no authority.
    fn canonical(raw: &[u8]) -> bool {
        let Ok(mut value) = <[u8; 32]>::try_from(raw) else {
            return false;
        };
        value[31] &= 127;
        let mut prime = [255u8; 32];
        prime[0] = 237;
        prime[31] = 127;
        value.iter().rev().cmp(prime.iter().rev()).is_lt()
    }
    if !canonical(key) || signature.len() != 64 || !canonical(&signature[..32]) {
        return false;
    }
    let Ok(key) = <[u8; 32]>::try_from(key) else {
        return false;
    };
    let Ok(key) = VerifyingKey::from_bytes(&key) else {
        return false;
    };
    let Ok(signature) = Signature::from_slice(signature) else {
        return false;
    };
    !key.is_weak() && key.verify_strict(message, &signature).is_ok()
}

pub struct CreatedRoot {
    pub vault: String,
    pub unlock: Zeroizing<String>,
    pub event: String,
}
/// The randomly generated 256-bit unlock code is returned once, never persisted.
/// It is a key, not a password: no human passphrase is accepted by this format.
pub fn create_root(name: &str, expiry: u32, event_end: u32, now: u32) -> Result<CreatedRoot> {
    check_root(name, expiry, event_end, now)?;
    let seed = random::<32>()?;
    let key = random::<32>()?;
    let nonce = random::<12>()?;
    let mut plain = Zeroizing::new(seed.to_vec());
    plain.extend_from_slice(&expiry.to_be_bytes());
    plain.extend_from_slice(&event_end.to_be_bytes());
    plain.extend_from_slice(name.as_bytes());
    let cipher = ChaCha20Poly1305::new_from_slice(key.as_ref()).map_err(|_| Invalid)?;
    let encrypted = cipher
        .encrypt(
            (&*nonce).into(),
            Payload {
                msg: &plain,
                aad: VAULT,
            },
        )
        .map_err(|_| Invalid)?;
    let mut vault = VAULT.to_vec();
    vault.extend_from_slice(nonce.as_ref());
    vault.extend_from_slice(&encrypted);
    Ok(CreatedRoot {
        vault: hex(&vault),
        unlock: Zeroizing::new(hex(key.as_ref())),
        event: event_uri(&seed, name, expiry)?,
    })
}
pub fn check_root(name: &str, expiry: u32, event_end: u32, now: u32) -> Result<()> {
    text::validate(name, text::Kind::EventName).map_err(|_| Invalid)?;
    if expiry <= now || event_end == 0 || u64::from(expiry) > u64::from(event_end) + 86_400 {
        return Err(Invalid);
    }
    Ok(())
}
fn event_uri(seed: &[u8; 32], name: &str, expiry: u32) -> Result<String> {
    let key = SigningKey::from_bytes(seed);
    let mut bundle = vec![1];
    bundle.extend_from_slice(key.verifying_key().as_bytes());
    bundle.extend_from_slice(&expiry.to_be_bytes());
    let mut signed = b"meshfest/event-root/v1\0".to_vec();
    signed.extend_from_slice(&bundle);
    bundle.extend_from_slice(&key.sign(&signed).to_bytes());
    let uri = format!(
        "meshfest://event/{}/{}",
        &*base64url(&bundle),
        segment(name)
    );
    links::parse(&uri).map_err(|_| Invalid)?;
    Ok(uri)
}
pub struct OpenRoot {
    seed: Zeroizing<[u8; 32]>,
    pub name: String,
    pub expiry: u32,
    pub event_end: u32,
}
impl OpenRoot {
    pub fn open(vault: &str, unlock: &str, now: u32) -> Result<Self> {
        let key = unhex(unlock)?;
        if key.len() != 32 {
            return Err(Invalid);
        }
        let raw = unhex(vault)?;
        if raw.len() < VAULT.len() + 12 + 16 + 41 || !raw.starts_with(VAULT) {
            return Err(Invalid);
        }
        let cipher = ChaCha20Poly1305::new_from_slice(&key).map_err(|_| Invalid)?;
        let nonce: &[u8; 12] = raw[VAULT.len()..VAULT.len() + 12]
            .try_into()
            .map_err(|_| Invalid)?;
        let plain = Zeroizing::new(
            cipher
                .decrypt(
                    nonce.into(),
                    Payload {
                        msg: &raw[VAULT.len() + 12..],
                        aad: VAULT,
                    },
                )
                .map_err(|_| Invalid)?,
        );
        if plain.len() < 41 {
            return Err(Invalid);
        }
        let expiry = u32::from_be_bytes(plain[32..36].try_into().map_err(|_| Invalid)?);
        let event_end = u32::from_be_bytes(plain[36..40].try_into().map_err(|_| Invalid)?);
        let name = std::str::from_utf8(&plain[40..])
            .map_err(|_| Invalid)?
            .to_owned();
        check_root(&name, expiry, event_end, now)?;
        Ok(Self {
            seed: Zeroizing::new(plain[..32].try_into().map_err(|_| Invalid)?),
            name,
            expiry,
            event_end,
        })
    }
    pub fn event(&self) -> Result<String> {
        event_uri(&self.seed, &self.name, self.expiry)
    }
    pub fn check_staff(&self, label: &str, before: u32, after: u32, now: u32) -> Result<()> {
        text::validate(label, text::Kind::CredentialLabel).map_err(|_| Invalid)?;
        if label.trim().is_empty()
            || before > after
            || after <= now
            || after > self.expiry
            || u64::from(after) > u64::from(self.event_end) + 86_400
            || self.expiry <= now
        {
            return Err(Invalid);
        }
        Ok(())
    }
    pub fn issue(
        &self,
        label: &str,
        before: u32,
        after: u32,
        now: u32,
    ) -> Result<Zeroizing<String>> {
        self.check_staff(label, before, after, now)?;
        let root = SigningKey::from_bytes(&self.seed);
        let seed = random::<32>()?;
        let staff = SigningKey::from_bytes(&seed);
        let mut credential = vec![1];
        credential.extend_from_slice(&id(root.verifying_key().as_bytes()));
        credential.extend_from_slice(staff.verifying_key().as_bytes());
        credential.extend_from_slice(&before.to_be_bytes());
        credential.extend_from_slice(&after.to_be_bytes());
        credential.push(label.len() as u8);
        credential.extend_from_slice(label.as_bytes());
        let mut signed = b"meshfest/credential/v1\0".to_vec();
        signed.extend_from_slice(&(credential.len() as u16).to_be_bytes());
        signed.extend_from_slice(&credential);
        credential.extend_from_slice(&root.sign(&signed).to_bytes());
        let uri = Zeroizing::new(format!(
            "meshfest://staff/{}/{}",
            &*base64url(&credential),
            &*base64url(seed.as_ref())
        ));
        // A scheduled shift is checked at its first valid second, never extended.
        validate_staff(&self.event()?, &uri, now.max(before))?;
        Ok(uri)
    }
}
pub fn validate_event(uri: &str, now: u32) -> Result<([u8; 101], String)> {
    let UnconfirmedProposal::Event { bundle, name, .. } = links::parse(uri).map_err(|_| Invalid)?
    else {
        return Err(Invalid);
    };
    let expiry = u32::from_be_bytes(bundle[33..37].try_into().unwrap());
    if expiry < now {
        return Err(Invalid);
    }
    let mut signed = b"meshfest/event-root/v1\0".to_vec();
    signed.extend_from_slice(&bundle[..37]);
    if !verify(&bundle[1..33], &signed, &bundle[37..]) {
        return Err(Invalid);
    }
    Ok((bundle, name))
}
/// Offline validation is not mobile adoption; the receiving phone still confirms
/// and verifies the canonical proposal through its protected organizer owner.
pub fn validate_staff(event: &str, staff: &str, now: u32) -> Result<()> {
    let (bundle, _) = validate_event(event, now)?;
    let UnconfirmedProposal::Staff(proposal) = links::parse(staff).map_err(|_| Invalid)? else {
        return Err(Invalid);
    };
    let (raw, seed) = proposal.into_import_parts();
    let c = codec::credential(&raw).map_err(|_| Invalid)?;
    if c.root_id != id(&bundle[1..33])
        || c.not_before > now
        || c.not_after < now
        || c.not_after > u32::from_be_bytes(bundle[33..37].try_into().unwrap())
        || SigningKey::from_bytes(&seed).verifying_key().as_bytes() != c.staff_public_key
    {
        return Err(Invalid);
    }
    let mut signed = b"meshfest/credential/v1\0".to_vec();
    signed.extend_from_slice(&((raw.len() - 64) as u16).to_be_bytes());
    signed.extend_from_slice(&raw[..raw.len() - 64]);
    if !verify(&bundle[1..33], &signed, c.root_signature) {
        return Err(Invalid);
    }
    Ok(())
}
/// Matrix only; private QR images are never written by this library.
pub fn matrix(uri: &str) -> Result<Vec<String>> {
    let qr = qrcode::QrCode::new(uri.as_bytes()).map_err(|_| Invalid)?;
    let w = qr.width();
    Ok(qr
        .to_colors()
        .chunks(w)
        .map(|row| {
            row.iter()
                .map(|p| if *p == qrcode::Color::Dark { '1' } else { '0' })
                .collect()
        })
        .collect())
}
