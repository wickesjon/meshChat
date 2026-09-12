use meshchat_core::{LinkHandle, framing::*};
fn link(generation: u64) -> LinkHandle {
    LinkHandle {
        instance_nonce: 7,
        generation,
    }
}
fn bytes(hex: &str) -> Vec<u8> {
    hex.split_whitespace()
        .collect::<String>()
        .as_bytes()
        .chunks_exact(2)
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect()
}
fn logical(id: &str) -> Vec<u8> {
    let line = include_str!("../../vectors/base/logical.txt")
        .lines()
        .find(|l| l.split_whitespace().next() == Some(id))
        .unwrap();
    bytes(line.split_whitespace().nth(3).unwrap())
}
fn encode(e: Encoder<'_>) -> Vec<Vec<u8>> {
    (0..e.frame_count())
        .map(|i| {
            let mut b = [0; 512];
            let n = e.frame(i, &mut b).unwrap();
            b[..n].to_vec()
        })
        .collect()
}
fn hello() -> Vec<u8> {
    let mut b = vec![0; 54];
    b[0] = 1;
    b[2..4].copy_from_slice(&146u16.to_be_bytes());
    b[4..6].copy_from_slice(&512u16.to_be_bytes());
    b[6] = 1;
    b[22] = 9;
    b
}
fn proof() -> Vec<u8> {
    let mut b = vec![0; 66];
    b[0] = 1;
    b[1] = 1;
    b
}
fn sync() -> Vec<u8> {
    let p = logical("max-chat");
    let mut b = bytes("000100000000000000");
    b.extend_from_slice(&(p.len() as u16).to_be_bytes());
    b.extend(p);
    b
}
fn setup(c: usize) -> Reassembler {
    let mut r = Reassembler::new(7, 8, 0).unwrap();
    r.register(&link(1), c).unwrap();
    r
}
fn outer(kind: u8, body: &[u8]) -> Vec<u8> {
    let mut v = vec![kind, 0];
    v.extend_from_slice(&(body.len() as u16).to_be_bytes());
    v.extend_from_slice(body);
    v
}
fn fragment(
    message: [u8; 8],
    group: u16,
    index: u8,
    count: u8,
    total: u16,
    slice: &[u8],
) -> Vec<u8> {
    let mut b = message.to_vec();
    b.extend_from_slice(&group.to_be_bytes());
    b.extend_from_slice(&[index, count]);
    b.extend_from_slice(&total.to_be_bytes());
    b.extend_from_slice(slice);
    outer(1, &b)
}
#[test]
fn canonical_frames_and_arbitrary_partition() {
    let raw = logical("reaction");
    let frames = encode(Encoder::logical(&raw, 146, 1).unwrap());
    assert_eq!(
        frames,
        [bytes(
            "000000240106000701020304050607081112131415161718bcf0eae3000a21222324252627280000"
        )]
    );
    let parts = [
        bytes("010000200102030405060708000100020024010600070102030405060708111213141516"),
        bytes("0100002001020304050607080001010200241718bcf0eae3000a21222324252627280000"),
    ];
    for order in [[0, 1], [1, 0]] {
        let mut r = setup(146);
        let mut out = [0; 1035];
        assert_eq!(
            r.ingest_admitted(&link(1), &parts[order[0]], 0, &mut out)
                .unwrap(),
            None
        );
        assert_eq!(
            r.ingest_admitted(&link(1), &parts[order[0]], 1, &mut out)
                .unwrap(),
            None
        );
        let done = r
            .ingest_admitted(&link(1), &parts[order[1]], 2, &mut out)
            .unwrap()
            .unwrap();
        assert_eq!((done.kind, done.len), (ObjectKind::Logical, raw.len()));
        assert_eq!(&out[..done.len], raw);
        assert_eq!(r.counters().duplicates, 1);
        assert_eq!(r.reservations().logical_groups, 0);
    }
    let msg = [1, 2, 3, 4, 5, 6, 7, 8];
    let a = fragment(msg, 4, 0, 2, 36, &raw[..1]);
    let b = fragment(msg, 4, 1, 2, 36, &raw[1..]);
    let mut r = setup(146);
    let mut out = [0; 1035];
    r.ingest_admitted(&link(1), &b, 0, &mut out).unwrap();
    assert!(
        r.ingest_admitted(&link(1), &a, 0, &mut out)
            .unwrap()
            .is_some()
    );
    assert_eq!(&out[..36], raw);
}
#[test]
fn directional_sizes_and_all_object_forms() {
    for c in [146, 182, 512] {
        let max = logical("unknown-maximum");
        let forms = [logical("reaction"), logical("max-chat"), max];
        for raw in forms {
            let e = Encoder::logical(&raw, c, 99).unwrap();
            assert!(e.frame_count() <= 8);
            let frames = encode(e);
            let mut r = setup(c);
            let mut out = [0; 1035];
            for (i, f) in frames.iter().enumerate().rev() {
                assert!(f.len() <= c);
                parse_frame(f, c).unwrap();
                let done = r.ingest_admitted(&link(1), f, 0, &mut out).unwrap();
                if i == 0 {
                    let done = done.unwrap();
                    assert_eq!(&out[..done.len], raw);
                } else {
                    assert_eq!(done, None);
                }
            }
        }
        for (kind, body) in [
            (1, sync()),
            (1, bytes("0001000005000000000000")),
            (2, hello()),
            (3, proof()),
        ] {
            let e = Encoder::transport(kind, &body, c, 88).unwrap();
            assert!(e.frame_count() <= 16);
            if kind != 1 {
                assert_eq!(e.frame_count(), 1);
            }
            let frames = encode(e);
            let mut r = setup(c);
            let mut out = [0; 1035];
            for (i, f) in frames.iter().enumerate() {
                assert!(f.len() <= c);
                let done = r.ingest_admitted(&link(1), f, i as u64, &mut out).unwrap();
                if i == frames.len() - 1 {
                    let done = done.unwrap();
                    assert_eq!(done.kind, ObjectKind::Transport(kind));
                    assert_eq!(&out[..done.len], body);
                } else {
                    assert_eq!(done, None);
                }
            }
        }
    }
    let b = logical("unknown-maximum");
    assert_eq!(Encoder::logical(&b, 146, 0).unwrap().frame_count(), 8);
    assert_eq!(Encoder::logical(&b, 182, 0).unwrap().frame_count(), 7);
    for c in [0, 59, 145, 513, usize::MAX] {
        assert!(Encoder::logical(&b, c, 0).is_err());
        assert!(
            Reassembler::new(7, 8, 0)
                .unwrap()
                .register(&link(1), c)
                .is_err()
        );
    }
    let e = Encoder::logical(&b, 146, 0).unwrap();
    assert!(e.frame(8, &mut [0; 512]).is_err());
    assert!(e.frame(0, &mut [0; 145]).is_err());
}
#[test]
fn malformed_outer_and_transport_rules() {
    let raw = logical("reaction");
    let valid = encode(Encoder::logical(&raw, 146, 0).unwrap()).remove(0);
    for end in 0..valid.len() {
        assert!(parse_frame(&valid[..end], 146).is_err());
    }
    for (at, v) in [(0, 4), (1, 1), (2, 1), (3, 0)] {
        let mut x = valid.clone();
        x[at] = v;
        assert!(parse_frame(&x, 146).is_err());
    }
    let mut x = valid.clone();
    x.push(0);
    assert!(parse_frame(&x, 146).is_err());
    for flags in 0..=255 {
        let mut b = bytes("0001000005000000000000");
        b[4] = flags;
        let expected = flags == 5 || flags == 7;
        assert_eq!(transport(1, &b).is_ok(), expected, "flags {flags}");
    }
    let mut marker = bytes("0001000003000000010000");
    assert!(transport(1, &marker).is_ok());
    marker[8] = 0;
    assert!(transport(1, &marker).is_err());
    let mut b = sync();
    b[4] = 3;
    assert!(transport(1, &b).is_err());
    let reaction = logical("reaction");
    let mut b = vec![0; 9];
    b.extend_from_slice(&(reaction.len() as u16).to_be_bytes());
    b.extend(reaction);
    assert!(transport(1, &b).is_err());
    for kind in [0, 4, 255] {
        assert!(transport(kind, &[]).is_err());
    }
    for (kind, body) in [(2, hello()), (3, proof())] {
        let mut b = vec![kind, 0, 0, 0, 1];
        b.extend_from_slice(&(body.len() as u16).to_be_bytes());
        b.extend_from_slice(&[0; 5]);
        b.extend_from_slice(&body);
        assert!(parse_frame(&outer(3, &b), 146).is_err());
    }
    let h = hello();
    let raw = encode(Encoder::transport(2, &h, 146, 0).unwrap()).remove(0);
    assert_eq!(raw.len(), 59);
    assert_eq!(bootstrap_hello(&raw, 59).unwrap(), h);
    assert!(bootstrap_hello(&raw, 58).is_err());
    assert!(bootstrap_hello(&valid, 512).is_err());
    for at in [0, 1, 2, 4, 6] {
        let mut h = hello();
        h[at] = if at == 6 { 0 } else { 255 };
        assert!(transport(2, &h).is_err());
    }
    for at in [0, 1] {
        let mut p = proof();
        p[at] = 2;
        assert!(transport(3, &p).is_err());
    }
}
#[test]
fn malformed_fragment_bounds_and_reserved_bytes() {
    let raw = logical("max-chat");
    let f = encode(Encoder::logical(&raw, 146, 0).unwrap()).remove(0);
    for (at, v) in [(14, 9), (15, 0), (15, 9), (16, 255), (17, 255)] {
        let mut b = f.clone();
        b[at] = v;
        assert!(parse_frame(&b, 146).is_err());
    }
    let mut b = f[..18].to_vec();
    b[2..4].copy_from_slice(&14u16.to_be_bytes());
    assert!(parse_frame(&b, 146).is_err());
    let b = sync();
    let f = encode(Encoder::transport(1, &b, 146, 0).unwrap()).remove(0);
    for at in 11..16 {
        let mut b = f.clone();
        b[at] = 1;
        assert!(parse_frame(&b, 146).is_err());
    }
    for (at, v) in [(4, 2), (7, 16), (8, 0), (8, 17), (9, 255)] {
        let mut b = f.clone();
        b[at] = v;
        assert!(parse_frame(&b, 146).is_err());
    }
}
#[test]
fn conflict_deadlines_and_no_accepted_id_poisoning() {
    let raw = logical("reaction");
    let msg = [1, 2, 3, 4, 5, 6, 7, 8];
    let a = fragment(msg, 1, 0, 2, 36, &raw[..18]);
    let b = fragment(msg, 1, 1, 2, 36, &raw[18..]);
    let mut r = setup(146);
    let mut out = [0; 1035];
    r.ingest_admitted(&link(1), &a, 0, &mut out).unwrap();
    let mut bad = a.clone();
    bad[18] ^= 1;
    assert_eq!(
        r.ingest_admitted(&link(1), &bad, 10_000, &mut out),
        Err(Error::Rejected)
    );
    assert_eq!(
        r.ingest_admitted(&link(1), &b, 29_999, &mut out),
        Err(Error::Rejected)
    );
    assert_eq!(r.reservations().logical_groups, 0);
    assert_eq!(r.reservations().rejected_groups, 1);
    let whole = encode(Encoder::logical(&raw, 146, 9).unwrap()).remove(0);
    assert!(
        r.ingest_admitted(&link(1), &whole, 29_999, &mut out)
            .unwrap()
            .is_some()
    );
    r.ingest_admitted(&link(1), &a, 30_000, &mut out).unwrap();
    assert!(
        r.ingest_admitted(&link(1), &b, 30_000, &mut out)
            .unwrap()
            .is_some()
    );
    let mut first = a.clone();
    first[15] = 0;
    assert!(
        r.ingest_admitted(&link(1), &first, 40_000, &mut out)
            .is_err()
    );
    assert!(r.ingest_admitted(&link(1), &a, 69_999, &mut out).is_err());
    r.ingest_admitted(&link(1), &a, 70_000, &mut out).unwrap();
    assert!(
        r.ingest_admitted(&link(1), &b, 70_000, &mut out)
            .unwrap()
            .is_some()
    );
}
#[test]
fn identity_link_group_and_generation_isolation() {
    let raw = logical("reaction");
    let msg = [1, 2, 3, 4, 5, 6, 7, 8];
    let a = fragment(msg, 1, 0, 2, 36, &raw[..18]);
    let b = fragment(msg, 1, 1, 2, 36, &raw[18..]);
    let mut r = setup(146);
    r.register(&link(2), 146).unwrap();
    let mut out = [0; 1035];
    r.ingest_admitted(&link(1), &a, 0, &mut out).unwrap();
    assert_eq!(r.ingest_admitted(&link(2), &b, 0, &mut out).unwrap(), None);
    let mut other = b.clone();
    other[13] = 2;
    assert_eq!(
        r.ingest_admitted(&link(1), &other, 0, &mut out).unwrap(),
        None
    );
    assert!(
        r.ingest_admitted(&link(1), &b, 0, &mut out)
            .unwrap()
            .is_some()
    );
    assert!(
        r.ingest_admitted(&link(2), &a, 0, &mut out)
            .unwrap()
            .is_some()
    );
    r.disconnect(&link(1)).unwrap();
    assert!(r.ingest_admitted(&link(1), &a, 0, &mut out).is_err());
    assert!(r.register(&link(1), 146).is_err());
    r.register(&link(3), 146).unwrap();
    assert_eq!(r.ingest_admitted(&link(3), &b, 0, &mut out).unwrap(), None);
    let foreign = LinkHandle {
        instance_nonce: 8,
        generation: 3,
    };
    assert!(r.ingest_admitted(&foreign, &a, 0, &mut out).is_err());
    let wrong = [9; 8];
    let a = fragment(wrong, 3, 0, 2, 36, &raw[..18]);
    let b = fragment(wrong, 3, 1, 2, 36, &raw[18..]);
    r.ingest_admitted(&link(3), &a, 0, &mut out).unwrap();
    assert!(r.ingest_admitted(&link(3), &b, 0, &mut out).is_err());
}
#[test]
fn timeout_sum_metadata_capacity_and_time_errors() {
    let raw = logical("reaction");
    let msg = [1, 2, 3, 4, 5, 6, 7, 8];
    let a = fragment(msg, 1, 0, 2, 36, &raw[..18]);
    let b = fragment(msg, 1, 1, 2, 36, &raw[18..]);
    let mut r = setup(146);
    let mut out = [0; 1035];
    r.ingest_admitted(&link(1), &a, 0, &mut out).unwrap();
    r.ingest_admitted(&link(1), &a, 29_999, &mut out).unwrap();
    assert_eq!(
        r.ingest_admitted(&link(1), &b, 30_000, &mut out).unwrap(),
        None
    ); // old first part expired
    let before = r.reservations();
    assert_eq!(
        r.ingest_admitted(&link(1), &a, 30_000, &mut [0; 1034]),
        Err(Error::Capacity)
    );
    assert_eq!(r.reservations(), before);
    assert_eq!(
        r.ingest_admitted(&link(1), &a, 29_999, &mut out),
        Err(Error::Time)
    );
    assert_eq!(r.reservations(), before);
    assert!(!r.capacity_changed(&link(1), 146).unwrap());
    assert!(r.capacity_changed(&link(1), 145).unwrap());
    assert_eq!(r.reservations().logical_groups, 0);
    assert!(r.register(&link(2), 145).is_err());
    r.register(&link(2), 182).unwrap();
    assert!(r.capacity_changed(&link(2), 512).unwrap());
    assert!(r.ingest_admitted(&link(2), &a, 30_000, &mut out).is_err());
    for variant in 0..3 {
        let mut r = setup(146);
        r.ingest_admitted(&link(1), &a, 0, &mut out).unwrap();
        let bad = match variant {
            0 => fragment(msg, 1, 1, 3, 36, &raw[18..]),
            1 => fragment(msg, 1, 1, 2, 35, &raw[18..]),
            _ => fragment(msg, 1, 1, 2, 36, &raw[18..35]),
        };
        assert!(r.ingest_admitted(&link(1), &bad, 10, &mut out).is_err());
        assert_eq!(r.reservations().logical_groups, 0);
        assert_eq!(r.reservations().rejected_groups, 1);
    }
    assert!(Reassembler::new(7, 8, u64::MAX).is_err());
    assert!(Reassembler::new(0, 8, 0).is_err());
    assert!(Reassembler::new(7, 9, 0).is_err());
}
#[test]
fn pool_exhaustion_reservations_and_cleanup() {
    let raw = logical("max-chat");
    let body = sync();
    let mut r = setup(146);
    let mut out = [0; 1035];
    for generation in 2..=8 {
        r.register(&link(generation), 146).unwrap();
    }
    assert!(r.register(&link(9), 146).is_err());
    let initial = r.reservations();
    for generation in 1..=8 {
        for group in 0..9 {
            let f = encode(Encoder::logical(&raw, 146, group).unwrap()).remove(0);
            r.ingest_admitted(&link(generation), &f, 0, &mut out)
                .unwrap();
        }
        for transfer in 0..5 {
            let f = encode(Encoder::transport(1, &body, 146, transfer).unwrap()).remove(0);
            r.ingest_admitted(&link(generation), &f, 0, &mut out)
                .unwrap();
        }
        for group in 100..180 {
            let mut f = encode(Encoder::logical(&raw, 146, group).unwrap()).remove(0);
            f[15] = 0;
            assert!(
                r.ingest_admitted(&link(generation), &f, 0, &mut out)
                    .is_err()
            );
        }
        let u = r.link_reservations(&link(generation)).unwrap();
        assert_eq!(
            (u.logical_groups, u.transport_groups, u.rejected_groups),
            (8, 4, 64)
        );
        assert!(
            u.logical_bytes <= 12 * 1024
                && u.transport_bytes <= 8 * 1024
                && u.rejected_bytes <= 8 * 1024
        );
    }
    let p = r.reservations();
    assert_eq!(
        (p.logical_groups, p.transport_groups, p.rejected_groups),
        (64, 32, 512)
    );
    assert_eq!(
        (
            p.peak_logical_groups,
            p.peak_transport_groups,
            p.peak_rejected_groups
        ),
        (64, 32, 512)
    );
    assert_eq!(
        (p.logical_bytes, p.transport_bytes, p.rejected_bytes),
        (
            initial.logical_bytes,
            initial.transport_bytes,
            initial.rejected_bytes
        )
    );
    assert!(
        p.logical_bytes <= 96 * 1024
            && p.transport_bytes <= 64 * 1024
            && p.rejected_bytes <= 64 * 1024
    );
    assert!(
        p.logical_bytes + p.transport_bytes + p.rejected_bytes + p.manager_bytes < 8 * 1024 * 1024
    );
    r.disconnect(&link(1)).unwrap();
    assert_eq!(
        (
            r.reservations().logical_groups,
            r.reservations().transport_groups,
            r.reservations().rejected_groups
        ),
        (56, 28, 448)
    );
    r.advance(30_000).unwrap();
    let p = r.reservations();
    assert_eq!(
        (p.logical_groups, p.transport_groups, p.rejected_groups),
        (0, 0, 0)
    );
    // Pools retain their bounded reusable reservation; no retained group or payload ownership.
    r.register(&link(9), 146).unwrap();
    let f = encode(Encoder::logical(&logical("reaction"), 146, 0).unwrap()).remove(0);
    assert!(
        r.ingest_admitted(&link(9), &f, 30_000, &mut out)
            .unwrap()
            .is_some()
    );
    println!("reservations: {p:?}");
}
#[test]
fn truncated_unknown_and_oversized_have_no_group_tracking() {
    let mut r = setup(146);
    let mut out = [0; 1035];
    for raw in [
        vec![],
        vec![1; 17],
        vec![3; 15],
        vec![0; 513],
        outer(3, &[9; 20]),
    ] {
        assert!(r.ingest_admitted(&link(1), &raw, 0, &mut out).is_err());
    }
    assert_eq!(r.reservations().rejected_groups, 0);
    assert_eq!(r.counters().frames, 5);
    assert_eq!(r.counters().malformed, 5);
    assert_eq!(r.counters().bytes, 17 + 15 + 513 + 24);
}

