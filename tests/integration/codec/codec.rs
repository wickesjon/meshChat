use meshchat_core::codec::{self, Context, Error, Payload, Signature};

fn bytes(hex: &str) -> Vec<u8> {
    if hex == "-" {
        return vec![];
    }
    hex.as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
fn fixtures() -> Vec<(&'static str, Context, &'static str, Vec<u8>)> {
    include_str!("../../vectors/base/logical.txt")
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| {
            let f: Vec<_> = l.split_whitespace().collect();
            (
                f[0],
                if f[1] == "live" {
                    Context::Live
                } else {
                    Context::StoredChat
                },
                f[2],
                bytes(f[3]),
            )
        })
        .collect()
}
fn expected(name: &str) -> Error {
    match name {
        "length" => Error::Length,
        "version" => Error::VersionOrType,
        "flags" => Error::Flags,
        "scope" => Error::Scope,
        "field" => Error::Field,
        "utf8" => Error::Utf8,
        _ => panic!("unknown expectation"),
    }
}
fn fixture(id: &str) -> Vec<u8> {
    fixtures().into_iter().find(|f| f.0 == id).unwrap().3
}

#[test]
fn golden_decode_and_independent_encode() {
    for (id, context, result, raw) in fixtures() {
        if result != "ok" {
            assert_eq!(
                codec::parse(&raw, context).unwrap_err(),
                expected(result),
                "{id}"
            );
            continue;
        }
        let packet = codec::parse(&raw, context).unwrap_or_else(|e| panic!("{id}: {e}"));
        assert_eq!(packet.as_bytes(), raw, "{id}");
        let mut output = [0xa5; 1024];
        let n = codec::serialize(
            packet.header(),
            packet.payload_bytes(),
            context,
            &mut output,
        )
        .unwrap();
        assert_eq!(&output[..n], raw, "{id}");
        assert_eq!(codec::parse(&output[..n], context).unwrap(), packet, "{id}");
        assert_eq!(packet.as_bytes().as_ptr(), raw.as_ptr());
        let (a, b) = packet.immutable_header();
        assert_eq!(a, &raw[..3]);
        assert_eq!(b, &raw[4..26]);
        assert!(
            codec::serialize(
                packet.header(),
                packet.payload_bytes(),
                context,
                &mut output[..n - 1]
            )
            .is_err()
        );
        for high in [0x08, 0x80, 0xf8] {
            let mut extra = raw.clone();
            extra[2] |= high;
            let p = codec::parse(&extra, context).unwrap();
            assert_eq!(p.payload(), packet.payload());
            assert_eq!(p.header().flags, extra[2]);
        }
    }
}

#[test]
fn every_truncation_and_header_length_boundary() {
    for (id, context, result, raw) in fixtures() {
        if result != "ok" {
            continue;
        }
        for end in 0..raw.len() {
            assert!(codec::parse(&raw[..end], context).is_err(), "{id}/{end}");
        }
        let mut trailing = raw.clone();
        trailing.push(0);
        assert!(codec::parse(&trailing, context).is_err(), "{id}");
        for length in [0, 1, 998, 999, u16::MAX] {
            if usize::from(length) == raw.len() - 26 {
                continue;
            }
            let mut changed = raw.clone();
            changed[24..26].copy_from_slice(&length.to_be_bytes());
            assert!(codec::parse(&changed, context).is_err(), "{id}/{length}");
        }
        for version in [0, 2, 0x11, 255] {
            let mut changed = raw.clone();
            changed[0] = version;
            assert_eq!(
                codec::parse(&changed, context).unwrap_err(),
                Error::VersionOrType
            );
        }
    }
    let mut raw = fixture("unknown-maximum");
    raw.push(0);
    raw[24..26].copy_from_slice(&999u16.to_be_bytes());
    assert_eq!(
        codec::parse(&raw, Context::Live).unwrap_err(),
        Error::Length
    );
}

#[test]
fn forwarding_preserves_immutable_bytes_and_opaque_semantics() {
    for (id, context, result, raw) in fixtures() {
        if result != "ok" {
            continue;
        }
        let p = codec::parse(&raw, context).unwrap();
        let mut out = [0xa5; 1024];
        let n = p.forward_to(&mut out).unwrap();
        if matches!(p.header().kind, 2 | 3 | 7) || p.header().ttl <= 1 {
            assert_eq!(n, None, "{id}");
            assert!(out.iter().all(|b| *b == 0xa5));
        } else {
            let n = n.unwrap();
            assert_eq!(n, raw.len());
            assert_eq!(out[3], p.header().ttl.min(7) - 1);
            assert_eq!(&out[..3], &raw[..3]);
            assert_eq!(&out[4..n], &raw[4..]);
            assert_eq!(p.forward_to(&mut out[..n - 1]), Err(Error::Length));
        }
        if p.header().kind >= 9 {
            assert!(matches!(p.payload(), Payload::Unknown(_)));
        }
    }
    for ttl in 0..=255 {
        let mut raw = fixture("unknown-empty");
        raw[3] = ttl;
        if ttl == 0 {
            assert!(codec::parse(&raw, Context::Live).is_err());
            continue;
        }
        let p = codec::parse(&raw, Context::Live).unwrap();
        let mut out = [0; 1024];
        assert_eq!(p.forward_to(&mut out).unwrap().is_some(), ttl > 1);
        if ttl > 1 {
            assert_eq!(out[3], ttl.min(7) - 1);
        }
    }
}

