use meshchat_core::{
    channel::{self, Channel},
    links::{self, UnconfirmedProposal},
    text::{self, Kind},
};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;
fn b64(raw: &[u8]) -> String {
    let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::new();
    let mut bits = 0;
    let mut acc = 0u32;
    for b in raw {
        acc = (acc << 8) | u32::from(*b);
        bits += 8;
        while bits >= 6 {
            bits -= 6;
            out.push(alphabet[((acc >> bits) & 63) as usize] as char);
        }
        acc &= (1 << bits) - 1;
    }
    if bits > 0 {
        out.push(alphabet[(acc << (6 - bits)) as usize] as char);
    }
    out
}
fn friend() -> Vec<u8> {
    let mut b = vec![9; 65];
    b[0] = 1;
    b
}
fn event() -> Vec<u8> {
    let mut b = vec![9; 101];
    b[0] = 1;
    b
}
fn credential(label: &[u8]) -> Vec<u8> {
    let mut b = vec![0; 50];
    b[0] = 1;
    b[45..49].copy_from_slice(&1000u32.to_be_bytes());
    b[49] = label.len() as u8;
    b.extend_from_slice(label);
    b.extend_from_slice(&[0; 64]);
    b
}
fn friend_url(name: &str) -> String {
    format!("meshfest://friend/{}/{name}", b64(&friend()))
}
fn error(raw: &str, wanted: links::Error) {
    assert!(matches!(links::parse(raw),Err(e) if e==wanted));
}

