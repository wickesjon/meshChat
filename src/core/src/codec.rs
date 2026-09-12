//! MC-006/008 structural codec, independent of ingress scheduling and authentication.
//!
//! Parsing borrows the input and allocates nothing. Success never means authenticated,
//! display-safe, budget-admitted or history-accepted. Callers must apply those gates.
//! Text is raw UTF-8: render a separately sanitized copy, never normalize signed bytes.

pub const HEADER_LEN: usize = 26;
pub const MAX_PACKET_LEN: usize = 1024;
pub const EVENT_CHANNEL: [u8; 4] = [0xd9, 0x1a, 0xe7, 0x6a];

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("invalid or truncated length")]
    Length,
    #[error("unsupported version or reserved type")]
    VersionOrType,
    #[error("invalid semantic flags")]
    Flags,
    #[error("invalid TTL, channel or history context")]
    Scope,
    #[error("invalid field value")]
    Field,
    #[error("invalid UTF-8")]
    Utf8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Context {
    Live,
    /// A stored CHAT inside SYNC. This does not grant history eligibility or trust.
    StoredChat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub kind: u8,
    pub flags: u8,
    /// Raw received TTL. Use `Packet::forward_to` for clamp/decrement semantics.
    pub ttl: u8,
    pub message_id: [u8; 8],
    pub sender_id: [u8; 8],
    pub channel_id: [u8; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FriendSignature<'a> {
    pub key_id: &'a [u8],
    pub public_key: Option<&'a [u8]>,
    pub signature: &'a [u8],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Credential<'a> {
    pub raw: &'a [u8],
    pub root_id: &'a [u8],
    pub staff_public_key: &'a [u8],
    pub not_before: u32,
    pub not_after: u32,
    pub label: &'a str,
    pub root_signature: &'a [u8],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signature<'a> {
    None,
    Friend(FriendSignature<'a>),
    Organizer {
        pin_state: u8,
        pin_expiry: u32,
        root_id: &'a [u8],
        staff_key_id: &'a [u8],
        credential: Option<Credential<'a>>,
        signature: &'a [u8],
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Payload<'a> {
    Chat {
        timestamp: u32,
        avatar: u8,
        nickname: &'a str,
        cosmetics: &'a [u8],
        text: &'a str,
        signature: Signature<'a>,
    },
    Announce {
        timestamp: u32,
        avatar: u8,
        peer_count: u8,
        status: u8,
        nickname: &'a str,
        cosmetics: &'a [u8],
        digest: &'a [u8],
        signature: Option<FriendSignature<'a>>,
    },
    SyncRequest {
        session_id: u16,
        item_count: u16,
        cursor: u32,
        bloom: &'a [u8],
    },
    EventInfo {
        root_id: &'a [u8],
        name: &'a str,
    },
    Reaction {
        target: &'a [u8],
        action: u8,
        code: u8,
    },
    CredentialRequest {
        root_id: &'a [u8],
        staff_key_id: &'a [u8],
    },
    CredentialOffer(Credential<'a>),
    /// Profile and canonical enc are structural only; no DH/AEAD runs here.
    Encrypted {
        recipient_hint: &'a [u8],
        enc: &'a [u8],
        ciphertext: &'a [u8],
    },
    /// Unknown types have no display, history or authentication interpretation.
    Unknown(&'a [u8]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Packet<'a> {
    raw: &'a [u8],
    header: Header,
    payload: Payload<'a>,
}

impl<'a> Packet<'a> {
    pub fn header(&self) -> Header {
        self.header
    }
    pub fn payload(&self) -> Payload<'a> {
        self.payload
    }
    pub fn as_bytes(&self) -> &'a [u8] {
        self.raw
    }
    pub fn payload_bytes(&self) -> &'a [u8] {
        &self.raw[HEADER_LEN..]
    }

    /// Exact original immutable-header segments, including reserved flag bits.
    pub fn immutable_header(&self) -> (&'a [u8], &'a [u8]) {
        (&self.raw[..3], &self.raw[4..HEADER_LEN])
    }

    /// Returns no output for direct controls or expired floods. Caller owns budgets,
    /// dedup and trust decisions. No other byte (including opaque flags) changes.
    pub fn forward_to(&self, output: &mut [u8]) -> Result<Option<usize>, Error> {
        if is_direct(self.header.kind) || self.header.ttl <= 1 {
            return Ok(None);
        }
        let out = output.get_mut(..self.raw.len()).ok_or(Error::Length)?;
        out.copy_from_slice(self.raw);
        out[3] = self.header.ttl.min(7) - 1;
        Ok(Some(out.len()))
    }
}

fn is_direct(kind: u8) -> bool {
    matches!(kind, 2 | 3 | 7)
}

/// Serialize a wire representation, preserving receiver-permitted reserved bits.
/// This is also usable for stored/relay representations; origin policy (fresh IDs,
/// initial TTL 7, zero reserved bits/cosmetics, signing) belongs to the sender.
/// The output is meaningful only on success; no allocation or partial success occurs.
pub fn serialize(
    header: Header,
    payload: &[u8],
    context: Context,
    output: &mut [u8],
) -> Result<usize, Error> {
    let len = HEADER_LEN.checked_add(payload.len()).ok_or(Error::Length)?;
    if len > MAX_PACKET_LEN {
        return Err(Error::Length);
    }
    let out = output.get_mut(..len).ok_or(Error::Length)?;
    out[..4].copy_from_slice(&[1, header.kind, header.flags, header.ttl]);
    out[4..12].copy_from_slice(&header.message_id);
    out[12..20].copy_from_slice(&header.sender_id);
    out[20..24].copy_from_slice(&header.channel_id);
    out[24..26].copy_from_slice(&(payload.len() as u16).to_be_bytes());
    out[26..].copy_from_slice(payload);
    parse(out, context)?;
    Ok(len)
}

pub fn parse(raw: &[u8], context: Context) -> Result<Packet<'_>, Error> {
    if !(HEADER_LEN..=MAX_PACKET_LEN).contains(&raw.len()) {
        return Err(Error::Length);
    }
    let mut c = Cursor(raw);
    if c.byte()? != 1 {
        return Err(Error::VersionOrType);
    }
    let header = Header {
        kind: c.byte()?,
        flags: c.byte()?,
        ttl: c.byte()?,
        message_id: c.array()?,
        sender_id: c.array()?,
        channel_id: c.array()?,
    };
    let len = usize::from(c.u16()?);
    if len != c.0.len() {
        return Err(Error::Length);
    }
    let flags = header.flags & 7;
    let allowed = match header.kind {
        0 | 4 => return Err(Error::VersionOrType),
        1 => matches!(flags, 0 | 1 | 2 | 6),
        2 => matches!(flags, 0 | 2),
        3 | 5 | 7 | 8 => flags == 0,
        6 => matches!(flags, 0 | 1),
        _ => true,
    };
    if !allowed {
        return Err(Error::Flags);
    }
    if context == Context::StoredChat && header.kind != 1 {
        return Err(Error::Scope);
    }
    if is_direct(header.kind) {
        if header.ttl != 1 || header.channel_id != [0; 4] {
            return Err(Error::Scope);
        }
    } else if context == Context::Live && header.ttl == 0 {
        return Err(Error::Scope);
    }
    if (matches!(header.kind, 5 | 8) || (header.kind == 1 && flags == 6))
        && header.channel_id != EVENT_CHANNEL
    {
        return Err(Error::Scope);
    }

    let payload = if matches!(header.kind, 1 | 6) && flags == 1 {
        if !(matches!(len, 121 | 201 | 361) && (header.kind == 1 || len == 121)) {
            return Err(Error::Length);
        }
        if c.byte()? != 1 {
            return Err(Error::Field);
        }
        let recipient_hint = c.take(8)?;
        let enc = c.take(32)?;
        // Canonical little-endian u < 2^255-19 (MC-008). All-zero DH is a later gate.
        let mut prime = [0xff; 32];
        prime[0] = 0xed;
        prime[31] = 0x7f;
        if enc.iter().rev().cmp(prime.iter().rev()) != std::cmp::Ordering::Less {
            return Err(Error::Field);
        }
        Payload::Encrypted {
            recipient_hint,
            enc,
            ciphertext: c.take(c.0.len())?,
        }
    } else {
        match header.kind {
            1 => {
                let timestamp = c.u32()?;
                let avatar = c.byte()?;
                let nickname = c.text8(1, 20)?;
                let cosmetics = c.take(4)?;
                let text_len = usize::from(c.u16()?);
                let text = c.text(text_len, 1, 280)?;
                let signature = match flags {
                    0 => Signature::None,
                    2 => Signature::Friend(friend(&mut c, &header.sender_id, false)?),
                    6 => organizer(&mut c)?,
                    _ => return Err(Error::Flags),
                };
                Payload::Chat {
                    timestamp,
                    avatar,
                    nickname,
                    cosmetics,
                    text,
                    signature,
                }
            }
            2 => {
                let timestamp = c.u32()?;
                let avatar = c.byte()?;
                let peer_count = c.byte()?;
                let status = c.byte()?;
                let nickname = c.text8(1, 20)?;
                let cosmetics = c.take(4)?;
                let digest_len = usize::from(c.u16()?);
                if !matches!(digest_len, 0 | 256) {
                    return Err(Error::Field);
                }
                let digest = c.take(digest_len)?;
                let signature = if flags == 2 {
                    Some(friend(&mut c, &header.sender_id, true)?)
                } else {
                    None
                };
                Payload::Announce {
                    timestamp,
                    avatar,
                    peer_count,
                    status,
                    nickname,
                    cosmetics,
                    digest,
                    signature,
                }
            }
            3 => {
                let session_id = c.u16()?;
                let item_count = c.u16()?;
                if item_count > 5000 {
                    return Err(Error::Field);
                }
                Payload::SyncRequest {
                    session_id,
                    item_count,
                    cursor: c.u32()?,
                    bloom: c.take(512)?,
                }
            }
            5 => Payload::EventInfo {
                root_id: c.take(8)?,
                name: c.text8(1, 32)?,
            },
            6 => Payload::Reaction {
                target: c.take(8)?,
                action: c.byte()?,
                code: c.byte()?,
            },
            7 => Payload::CredentialRequest {
                root_id: c.take(8)?,
                staff_key_id: c.take(8)?,
            },
            8 => Payload::CredentialOffer(credential(c.take(c.0.len())?)?),
            _ => Payload::Unknown(c.take(c.0.len())?),
        }
    };
    if !c.0.is_empty() {
        return Err(Error::Length);
    }
    Ok(Packet {
        raw,
        header,
        payload,
    })
}

fn friend<'a>(
    c: &mut Cursor<'a>,
    sender_id: &[u8; 8],
    require_key: bool,
) -> Result<FriendSignature<'a>, Error> {
    let key_id = c.take(8)?;
    if key_id != sender_id {
        return Err(Error::Field);
    }
    let public_key = match c.byte()? {
        0 if !require_key => None,
        1 => Some(c.take(32)?),
        _ => return Err(Error::Field),
    };
    // SHA-256/public-key binding, strict key/signature validity and cache resolution
    // are authentication work owned by MC-019, never implied by structural success.
    Ok(FriendSignature {
        key_id,
        public_key,
        signature: c.take(64)?,
    })
}

fn organizer<'a>(c: &mut Cursor<'a>) -> Result<Signature<'a>, Error> {
    let pin_state = c.byte()?;
    let pin_expiry = c.u32()?;
    if !matches!((pin_state, pin_expiry), (0, 0) | (1, 1..=u32::MAX)) {
        return Err(Error::Field);
    }
    let root_id = c.take(8)?;
    let staff_key_id = c.take(8)?;
    let included = c.byte()?;
    let len = usize::from(c.u16()?);
    let credential = match (included, len) {
        (0, 0) => None,
        (1, 114..=130) => {
            let value = credential(c.take(len)?)?;
            if value.root_id != root_id {
                return Err(Error::Field);
            }
            Some(value)
        }
        _ => return Err(Error::Field),
    };
    // Staff-key hash, adopted root, signatures and clock/pin authority are MC-021 gates.
    Ok(Signature::Organizer {
        pin_state,
        pin_expiry,
        root_id,
        staff_key_id,
        credential,
        signature: c.take(64)?,
    })
}