#[test]
fn field_views_and_maximum_sizes() {
    let raw = fixture("chat");
    match codec::parse(&raw, Context::Live).unwrap().payload() {
        Payload::Chat {
            timestamp,
            avatar,
            nickname,
            cosmetics,
            text,
            signature,
        } => {
            assert_eq!(
                (timestamp, avatar, nickname, text),
                (123, 255, "N", "Hello")
            );
            assert_eq!(cosmetics, [0x83, 1, 2, 3]);
            assert_eq!(signature, Signature::None);
        }
        _ => panic!("CHAT"),
    }
    let raw = fixture("organizer-included");
    match codec::parse(&raw, Context::Live).unwrap().payload() {
        Payload::Chat {
            signature:
                Signature::Organizer {
                    pin_state,
                    pin_expiry,
                    credential: Some(c),
                    ..
                },
            ..
        } => {
            assert_eq!((pin_state, pin_expiry), (1, 100));
            assert_eq!((c.not_before, c.not_after, c.label), (10, 1000, "Staff"));
        }
        _ => panic!("organizer"),
    }
    let raw = fixture("signed-announce");
    match codec::parse(&raw, Context::Live).unwrap().payload() {
        Payload::Announce {
            peer_count,
            status,
            digest,
            signature: Some(s),
            ..
        } => {
            assert_eq!((peer_count, status, digest.len()), (4, 255, 256));
            assert_eq!(s.public_key.unwrap().len(), 32);
        }
        _ => panic!("announce"),
    }
    let raw = fixture("sync-request");
    match codec::parse(&raw, Context::Live).unwrap().payload() {
        Payload::SyncRequest {
            session_id,
            item_count,
            cursor,
            bloom,
        } => assert_eq!(
            (session_id, item_count, cursor, bloom.len()),
            (1, 5000, 0, 512)
        ),
        _ => panic!("sync"),
    }
    for (id, size) in [
        ("max-chat", 338),
        ("max-friend", 443),
        ("max-organizer", 556),
        ("max-announce", 421),
        ("max-event", 67),
        ("max-credential", 156),
    ] {
        assert_eq!(fixture(id).len(), size, "{id}");
    }
    assert_eq!(
        fixture("reaction"),
        bytes("0106000701020304050607081112131415161718bcf0eae3000a21222324252627280000")
    );
    let raw = fixture("chat-raw-controls");
    assert!(matches!(
        codec::parse(&raw, Context::Live).unwrap().payload(),
        Payload::Chat { text: "a\0b", .. }
    ));
}

#[test]
fn serialized_invalid_layout_is_an_error() {
    let raw = fixture("reaction");
    let p = codec::parse(&raw, Context::Live).unwrap();
    let mut out = [0; 1024];
    let mut h = p.header();
    h.flags = 0x82;
    assert_eq!(
        codec::serialize(h, p.payload_bytes(), Context::Live, &mut out),
        Err(Error::Flags)
    );
    assert_eq!(
        codec::serialize(p.header(), &[0; 999], Context::Live, &mut out),
        Err(Error::Length)
    );
}

#[test]
fn encoder_and_views_match_literal_contract_offsets() {
    let header = codec::Header {
        kind: 6,
        flags: 0,
        ttl: 7,
        message_id: [1, 2, 3, 4, 5, 6, 7, 8],
        sender_id: [17, 18, 19, 20, 21, 22, 23, 24],
        channel_id: [0xbc, 0xf0, 0xea, 0xe3],
    };
    let payload = [33, 34, 35, 36, 37, 38, 39, 40, 0, 0];
    let mut output = [0; 1024];
    let n = codec::serialize(header, &payload, Context::Live, &mut output).unwrap();
    assert_eq!(&output[..n], fixture("reaction"));
    let packet = codec::parse(&output[..n], Context::Live).unwrap();
    assert_eq!(packet.header(), header);
    assert!(
        matches!(packet.payload(),Payload::Reaction{target,action:0,code:0} if target==&payload[..8])
    );
    let raw = fixture("encrypted-chat-201");
    match codec::parse(&raw, Context::Live).unwrap().payload() {
        Payload::Encrypted {
            recipient_hint,
            enc,
            ciphertext,
        } => {
            assert_eq!(recipient_hint, [7; 8]);
            assert_eq!(enc[0], 9);
            assert!(enc[1..].iter().all(|b| *b == 0));
            assert_eq!(ciphertext.len(), 160);
        }
        _ => panic!("encrypted"),
    }
    let raw = fixture("event-info");
    assert!(
        matches!(codec::parse(&raw,Context::Live).unwrap().payload(),Payload::EventInfo{root_id,name:"Event"} if root_id==[49,50,51,52,53,54,55,56])
    );
    let raw = fixture("credential-request");
    assert!(
        matches!(codec::parse(&raw,Context::Live).unwrap().payload(),Payload::CredentialRequest{root_id,staff_key_id} if root_id==[49,50,51,52,53,54,55,56] && staff_key_id==header.sender_id)
    );
    let unknown = fixture("unknown-empty");
    for kind in 9..=255 {
        for flags in 0..=255 {
            let mut raw = unknown.clone();
            raw[1] = kind;
            raw[2] = flags;
            let p = codec::parse(&raw, Context::Live).unwrap();
            assert_eq!((p.header().kind, p.header().flags), (kind, flags));
            assert!(matches!(p.payload(), Payload::Unknown([])));
        }
    }
}