#[test]
fn every_channel_matches_independent_vectors_and_roundtrips() {
    let mut seen = std::collections::BTreeSet::new();
    let mut count = 0;
    for line in include_str!("../../vectors/channels/channels.tsv")
        .lines()
        .filter(|l| !l.starts_with("# canonical_name"))
    {
        let p: Vec<_> = line.split('\t').collect();
        let c = channel::parse_name(p[0]).unwrap();
        assert_eq!(c.canonical_name(), p[0]);
        assert_eq!(
            u32::from_be_bytes(c.id()),
            u32::from_str_radix(p[1], 16).unwrap()
        );
        assert_eq!(c.glyph(), p[2]);
        assert!(seen.insert(c.id()));
        if matches!(c, Channel::Private(_)) {
            for https in [false, true] {
                let url = c.join_link(https).unwrap();
                assert!(matches!(links::parse(&url),Ok(UnconfirmedProposal::Channel(p))if p==c));
            }
            assert!(!["wave", "fire", "lightning-bolt"].contains(&c.glyph()));
        } else {
            assert!(c.join_link(false).is_err());
        }
        count += 1;
    }
    assert_eq!(count, 8003);
    assert_eq!(
        channel::parse_name("  #EVENT\u{2003}  UPDATES ")
            .unwrap()
            .id(),
        [0xd9, 0x1a, 0xe7, 0x6a]
    );
    assert_eq!(channel::normalize_name(" E\u{301}  X ").unwrap(), "é x");
    assert!(channel::normalize_name(&"x".repeat(129)).is_err());
    assert_eq!(
        channel::private(["new", "techno", "valley"]),
        Err(channel::Error::UpdateNeeded)
    );
    for raw in [
        "melodic-techno-valley",
        "melodic|techno|valley|extra",
        "#unknown",
        "melodic||valley",
    ] {
        assert!(channel::parse_name(raw).is_err());
    }
}
#[test]
fn route_authority_and_segment_grammar() {
    for raw in [
        "meshfest://j/melodic-techno-valley",
        "MESHFEST://J/MELODIC-TECHNO-VALLEY",
        "HTTPS://MESHFEST.APP/j/melodic-techno-valley",
    ] {
        assert!(matches!(
            links::parse(raw),
            Ok(UnconfirmedProposal::Channel(_))
        ));
    }
    for raw in [
        "http://meshfest.app/j/melodic-techno-valley",
        "https://meshfest.app:443/j/melodic-techno-valley",
        "https://u@meshfest.app/j/melodic-techno-valley",
        "https://meshfest.app.evil/j/melodic-techno-valley",
        "https://meshfest.app/J/melodic-techno-valley",
        "meshfest://j/melodic-techno-valley/",
        "meshfest://j//melodic-techno-valley",
        "meshfest://j/melodic-techno-valley?q=x",
        "meshfest://j/melodic-techno-valley#x",
        "meshfest://j/melodic%2Dtechno-valley",
        " meshfest://j/melodic-techno-valley",
        "meshfest://j/mélodic-techno-valley",
        "meshfest://j/melodic-techno-valley\n",
    ] {
        assert!(links::parse(raw).is_err());
    }
    error("meshfest://j/new-techno-valley", links::Error::UpdateNeeded);
    assert!(links::parse(&"x".repeat(2049)).is_err());
    assert!(links::parse("https://meshfest.app/staff/x/y").is_err());
}
#[test]
fn inert_public_bundle_proposals_preserve_bytes_and_claims() {
    let binary = friend();
    let uri = friend_url("A%2fB%252FC");
    match links::parse(&uri).unwrap() {
        UnconfirmedProposal::Friend { bundle, nickname } => {
            assert_eq!(bundle.as_slice(), binary);
            assert_eq!(nickname, "A/B%2FC");
        }
        _ => panic!("friend"),
    }
    let binary = event();
    let uri = format!("https://meshfest.app/event/{}/%C3%A9", b64(&binary));
    match links::parse(&uri).unwrap() {
        UnconfirmedProposal::Event {
            bundle,
            name,
            root_id,
        } => {
            assert_eq!(bundle.as_slice(), binary);
            assert_eq!(name, "é");
            assert_eq!(root_id.as_slice(), &Sha256::digest(&binary[1..33])[..8]);
        }
        _ => panic!("event"),
    }
    for size in [0, 1, 64, 66, 100, 102, 131] {
        error(
            &format!("meshfest://friend/{}/N", b64(&vec![1; size])),
            links::Error::Invalid,
        );
    }
    let mut b = friend();
    b[0] = 2;
    error(
        &format!("meshfest://friend/{}/N", b64(&b)),
        links::Error::UpdateNeeded,
    );
    let mut b = event();
    b[0] = 2;
    error(
        &format!("meshfest://event/{}/N", b64(&b)),
        links::Error::UpdateNeeded,
    );
    // These synthetic keys/signatures are not trusted or cryptographically validated.
}
#[test]
fn strict_base64_and_percent_encoding() {
    let good = b64(&friend());
    for bad in [
        format!("{good}="),
        format!("{good}%3D"),
        format!("+{}", &good[1..]),
        format!("/{}", &good[1..]),
        format!("{}A", &good[..good.len() - 2]),
    ] {
        assert!(links::parse(&format!("meshfest://friend/{bad}/N")).is_err());
    }
    let mut bad = good.as_bytes().to_vec();
    *bad.last_mut().unwrap() += 1;
    error(
        &format!("meshfest://friend/{}/N", String::from_utf8(bad).unwrap()),
        links::Error::Invalid,
    ); // nonzero unused bits
    for name in [
        "%",
        "%0",
        "%GG",
        "x:y",
        "x@y",
        "x+y",
        "é",
        "%FF",
        "%C0%AF",
        "%00",
        "%C2%80",
        "%E2%80%AE",
        "%E2%80%8D",
        "%EF%BB%BF",
        "",
    ] {
        assert!(links::parse(&friend_url(name)).is_err());
    }
    assert!(links::parse(&friend_url(&"%F0%9F%8C%BB".repeat(5))).is_ok());
    assert!(links::parse(&friend_url(&"%F0%9F%8C%BB".repeat(6))).is_err());
    assert!(links::parse(&format!("{}/extra", friend_url("N"))).is_err());
}
#[test]
fn staff_seeds_are_runtime_only_and_transfer_in_zeroizing_storage() {
    // Synthetic bytes generated only for structural parsing, never used as an identity.
    let seed = Zeroizing::new(<[u8; 32]>::from(Sha256::digest(
        b"MC-011 public structural test input",
    )));
    let c = credential(b"Staff");
    let uri = Zeroizing::new(format!("meshfest://staff/{}/{}", b64(&c), b64(&*seed)));
    match links::parse(&uri).unwrap() {
        UnconfirmedProposal::Staff(p) => {
            assert_eq!(p.credential(), c);
            let (_, decoded) = p.into_import_parts();
            assert!(decoded.as_slice() == seed.as_slice());
        }
        _ => panic!("staff variant"),
    }
    for len in [0, 31, 33] {
        let uri = Zeroizing::new(format!(
            "meshfest://staff/{}/{}",
            b64(&c),
            b64(&vec![0; len])
        ));
        assert!(links::parse(&uri).is_err());
    }
    for c in [
        credential(&[0]),
        credential(&[0xff]),
        credential(&[b'a'; 17]),
    ] {
        let uri = Zeroizing::new(format!("meshfest://staff/{}/{}", b64(&c), b64(&*seed)));
        assert!(links::parse(&uri).is_err());
    }
}
#[test]
fn unicode_display_and_wire_byte_boundaries() {
    assert_eq!(unicode_normalization::UNICODE_VERSION, (16, 0, 0));
    assert_eq!(unicode_security::UNICODE_VERSION, (16, 0, 0));
    let raw = "e\u{301}";
    let d = text::display(raw, Kind::Nickname).unwrap();
    assert_eq!(d.original.as_bytes(), raw.as_bytes());
    assert_eq!(d.rendered, "é");
    assert!(d.changed() && d.single_line_ellipsis);
    let raw = "a\u{301}\u{302}\u{303}\u{304}";
    let d = text::display(raw, Kind::Message).unwrap();
    use unicode_normalization::UnicodeNormalization;
    assert_eq!(
        d.rendered
            .nfd()
            .filter(|c| unicode_normalization::char::is_combining_mark(*c))
            .count(),
        2
    );
    assert_eq!(d.original, raw);
    assert_eq!(
        text::display("✓STAFF official", Kind::Nickname)
            .unwrap()
            .rendered,
        "unnamed"
    );
    assert_eq!(
        text::display("ofSTAFFficial", Kind::Nickname)
            .unwrap()
            .rendered,
        "unnamed"
    );
    assert_eq!(
        text::display("A\u{2028}B", Kind::Nickname)
            .unwrap()
            .rendered,
        "A B"
    );
    assert_eq!(
        text::display("\u{301}A", Kind::Nickname).unwrap().rendered,
        "A"
    );
    for raw in [
        "x\0",
        "x\n",
        "x\u{85}",
        "x\u{202a}",
        "x\u{2069}",
        "x\u{200b}",
        "👨\u{200d}👩",
    ] {
        assert!(text::display(raw, Kind::Message).is_err());
    }
    for (n, ok) in [(5, true), (6, false)] {
        assert_eq!(text::outgoing(&"🌻".repeat(n), Kind::Nickname).is_ok(), ok);
    }
    for (n, ok) in [(70, true), (71, false)] {
        assert_eq!(text::outgoing(&"🌻".repeat(n), Kind::Message).is_ok(), ok);
    }
    assert!(text::confusable_with_own("sсоре", "scope").unwrap());
    assert!(!text::confusable_with_own("Alice", "Bob").unwrap());
    assert_eq!(
        text::display("<b>https://evil</b>", Kind::Message)
            .unwrap()
            .rendered,
        "<b>https://evil</b>"
    );
}

#[test]
fn independent_public_uri_vectors() {
    for line in include_str!("../../vectors/channels/public-links.tsv")
        .lines()
        .filter(|l| !l.starts_with('#'))
    {
        let p: Vec<_> = line.split('\t').collect();
        assert_eq!(links::parse(p[2]).is_ok(), p[1] == "true", "{}", p[0]);
    }
}