pub fn credential(raw: &[u8]) -> Result<Credential<'_>, Error> {
    if !(114..=130).contains(&raw.len()) {
        return Err(Error::Length);
    }
    let mut c = Cursor(raw);
    if c.byte()? != 1 {
        return Err(Error::VersionOrType);
    }
    let root_id = c.take(8)?;
    let staff_public_key = c.take(32)?;
    let not_before = c.u32()?;
    let not_after = c.u32()?;
    if not_before > not_after {
        return Err(Error::Field);
    }
    let label = c.text8(0, 16)?;
    let root_signature = c.take(64)?;
    if !c.0.is_empty() {
        return Err(Error::Length);
    }
    Ok(Credential {
        raw,
        root_id,
        staff_public_key,
        not_before,
        not_after,
        label,
        root_signature,
    })
}

struct Cursor<'a>(&'a [u8]);
impl<'a> Cursor<'a> {
    fn take(&mut self, len: usize) -> Result<&'a [u8], Error> {
        let value = self.0.get(..len).ok_or(Error::Length)?;
        self.0 = self.0.get(len..).ok_or(Error::Length)?;
        Ok(value)
    }
    fn array<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        self.take(N)?.try_into().map_err(|_| Error::Length)
    }
    fn byte(&mut self) -> Result<u8, Error> {
        Ok(self.array::<1>()?[0])
    }
    fn u16(&mut self) -> Result<u16, Error> {
        Ok(u16::from_be_bytes(self.array()?))
    }
    fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_be_bytes(self.array()?))
    }
    fn text8(&mut self, min: usize, max: usize) -> Result<&'a str, Error> {
        let len = usize::from(self.byte()?);
        self.text(len, min, max)
    }
    fn text(&mut self, len: usize, min: usize, max: usize) -> Result<&'a str, Error> {
        if !(min..=max).contains(&len) {
            return Err(Error::Field);
        }
        std::str::from_utf8(self.take(len)?).map_err(|_| Error::Utf8)
    }
}
