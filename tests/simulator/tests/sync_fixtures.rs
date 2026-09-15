// This selector test shares ingress helpers; receiver helpers are used by core tests.
#[allow(dead_code)]
#[path = "../../integration/sync/admission.rs"]
mod admission;
use admission::RawSessions;
use meshchat_core::{
    LinkHandle,
    codec::{self, Context, Header},
    framing::{self, Transport},
    power::Mode,
    sync::{Cache, CacheState, bloom_contains, session::*},
};
use serde_json::Value;
fn hex(s: &str) -> Vec<u8> {
    s.as_bytes()
        .chunks_exact(2)
        .map(|x| u8::from_str_radix(std::str::from_utf8(x).unwrap(), 16).unwrap())
        .collect()
}
fn request(id: u16, cursor: u32, count: u16, filter: &[u8; 512]) -> Vec<u8> {
    let mut p = [0; 520];
    p[..2].copy_from_slice(&id.to_be_bytes());
    p[2..4].copy_from_slice(&count.to_be_bytes());
    p[4..8].copy_from_slice(&cursor.to_be_bytes());
    p[8..].copy_from_slice(filter);
    let mut out = vec![0; 546];
    codec::serialize(
        Header {
            kind: 3,
            flags: 0,
            ttl: 1,
            message_id: [1; 8],
            sender_id: [2; 8],
            channel_id: [0; 4],
        },
        &p,
        Context::Live,
        &mut out,
    )
    .unwrap();
    out
}
#[test]
fn fixed_bloom_selection_is_subscription_neutral_and_retries_retain_false_positive() {
    let fixtures: Value =
        serde_json::from_str(include_str!("../scenarios/MC-007-sync-fixtures.json")).unwrap();
    let line = include_str!("../../vectors/base/logical.txt")
        .lines()
        .find(|l| l.starts_with("max-chat "))
        .unwrap();
    let template = hex(line.split_whitespace().nth(3).unwrap());
    for case in fixtures["cases"].as_array().unwrap() {
        let filter: [u8; 512] = hex(case["request_bloom_hex"].as_str().unwrap())
            .try_into()
            .unwrap();
        let count = case["request_item_count"].as_u64().unwrap() as u16;
        let mut c = Cache::new(Mode::Normal, 0).unwrap();
        for record in case["responder_newest_first"]
            .as_array()
            .unwrap()
            .iter()
            .rev()
        {
            let mut b = template.clone();
            let id = hex(record["msg_id"].as_str().unwrap());
            b[4..12].copy_from_slice(&id);
            b[12..20].copy_from_slice(&id);
            b[20..24].copy_from_slice(&hex(record["channel_id"].as_str().unwrap()));
            b[3] = 0;
            c.insert_admitted(&b, CacheState::Unverified, 0).unwrap();
        }
        let link = LinkHandle {
            instance_nonce: 1,
            generation: 1,
        };
        let mut s = Sessions::new(1, 0).unwrap();
        s.register(&link, 0).unwrap();
        let expected: Vec<_> = case["expected_selected_ids"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_str().unwrap().to_owned())
            .collect();
        for walk in 1..=2 {
            let now = u64::from(walk) * 60000;
            s.accept_request(
                &link,
                &request(walk, 0, count, &filter),
                &mut c,
                now,
                &mut |_| {},
            )
            .unwrap();
            let mut selected = vec![];
            let mut markers = vec![];
            let mut bytes = 0;
            loop {
                let mut raw = [0; 1035];
                let sent = s
                    .next_served(&link, &mut c, now, &mut raw, &mut |_| {})
                    .unwrap()
                    .unwrap();
                let Transport::SyncItem {
                    flags,
                    next_cursor,
                    blob,
                    ..
                } = framing::transport(1, &raw[..sent.len]).unwrap()
                else {
                    panic!()
                };
                if flags == 0 {
                    assert_eq!(blob[3], 0);
                    bytes += blob.len();
                    selected.push(format!(
                        "{:016x}",
                        u64::from_be_bytes(blob[4..12].try_into().unwrap())
                    ));
                } else {
                    markers.push(flags);
                }
                s.served_complete(sent.token, true, now, &mut |_| {})
                    .unwrap();
                if flags == 3 {
                    s.accept_request(
                        &link,
                        &request(walk, next_cursor, count, &filter),
                        &mut c,
                        now,
                        &mut |_| {},
                    )
                    .unwrap();
                }
                if flags == 5 || flags == 7 {
                    break;
                }
            }
            assert_eq!(selected, expected);
            assert_eq!(
                bytes,
                case["expected_selected_encoded_bytes"].as_u64().unwrap() as usize
            );
            assert_eq!(
                markers,
                case["expected_page_marker_flags"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_u64().unwrap() as u8)
                    .collect::<Vec<_>>()
            );
            for omitted in case["expected_bloom_false_positive_omissions"]
                .as_array()
                .unwrap()
            {
                assert!(bloom_contains(
                    &filter,
                    &hex(omitted.as_str().unwrap()).try_into().unwrap()
                ));
                assert!(!selected.contains(&omitted.as_str().unwrap().to_owned()));
            }
        }
    }
}