#[test]
fn independent_committed_outer_vectors_match_encoder() {
    let vectors: std::collections::BTreeMap<_, _> = include_str!("../../vectors/base/frames.txt")
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| {
            let (id, hex) = l.split_once(' ').unwrap();
            (id, bytes(hex))
        })
        .collect();
    for raw in vectors.values() {
        parse_frame(raw, 146).unwrap();
    }
    let reaction = logical("reaction");
    assert_eq!(
        encode(Encoder::logical(&reaction, 146, 0).unwrap())[0],
        vectors["reaction"]
    );
    for (name, kind, body) in [
        ("terminal", 1, bytes("0001000005000000000000")),
        ("hello", 2, hello()),
        ("proof", 3, proof()),
    ] {
        assert_eq!(
            encode(Encoder::transport(kind, &body, 146, 0).unwrap())[0],
            vectors[name]
        );
    }
    let logical = logical("unknown-maximum");
    for (prefix, frames) in [
        (
            "logical",
            encode(Encoder::logical(&logical, 146, 99).unwrap()),
        ),
        (
            "sync",
            encode(Encoder::transport(1, &sync(), 146, 88).unwrap()),
        ),
    ] {
        for (i, frame) in frames.iter().enumerate() {
            assert_eq!(frame, &vectors[format!("{prefix}-{i}").as_str()]);
        }
    }
}

