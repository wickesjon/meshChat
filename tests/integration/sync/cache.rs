use meshchat_core::{power::Mode, sync::*};
fn fixture(name: &str, id: u64) -> Vec<u8> {
    let l = include_str!("../../vectors/base/logical.txt")
        .lines()
        .find(|l| l.split_whitespace().next() == Some(name))
        .unwrap();
    let mut b: Vec<u8> = l
        .split_whitespace()
        .nth(3)
        .unwrap()
        .as_bytes()
        .chunks_exact(2)
        .map(|x| u8::from_str_radix(std::str::from_utf8(x).unwrap(), 16).unwrap())
        .collect();
    b[4..12].copy_from_slice(&id.to_be_bytes());
    b
}
fn added(i: Insert) -> u64 {
    if let Insert::Added(n) = i {
        n
    } else {
        panic!("{i:?}")
    }
}
#[test]
fn cache_expiry_snapshot_mutation_and_replay_keep_first_arrival_and_ttl() {
    let mut c = Cache::new(Mode::Normal, 0).unwrap();
    let mut b = fixture("max-chat", 1);
    b[3] = 0;
    let a = added(c.insert_admitted(&b, CacheState::Unverified, 0).unwrap());
    let d = added(
        c.insert_admitted(&fixture("max-chat", 2), CacheState::Unverified, 10)
            .unwrap(),
    );
    let mut snapshot = [0; 500];
    assert_eq!(c.snapshot(10, &mut snapshot).unwrap(), 2);
    assert_eq!(&snapshot[..2], &[d, a]);
    let newer = added(
        c.insert_admitted(&fixture("max-chat", 3), CacheState::Unverified, 20)
            .unwrap(),
    );
    assert!(!snapshot[..2].contains(&newer));
    let mut out = [0; 1024];
    assert_eq!(c.copy(a, 899_999, &mut out).unwrap().0, b.len());
    assert_eq!(out[3], 0);
    assert_eq!(
        c.insert_admitted(&b, CacheState::Unverified, 899_999)
            .unwrap(),
        Insert::Existing(a)
    );
    assert_eq!(c.copy(a, 900_000, &mut out), Err(Error::Missing));
    assert_eq!(
        c.insert_admitted(&b, CacheState::Unverified, 900_001)
            .unwrap(),
        Insert::Expired
    );
    assert!(c.copy(d, 900_001, &mut out).is_ok());
    assert_eq!(c.copy(d, 900_010, &mut out), Err(Error::Missing));
}
#[test]
fn cache_count_and_byte_caps_lru_and_compaction_preserve_exact_bytes() {
    let mut c = Cache::new(Mode::Normal, 0).unwrap();
    let mut first = 0;
    for id in 1..=800 {
        let b = fixture("max-organizer", id);
        let s = added(c.insert_admitted(&b, CacheState::Pending, id).unwrap());
        if id == 1 {
            first = s;
        }
        let r = c.reservations();
        assert!(
            r.entries <= 500 && r.encoded_bytes <= 256 * 1024 && r.allocated_bytes <= 320 * 1024
        );
        let mut out = [0; 1024];
        let (n, state) = c.copy(s, id, &mut out).unwrap();
        assert_eq!(&out[..n], b);
        assert_eq!(state, CacheState::Pending);
    }
    assert_eq!(c.copy(first, 800, &mut [0; 1024]), Err(Error::Missing));
    let mut seq = [0; 500];
    let count = c.snapshot(800, &mut seq).unwrap();
    assert_eq!(count, 256 * 1024 / 556);
    for &s in &seq[..count] {
        let mut out = [0; 1024];
        let (n, _) = c.copy(s, 800, &mut out).unwrap();
        assert_eq!(&out[..n], fixture("max-organizer", s));
    }
}
#[test]
fn cache_storable_states_and_bloom_are_explicit() {
    let mut c = Cache::new(Mode::Normal, 0).unwrap();
    assert_eq!(
        c.insert_admitted(&fixture("reaction", 1), CacheState::Unverified, 0),
        Err(Error::Invalid)
    );
    assert_eq!(
        c.insert_admitted(&fixture("friend-included", 1), CacheState::Unverified, 0),
        Err(Error::Invalid)
    );
    c.insert_admitted(&fixture("encrypted-chat-121", 2), CacheState::Pending, 0)
        .unwrap();
    let (n, bloom) = c.bloom(0).unwrap();
    assert_eq!(n, 1);
    assert!(bloom_contains(&bloom, &2u64.to_be_bytes()));
    assert_eq!(c.bloom(900_000).unwrap(), (0, [0; 512]));
}
#[test]
fn cache_mode_changes_do_not_resurrect_expired_payloads_and_fit_new_caps() {
    let mut c = Cache::new(Mode::Beacon, 0).unwrap();
    for id in 1..=501 {
        c.insert_admitted(&fixture("max-chat", id), CacheState::Unverified, 0)
            .unwrap();
    }
    assert!(c.reservations().allocated_bytes <= 6 * 1024 * 1024);
    c.set_mode(Mode::Normal, 0).unwrap();
    assert_eq!(c.reservations().entries, 500);
    assert!(c.reservations().allocated_bytes <= 320 * 1024);
    c.advance(900_000).unwrap();
    c.set_mode(Mode::Beacon, 900_000).unwrap();
    assert_eq!(c.reservations().entries, 0);
    assert_eq!(
        c.insert_admitted(&fixture("max-chat", 501), CacheState::Unverified, 900_000)
            .unwrap(),
        Insert::Expired
    );
    assert_eq!(c.copy(501, 899_999, &mut [0; 1024]), Err(Error::Time));
}
