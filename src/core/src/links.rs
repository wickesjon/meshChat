//! MC-006 URI grammar. Results are inert, unconfirmed proposals, not authority.
//! Native callers must display confirmation, then invoke the relevant validated
//! join/pin/root/staff flow. This module cannot mutate subscriptions or trust.
use crate::{channel, codec, text};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("invalid link grammar or encoding")]
    Invalid,
    #[error("unsupported bundle version or newer channel words")]
    UpdateNeeded,
    #[error("invalid display text")]
    Text,
}

// Deliberately no Debug/Clone/serialization for proposals containing seed material.
pub enum UnconfirmedProposal {
    Channel(channel::Channel),
    Friend {
        bundle: [u8; 65],
        nickname: String,
    },
    Event {
        bundle: [u8; 101],
        name: String,
        root_id: [u8; 8],
    },
    Staff(StaffProposal),
}
pub struct StaffProposal {
    credential: Vec<u8>,
    seed: Zeroizing<[u8; 32]>,
}
impl StaffProposal {
    pub fn credential(&self) -> &[u8] {
        &self.credential
    }
    /// Consuming transfer to a future confirmation-gated protected importer.
    /// Returned seed storage is zeroizing; never log/serialize its contents.
    pub fn into_import_parts(self) -> (Vec<u8>, Zeroizing<[u8; 32]>) {
        (self.credential, self.seed)
    }
}

/// Input is caller-owned: the native caller must also clear any seed-bearing QR
/// or URL buffers after use. Error values never contain any portion of the URI.
pub fn parse(raw: &str) -> Result<UnconfirmedProposal, Error> {
    if raw.is_empty()
        || raw.len() > 2048
        || !raw.is_ascii()
        || raw
            .bytes()
            .any(|b| b <= 32 || b == 127 || matches!(b, b'?' | b'#'))
    {
        return Err(Error::Invalid);
    }
    let (scheme, rest) = raw.split_once("://").ok_or(Error::Invalid)?;
    let (authority, path) = rest.split_once('/').ok_or(Error::Invalid)?;
    let mut segments = path.split('/');
    let route = if scheme.eq_ignore_ascii_case("meshfest") {
        if authority.eq_ignore_ascii_case("j") {
            "j"
        } else if authority.eq_ignore_ascii_case("friend") {
            "friend"
        } else if authority.eq_ignore_ascii_case("event") {
            "event"
        } else if authority.eq_ignore_ascii_case("staff") {
            "staff"
        } else {
            return Err(Error::Invalid);
        }
    } else if scheme.eq_ignore_ascii_case("https") && authority.eq_ignore_ascii_case("meshfest.app")
    {
        match segments.next() {
            Some("j") => "j",
            Some("friend") => "friend",
            Some("event") => "event",
            _ => return Err(Error::Invalid),
        }
    } else {
        return Err(Error::Invalid);
    };
    let first = segments
        .next()
        .filter(|s| !s.is_empty())
        .ok_or(Error::Invalid)?;
    if route == "j" {
        if segments.next().is_some() {
            return Err(Error::Invalid);
        }
        let mut words = first.split('-');
        let triple = [
            words.next().ok_or(Error::Invalid)?,
            words.next().ok_or(Error::Invalid)?,
            words.next().ok_or(Error::Invalid)?,
        ];
        if words.next().is_some() {
            return Err(Error::Invalid);
        }
        return channel::private(triple)
            .map(UnconfirmedProposal::Channel)
            .map_err(|e| match e {
                channel::Error::UpdateNeeded => Error::UpdateNeeded,
                _ => Error::Invalid,
            });
    }
    let second = segments
        .next()
        .filter(|s| !s.is_empty())
        .ok_or(Error::Invalid)?;
    if segments.next().is_some() {
        return Err(Error::Invalid);
    }
    // Fixed decoded scratch space, zeroized on every success/error path.
    let mut binary = Zeroizing::new([0u8; 130]);
    let len = decode_binary(first, &mut *binary)?;
    match route {
        "friend" if len == 65 => {
            if binary[0] != 1 {
                return Err(Error::UpdateNeeded);
            }
            let nickname = display_segment(second, text::Kind::Nickname)?;
            Ok(UnconfirmedProposal::Friend {
                bundle: binary[..65].try_into().map_err(|_| Error::Invalid)?,
                nickname,
            })
        }
        "event" if len == 101 => {
            if binary[0] != 1 {
                return Err(Error::UpdateNeeded);
            }
            let name = display_segment(second, text::Kind::EventName)?;
            let digest = Sha256::digest(&binary[1..33]);
            Ok(UnconfirmedProposal::Event {
                bundle: binary[..101].try_into().map_err(|_| Error::Invalid)?,
                name,
                root_id: digest[..8].try_into().map_err(|_| Error::Invalid)?,
            })
        }
        "staff" if (114..=130).contains(&len) => {
            if binary[0] != 1 {
                return Err(Error::UpdateNeeded);
            }
            let credential = codec::credential(&binary[..len]).map_err(|_| Error::Invalid)?;
            text::validate(credential.label, text::Kind::CredentialLabel)
                .map_err(|_| Error::Text)?;
            let mut seed = Zeroizing::new([0u8; 32]);
            if decode_binary(second, &mut *seed)? != 32 {
                return Err(Error::Invalid);
            }
            Ok(UnconfirmedProposal::Staff(StaffProposal {
                credential: binary[..len].to_vec(),
                seed,
            }))
        }
        _ => Err(Error::Invalid),
    }
}

fn decode_binary(raw: &str, out: &mut [u8]) -> Result<usize, Error> {
    if raw.is_empty() || raw.len() % 4 == 1 || raw.len() > (out.len() * 8).div_ceil(6) {
        return Err(Error::Invalid);
    }
    let mut bits = 0;
    let mut acc = 0u32;
    let mut len = 0;
    for byte in raw.bytes() {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'-' => 62,
            b'_' => 63,
            _ => return Err(Error::Invalid),
        };
        acc = (acc << 6) | u32::from(value);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            *out.get_mut(len).ok_or(Error::Invalid)? = (acc >> bits) as u8;
            len += 1;
            acc &= (1 << bits) - 1;
        }
    }
    // Alphabet + exact unpadded length + zero unused bits uniquely define the
    // canonical decode/re-encode representation (RFC 4648).
    if acc != 0 || raw.len() != (len * 8).div_ceil(6) {
        return Err(Error::Invalid);
    }
    Ok(len)
}
fn hex(byte: u8) -> Result<u8, Error> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(Error::Invalid),
    }
}
fn unreserved(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~')
}
fn display_segment(raw: &str, kind: text::Kind) -> Result<String, Error> {
    // Length check before allocating or decoding; percent escapes are at most 3
    // characters per decoded byte. No recursive decoding, even for "%25".
    if raw.len() > kind.max_bytes() * 3 {
        return Err(Error::Text);
    }
    let mut out = Vec::with_capacity(kind.max_bytes());
    let mut bytes = raw.bytes();
    while let Some(b) = bytes.next() {
        let decoded = if b == b'%' {
            (hex(bytes.next().ok_or(Error::Invalid)?)? << 4)
                | hex(bytes.next().ok_or(Error::Invalid)?)?
        } else if unreserved(b) {
            b
        } else {
            return Err(Error::Invalid);
        };
        if out.len() == kind.max_bytes() {
            return Err(Error::Text);
        }
        out.push(decoded);
    }
    let result = String::from_utf8(out).map_err(|_| Error::Text)?;
    text::validate(&result, kind).map_err(|_| Error::Text)?;
    Ok(result)
}
