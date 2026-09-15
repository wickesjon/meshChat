mod admission;
use admission::RawSessions;
use meshchat_core::{
    LinkHandle,
    codec::{self, Context, Header, Payload},
    framing::{self, Transport},
    power::Mode,
    sync::session::*,
    sync::{Cache, CacheState},
};
fn link(generation: u64) -> LinkHandle {
    LinkHandle {
        instance_nonce: 7,
        generation,
    }
}
fn body(id: u64) -> Vec<u8> {
    let l = include_str!("../../vectors/base/logical.txt")
        .lines()
        .find(|l| l.starts_with("max-chat "))
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
fn req(id: u16, cursor: u32, bloom: &[u8; 512]) -> Vec<u8> {
    let mut payload = [0; 520];
    payload[..2].copy_from_slice(&id.to_be_bytes());
    payload[4..8].copy_from_slice(&cursor.to_be_bytes());
    payload[8..].copy_from_slice(bloom);
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
        &payload,
        Context::Live,
        &mut out,
    )
    .unwrap();
    out
}
fn setup(n: u64) -> (Sessions, Cache) {
    let mut s = Sessions::new(7, 0).unwrap();
    for g in 1..=n {
        s.register(&link(g), 0).unwrap();
    }
    (s, Cache::new(Mode::Normal, 0).unwrap())
}
#[test]
fn serving_pages_budget_marker_exact_stored_ttl_and_single_use_bound_cursor() {
    let (mut s, mut c) = setup(2);
    let mut events = vec![];
    for id in 1..=9 {
        let mut b = body(id);
        b[3] = id as u8 % 8;
        c.insert_admitted(&b, CacheState::Unverified, 0).unwrap();
    }
    s.accept_request(&link(1), &req(5, 0, &[0; 512]), &mut c, 0, &mut |e| {
        events.push(e)
    })
    .unwrap();
    let mut out = [0; 1035];
    let mut count = 0;
    let mut cursor = 0;
    for seq in 0..10 {
        let sent = s
            .next_served(&link(1), &mut c, seq, &mut out, &mut |e| events.push(e))
            .unwrap()
            .unwrap();
        let Transport::SyncItem {
            sequence,
            flags,
            next_cursor,
            blob,
            ..
        } = framing::transport(1, &out[..sent.len]).unwrap()
        else {
            panic!()
        };
        assert_eq!(sequence, seq as u16);
        if flags == 0 {
            count += 1;
            let id = u64::from_be_bytes(blob[4..12].try_into().unwrap());
            assert_eq!(id, 10 - count);
            assert_eq!(blob[3], id as u8 % 8);
        } else if flags == 3 {
            assert_eq!(count, 4);
            cursor = next_cursor;
        } else {
            assert_eq!(flags, 7);
            assert_eq!(count, 8);
        }
        assert!(
            s.next_served(&link(1), &mut c, seq, &mut [0; 1035], &mut |_| {})
                .unwrap()
                .is_none()
        );
        s.served_complete(sent.token, true, seq, &mut |e| events.push(e))
            .unwrap();
        if sent.flags == 3 {
            assert_eq!(
                s.accept_request(
                    &link(2),
                    &req(5, cursor, &[0; 512]),
                    &mut c,
                    seq,
                    &mut |_| {}
                ),
                Err(Error::Stale)
            );
            assert_eq!(
                s.accept_request(
                    &link(1),
                    &req(5, cursor, &[0; 512]),
                    &mut c,
                    seq,
                    &mut |_| {}
                )
                .unwrap(),
                RequestOutcome::Continued
            );
            assert_eq!(
                s.accept_request(
                    &link(1),
                    &req(5, cursor, &[0; 512]),
                    &mut c,
                    seq,
                    &mut |_| {}
                ),
                Err(Error::Stale)
            );
        }
    }
    assert_eq!(events.last().unwrap().end, End::Truncated);
    assert_eq!(
        s.accept_request(&link(1), &req(5, 0, &[0; 512]), &mut c, 10, &mut |_| {}),
        Err(Error::Stale)
    );
}
#[test]
fn empty_and_expired_snapshots_terminate_without_reaccess_extending_age() {
    let (mut s, mut c) = setup(1);
    let mut events = vec![];
    c.insert_admitted(&body(1), CacheState::Unverified, 0)
        .unwrap();
    s.accept_request(
        &link(1),
        &req(1, 0, &[0; 512]),
        &mut c,
        899_999,
        &mut |_| {},
    )
    .unwrap();
    c.insert_admitted(&body(2), CacheState::Unverified, 900_000)
        .unwrap();
    let mut out = [0; 1035];
    let sent = s
        .next_served(&link(1), &mut c, 900_000, &mut out, &mut |_| {})
        .unwrap()
        .unwrap();
    assert_eq!(sent.len, 11);
    assert_eq!(sent.flags, 5);
    s.served_complete(sent.token, true, 900_000, &mut |e| events.push(e))
        .unwrap();
    assert_eq!(events[0].items, 0);
    assert_eq!(events[0].end, End::Complete);
}
#[test]
fn serving_global_caps_refused_id_replay_and_disconnect_tokens_fail_closed() {
    let (mut s, mut c) = setup(3);
    for g in 1..=2 {
        s.accept_request(&link(g), &req(7, 0, &[0; 512]), &mut c, 0, &mut |_| {})
            .unwrap();
    }
    assert_eq!(
        s.accept_request(&link(3), &req(7, 0, &[0; 512]), &mut c, 0, &mut |_| {}),
        Err(Error::Full)
    );
    assert_eq!(
        s.accept_request(&link(1), &req(8, 0, &[0; 512]), &mut c, 0, &mut |_| {}),
        Err(Error::Full)
    );
    let sent = s
        .next_served(&link(1), &mut c, 0, &mut [0; 1035], &mut |_| {})
        .unwrap()
        .unwrap();
    s.disconnect(&link(1), 1, &mut |_| {}).unwrap();
    assert_eq!(
        s.served_complete(sent.token, true, 1, &mut |_| {}),
        Err(Error::Stale)
    );
    assert_eq!(
        s.accept_request(&link(3), &req(7, 0, &[0; 512]), &mut c, 1, &mut |_| {}),
        Err(Error::Stale)
    );
    s.register(&link(4), 1).unwrap();
    s.accept_request(&link(4), &req(8, 0, &[0; 512]), &mut c, 1, &mut |_| {})
        .unwrap();
    let r = s.reservations();
    assert_eq!(r.serving, 2);
    assert!(r.snapshot_bytes <= 64 * 1024);
    assert!(r.allocated_bytes <= 256 * 1024);
}
#[test]
fn altered_filter_aborts_and_replayed_initial_never_refreshes_deadline() {
    let (mut s, mut c) = setup(1);
    s.accept_request(&link(1), &req(1, 0, &[0; 512]), &mut c, 0, &mut |_| {})
        .unwrap();
    assert_eq!(
        s.accept_request(
            &link(1),
            &req(1, 0, &[0; 512]),
            &mut c,
            119_999,
            &mut |_| {}
        )
        .unwrap(),
        RequestOutcome::Duplicate
    );
    let mut events = vec![];
    s.advance(120_000, &mut |e| events.push(e)).unwrap();
    assert_eq!(events[0].end, End::TimedOut);
    s.accept_request(
        &link(1),
        &req(2, 0, &[0; 512]),
        &mut c,
        120_000,
        &mut |_| {},
    )
    .unwrap();
    assert_eq!(
        s.accept_request(&link(1), &req(2, 0, &[1; 512]), &mut c, 120_000, &mut |e| {
            events.push(e)
        }),
        Err(Error::Invalid)
    );
    assert_eq!(events.last().unwrap().end, End::Conflict);
}
use meshchat_core::ingress::{Ingress, State};
fn ingress() -> Ingress {
    let mut i = Ingress::new(7, 8, 0).unwrap();
    i.register(&link(1), 512, 0).unwrap();
    i
}
fn start(s: &mut Sessions, i: &mut Ingress) -> Requested {
    let mut out = [0; 546];
    let p = s
        .request(
            &link(1),
            Request {
                identity: Identity {
                    message_id: [3; 8],
                    sender_id: [4; 8],
                },
                held_count: 0,
                filter: [0; 512],
            },
            i,
            0,
            &mut out,
        )
        .unwrap();
    s.request_started(p.token, 0).unwrap();
    p
}
fn response(session: u16, sequence: u16, flags: u8, cursor: u32, blob: &[u8]) -> Vec<u8> {
    let mut raw = vec![0; 11 + blob.len()];
    raw[..2].copy_from_slice(&session.to_be_bytes());
    raw[2..4].copy_from_slice(&sequence.to_be_bytes());
    raw[4] = flags;
    raw[5..9].copy_from_slice(&cursor.to_be_bytes());
    raw[9..11].copy_from_slice(&(blob.len() as u16).to_be_bytes());
    raw[11..].copy_from_slice(blob);
    raw
}
#[test]
fn deferred_session_order_prevents_unsolicited_inner_state_and_false_completion() {
    let (mut s, _) = setup(1);
    let mut i = ingress();
    let mut events = vec![];
    let mut messages = vec![];
    let raw = response(1, 0, 0, 0, &body(1));

    assert_eq!(i.reservations().accepted, 0);
    assert_eq!(
        s.receive(
            &link(1),
            &raw,
            &mut i,
            0,
            &mut Sink {
                events: &mut |e| events.push(e),
                messages: &mut |b, state| messages.push((b.to_vec(), state))
            }
        ),
        Err(Error::Stale)
    );
    assert!(messages.is_empty());
    assert_eq!(i.reservations().accepted, 0);
    start(&mut s, &mut i);
    let marker = response(1, 1, 5, 0, &[]);
    assert_eq!(
        s.receive(
            &link(1),
            &marker,
            &mut i,
            0,
            &mut Sink {
                events: &mut |e| events.push(e),
                messages: &mut |b, state| messages.push((b.to_vec(), state))
            }
        )
        .unwrap(),
        Received::Buffered
    );
    assert!(events.is_empty());
    assert_eq!(i.reservations().accepted, 0);
    assert_eq!(
        s.receive(
            &link(1),
            &raw,
            &mut i,
            0,
            &mut Sink {
                events: &mut |e| events.push(e),
                messages: &mut |b, state| messages.push((b.to_vec(), state))
            }
        )
        .unwrap(),
        Received::Complete { truncated: false }
    );
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].1, State::Unverified);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].items, 1);
    assert_eq!(events[0].end, End::Complete);
    assert_eq!(i.counters().admitted_frames, 3);
    assert_eq!(i.reservations().accepted, 1);
}
#[test]
fn duplicate_and_conflicting_responses_are_exact_and_gap_deadline_does_not_refresh() {
    let (mut s, _) = setup(1);
    let mut i = ingress();
    start(&mut s, &mut i);
    let mut events = vec![];
    let mut messages = 0;
    let raw = response(1, 1, 0, 0, &body(2));
    for t in [0, 29_999] {
        let result = s
            .receive(
                &link(1),
                &raw,
                &mut i,
                t,
                &mut Sink {
                    events: &mut |e| events.push(e),
                    messages: &mut |_, _| messages += 1,
                },
            )
            .unwrap();
        assert_eq!(
            result,
            if t == 0 {
                Received::Buffered
            } else {
                Received::Duplicate
            }
        );
    }
    s.advance(30_000, &mut |e| events.push(e)).unwrap();
    assert_eq!(messages, 0);
    assert_eq!(events.last().unwrap().end, End::TimedOut);
    assert_eq!(i.reservations().accepted, 0);
    // New instance for the exact processed-duplicate/conflict case.
    let (mut s, _) = setup(1);
    let mut i = ingress();
    start(&mut s, &mut i);
    let raw = response(1, 0, 0, 0, &body(1));
    for _ in 0..2 {
        s.receive(
            &link(1),
            &raw,
            &mut i,
            0,
            &mut Sink {
                events: &mut |_| {},
                messages: &mut |_, _| messages += 1,
            },
        )
        .unwrap();
    }
    assert_eq!(messages, 1);
    let conflict = response(1, 0, 0, 0, &body(2));
    assert_eq!(
        s.receive(
            &link(1),
            &conflict,
            &mut i,
            0,
            &mut Sink {
                events: &mut |e| events.push(e),
                messages: &mut |_, _| messages += 1
            }
        ),
        Err(Error::Invalid)
    );
    assert_eq!(messages, 1);
    assert_eq!(events.last().unwrap().end, End::Conflict);
}
#[test]
fn continuation_preserves_filter_session_and_deadline_and_old_session_does_not_bind() {
    let (mut s, _) = setup(1);
    let mut i = ingress();
    let p = start(&mut s, &mut i);
    let mut events = vec![];
    let marker = response(1, 0, 3, 123, &[]);
    assert_eq!(
        s.receive(
            &link(1),
            &marker,
            &mut i,
            1,
            &mut Sink {
                events: &mut |_| {},
                messages: &mut |_, _| panic!()
            }
        )
        .unwrap(),
        Received::Page { cursor: 123 }
    );
    let mut out = [0; 546];
    let c = s
        .continuation(
            &link(1),
            Identity {
                message_id: [5; 8],
                sender_id: [4; 8],
            },
            2,
            &mut out,
        )
        .unwrap();
    assert_eq!(c.token, p.token);
    let Payload::SyncRequest {
        session_id,
        cursor,
        bloom,
        ..
    } = codec::parse(&out, Context::Live).unwrap().payload()
    else {
        panic!()
    };
    assert_eq!((session_id, cursor), (1, 123));
    assert_eq!(bloom, [0; 512]);
    assert_eq!(
        s.continuation(
            &link(1),
            Identity {
                message_id: [5; 8],
                sender_id: [4; 8]
            },
            2,
            &mut out
        )
        .unwrap_err(),
        Error::Stale
    );
    let stale = response(2, 1, 5, 0, &[]);
    assert_eq!(
        s.receive(
            &link(1),
            &stale,
            &mut i,
            3,
            &mut Sink {
                events: &mut |_| {},
                messages: &mut |_, _| panic!()
            }
        ),
        Err(Error::Stale)
    );
    assert_eq!(s.reservations().requesting, 1);
    s.request_started(c.token, 119_999).unwrap();
    s.advance(120_000, &mut |e| events.push(e)).unwrap();
    assert_eq!(events[0].end, End::TimedOut);
    assert_eq!(s.reservations().requesting, 0);
}
#[test]
fn bounded_gap_pool_does_not_admit_inner_packets() {
    let (mut s, _) = setup(1);
    let mut i = ingress();
    start(&mut s, &mut i);
    let mut events = vec![];
    for sequence in 1..=4 {
        let raw = response(1, sequence, 0, 0, &body(u64::from(sequence)));
        assert_eq!(
            s.receive(
                &link(1),
                &raw,
                &mut i,
                0,
                &mut Sink {
                    events: &mut |_| {},
                    messages: &mut |_, _| panic!()
                }
            )
            .unwrap(),
            Received::Buffered
        );
    }
    let r = s.reservations();
    assert_eq!(r.gap_objects, 4);
    assert!(r.gap_bytes_per_session <= 8 * 1024);
    assert!(r.allocated_bytes <= 256 * 1024);
    assert_eq!(i.reservations().accepted, 0);
    let raw = response(1, 5, 0, 0, &body(5));
    assert_eq!(
        s.receive(
            &link(1),
            &raw,
            &mut i,
            0,
            &mut Sink {
                events: &mut |e| events.push(e),
                messages: &mut |_, _| panic!()
            }
        ),
        Err(Error::Invalid)
    );
    assert_eq!(s.reservations().requesting, 0);
    assert_eq!(events[0].end, End::Failed);
}
#[test]
fn encoded_byte_sizing_bounds_and_failed_native_walk_are_explicit() {
    // Arithmetic witness only: the current largest valid v1 CHAT is 556 bytes.
    for items in 0..8 {
        assert!(fits_session_budget(items, u32::from(items) * 1024, 1024));
    }
    assert!(!fits_session_budget(8, 8192, 1));
    assert!(fits_session_budget(7, 7168, 1024));
    assert!(!fits_session_budget(7, 7168, 1025));
    assert!(!fits_session_budget(0, u32::MAX, usize::MAX));
    let (mut s, mut c) = setup(1);
    s.accept_request(&link(1), &req(1, 0, &[0; 512]), &mut c, 0, &mut |_| {})
        .unwrap();
    let sent = s
        .next_served(&link(1), &mut c, 0, &mut [0; 1035], &mut |_| {})
        .unwrap()
        .unwrap();
    let mut events = vec![];
    s.served_complete(sent.token, false, 1, &mut |e| events.push(e))
        .unwrap();
    assert_eq!(events[0].end, End::Failed);
    assert_eq!(s.reservations().serving, 0);
    assert_eq!(
        s.accept_request(&link(1), &req(1, 0, &[0; 512]), &mut c, 1, &mut |_| {}),
        Err(Error::Stale)
    );
}
#[test]
fn requester_caps_session_bucket_and_cancel_do_not_grant_fresh_credit() {
    let (mut s, _) = setup(3);
    let mut i = Ingress::new(7, 3, 0).unwrap();
    for g in 1..=3 {
        i.register(&link(g), 512, 0).unwrap();
    }
    let request = || Request {
        identity: Identity {
            message_id: [1; 8],
            sender_id: [2; 8],
        },
        held_count: 0,
        filter: [0; 512],
    };
    let p = s
        .request(&link(1), request(), &mut i, 0, &mut [0; 546])
        .unwrap();
    let q = s
        .request(&link(2), request(), &mut i, 0, &mut [0; 546])
        .unwrap();
    assert_eq!(
        s.request(&link(3), request(), &mut i, 0, &mut [0; 546])
            .unwrap_err(),
        Error::Full
    );
    s.cancel_request(p.token, 0, &mut |_| {}).unwrap();
    assert_eq!(
        s.request(&link(1), request(), &mut i, 0, &mut [0; 546])
            .unwrap_err(),
        Error::Full
    );
    s.disconnect(&link(2), 1, &mut |_| {}).unwrap();
    assert_eq!(s.request_started(q.token, 1), Err(Error::Stale));
    let r = s
        .request(&link(1), request(), &mut i, 60_000, &mut [0; 546])
        .unwrap();
    assert_ne!(p.token, r.token);
}
#[test]
fn embedded_sync_cannot_bypass_sender_bucket_or_claim_completion_after_rejection() {
    let (mut s, _) = setup(1);
    let mut i = ingress();
    start(&mut s, &mut i);
    let mut events = vec![];
    let mut messages = 0;
    for seq in 0..7 {
        let raw = if seq == 4 {
            response(1, seq, 3, 9, &[])
        } else {
            response(1, seq, 0, 0, &body(u64::from(seq) + 1))
        };

        let result = s.receive(
            &link(1),
            &raw,
            &mut i,
            0,
            &mut Sink {
                events: &mut |e| events.push(e),
                messages: &mut |_, _| messages += 1,
            },
        );
        if seq == 4 {
            s.continuation(
                &link(1),
                Identity {
                    message_id: [5; 8],
                    sender_id: [4; 8],
                },
                0,
                &mut [0; 546],
            )
            .unwrap();
        } else if seq == 6 {
            assert_eq!(result, Err(Error::Invalid));
        } else {
            assert_eq!(result.unwrap(), Received::Processed);
        }
    }
    assert_eq!(messages, 5);
    assert_eq!(i.counters().budget_drops, 1);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].end, End::Admission);
    assert_eq!(s.reservations().requesting, 0);
}
#[test]
fn admission_tokens_bind_kind_link_and_dispatch_time_and_drops_issue_none() {
    let (mut s, mut c) = setup(2);
    let mut i = ingress();
    let mut out = [0; 1035];
    let raw = req(1, 0, &[0; 512]);
    let token = admission::admission(&mut i, &link(1), &raw, false, 0, &mut out);
    assert_eq!(
        s.accept_admitted_request(&link(2), token, &mut c, 0, &mut |_| {}),
        Err(Error::Stale)
    );
    assert_eq!(s.reservations().serving, 0);
    let token = admission::admission(&mut i, &link(1), &raw, false, 0, &mut out);
    assert_eq!(
        s.accept_admitted_request(&link(1), token, &mut c, 1, &mut |_| {}),
        Err(Error::Stale)
    );
    let token = admission::admission(
        &mut i,
        &link(1),
        &response(1, 0, 5, 0, &[]),
        true,
        0,
        &mut out,
    );
    assert_eq!(
        s.accept_admitted_request(&link(1), token, &mut c, 0, &mut |_| {}),
        Err(Error::Stale)
    );
    assert_eq!(s.reservations().serving, 0);
    let packet = framing::Encoder::transport(1, &[0; 11], 512, 1);
    assert!(packet.is_err());
    // Exhaust outer admission without producing a valid completion capability.
    for _ in 0..20 {
        let got = i
            .receive_deferred_sync(&link(1), 1, &[255], 0, &mut out)
            .unwrap();
        assert!(got.sync.is_none());
    }
    assert!(i.counters().budget_drops > 0);
    assert_eq!(s.reservations().requesting, 0);
}