#[test]
fn oldest_eviction_keeps_original_deadline_and_transport_isolation() {
    let raw = logical("max-chat");
    let mut r = setup(146);
    r.register(&link(2), 146).unwrap();
    let mut out = [0; 1035];
    for group in 0..8 {
        let f = encode(Encoder::logical(&raw, 146, group).unwrap()).remove(0);
        r.ingest_admitted(&link(1), &f, u64::from(group) * 1000, &mut out)
            .unwrap();
    }
    let f = encode(Encoder::logical(&raw, 146, 8).unwrap()).remove(0);
    r.ingest_admitted(&link(1), &f, 10_000, &mut out).unwrap();
    let old = encode(Encoder::logical(&raw, 146, 0).unwrap()).remove(0);
    assert_eq!(
        r.ingest_admitted(&link(1), &old, 29_999, &mut out),
        Err(Error::Rejected)
    );
    assert!(r.ingest_admitted(&link(1), &old, 30_000, &mut out).is_ok());
    let body = sync();
    let frames = encode(Encoder::transport(1, &body, 146, 77).unwrap());
    r.ingest_admitted(&link(1), &frames[0], 30_000, &mut out)
        .unwrap();
    r.ingest_admitted(&link(2), &frames[1], 30_000, &mut out)
        .unwrap();
    assert_eq!(
        r.ingest_admitted(&link(2), &frames[2], 30_000, &mut out)
            .unwrap(),
        None
    );
    let mut conflict = frames[0].clone();
    conflict[16] ^= 1;
    assert_eq!(
        r.ingest_admitted(&link(1), &conflict, 30_001, &mut out),
        Err(Error::Rejected)
    );
    let done = r
        .ingest_admitted(&link(2), &frames[0], 30_001, &mut out)
        .unwrap()
        .unwrap();
    assert_eq!(&out[..done.len], body);
    assert_eq!(
        r.ingest_admitted(&link(1), &frames[0], 59_999, &mut out),
        Err(Error::Rejected)
    );
    assert!(
        r.ingest_admitted(&link(1), &frames[0], 60_000, &mut out)
            .is_ok()
    );
}
