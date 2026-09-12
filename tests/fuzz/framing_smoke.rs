use meshchat_core::{LinkHandle, framing::*};
fn next(s: &mut u64) -> u64 {
    *s ^= *s << 13;
    *s ^= *s >> 7;
    *s ^= *s << 17;
    *s
}
#[test]
fn framing_mutation_smoke() {
    let mut corpus = vec![vec![], vec![0; 4], vec![0; 513]];
    for line in include_str!("../vectors/base/logical.txt")
        .lines()
        .filter(|l| !l.starts_with('#'))
    {
        let parts: Vec<_> = line.split_whitespace().collect();
        if parts[1] != "live" || parts[2] != "ok" {
            continue;
        }
        let raw: Vec<u8> = parts[3]
            .as_bytes()
            .chunks_exact(2)
            .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
            .collect();
        for c in [146, 182, 512] {
            let encoder = Encoder::logical(&raw, c, 0).unwrap();
            for i in 0..encoder.frame_count() {
                let mut out = [0; 512];
                let n = encoder.frame(i, &mut out).unwrap();
                corpus.push(out[..n].to_vec());
            }
        }
    }
    let mut r = Reassembler::new(7, 8, 0).unwrap();
    let links: Vec<_> = (1..=8)
        .map(|generation| LinkHandle {
            instance_nonce: 7,
            generation,
        })
        .collect();
    for link in &links {
        r.register(link, 512).unwrap();
    }
    let baseline = r.reservations();
    let mut state = 0x4d43_3031_305f_7631;
    let mut out = [0; 1035];
    for step in 0..100_000u64 {
        let mut data = corpus[next(&mut state) as usize % corpus.len()].clone();
        for _ in 0..next(&mut state) % 8 {
            let at = next(&mut state) as usize % (data.len() + 1);
            match next(&mut state) % 4 {
                0 if data.len() < 600 => data.insert(at, next(&mut state) as u8),
                1 if at < data.len() => {
                    data.remove(at);
                }
                2 if at < data.len() => data[at] ^= next(&mut state) as u8,
                _ => data.truncate(at),
            }
        }
        if step % 2 == 0 && data.len() >= 4 {
            let len = (data.len() - 4) as u16;
            data[2..4].copy_from_slice(&len.to_be_bytes());
        }
        for capacity in [145, 146, 182, 512] {
            let _ = parse_frame(&data, capacity);
        }
        let _ = bootstrap_hello(&data, 512);
        for kind in 0..=4 {
            let _ = transport(kind, &data);
        }
        let link = &links[next(&mut state) as usize % links.len()];
        if let Ok(Some(done)) = r.ingest_admitted(link, &data, step * 10, &mut out) {
            assert!(done.len <= 1035);
            match done.kind {
                ObjectKind::Logical => {
                    meshchat_core::codec::parse(
                        &out[..done.len],
                        meshchat_core::codec::Context::Live,
                    )
                    .unwrap();
                }
                ObjectKind::Transport(kind) => {
                    transport(kind, &out[..done.len]).unwrap();
                }
            }
        }
        let p = r.reservations();
        assert!(p.logical_groups <= 64 && p.transport_groups <= 32 && p.rejected_groups <= 512);
        assert_eq!(
            (p.logical_bytes, p.transport_bytes, p.rejected_bytes),
            (
                baseline.logical_bytes,
                baseline.transport_bytes,
                baseline.rejected_bytes
            )
        );
    }
    r.advance(1_030_000).unwrap();
    let p = r.reservations();
    assert_eq!(
        (p.logical_groups, p.transport_groups, p.rejected_groups),
        (0, 0, 0)
    );
}
