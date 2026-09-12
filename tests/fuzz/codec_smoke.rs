//! Deterministic mutation fuzz target; no coverage-guided result is claimed.
use meshchat_core::codec::{self, Context};

/// Shared target: arbitrary bytes in both ingress contexts, bounded stack output.
fn fuzz_target(data: &[u8]) {
    for context in [Context::Live, Context::StoredChat] {
        if let Ok(p) = codec::parse(data, context) {
            assert!((26..=1024).contains(&data.len()));
            let mut out = [0; 1024];
            let n = codec::serialize(p.header(), p.payload_bytes(), context, &mut out).unwrap();
            assert_eq!(&out[..n], data);
            assert_eq!(codec::parse(&out[..n], context).unwrap(), p);
            if let Some(n) = p.forward_to(&mut out).unwrap() {
                assert_eq!(&out[..3], &data[..3]);
                assert_eq!(&out[4..n], &data[4..]);
                assert_eq!(out[3], data[3].min(7) - 1);
            }
        }
        let _ = codec::credential(data);
    }
}
fn next(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}
#[test]
fn deterministic_mutation_smoke() {
    let corpus: Vec<Vec<u8>> = include_str!("../vectors/base/logical.txt")
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|line| {
            let hex = line.split_whitespace().nth(3).unwrap();
            if hex == "-" {
                return vec![];
            }
            hex.as_bytes()
                .chunks_exact(2)
                .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
                .collect()
        })
        .collect();
    let mut state = 0x4d43_3030_395f_7631;
    for seed in &corpus {
        fuzz_target(seed);
        for end in 0..seed.len() {
            fuzz_target(&seed[..end]);
        }
        for index in 0..seed.len() {
            let mut mutated = seed.clone();
            mutated[index] ^= 0xff;
            fuzz_target(&mutated);
        }
    }
    for attempt in 0..100_000 {
        let mut data = corpus[(next(&mut state) as usize) % corpus.len()].clone();
        for _ in 0..1 + (next(&mut state) % 8) {
            let at = (next(&mut state) as usize) % (data.len() + 1);
            match next(&mut state) % 4 {
                0 if data.len() < 1100 => data.insert(at, next(&mut state) as u8),
                1 if at < data.len() => {
                    data.remove(at);
                }
                2 if at < data.len() => data[at] ^= next(&mut state) as u8,
                _ => data.truncate(at),
            }
        }
        // Retain deep parser coverage despite mutations to length fields.
        if attempt % 2 == 0 && data.len() >= 26 {
            let len = (data.len() - 26) as u16;
            data[24..26].copy_from_slice(&len.to_be_bytes());
        }
        fuzz_target(&data);
    }
    for len in 0..=1100 {
        let data: Vec<u8> = (0..len).map(|_| next(&mut state) as u8).collect();
        fuzz_target(&data);
    }
}
