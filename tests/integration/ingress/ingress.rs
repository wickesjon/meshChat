use meshchat_core::{LinkHandle, framing, ingress::*};
#[path = "../crypto/text_cases.rs"]
mod text_cases;

#[test]
fn unsigned_text_rejection_precedes_accepted_dedup_and_valid_variant_recovers() {
    for forbidden in text_cases::FORBIDDEN {
        for nickname in [false, true] {
            let mut ingress = setup(1);
            let good = text_cases::replace(&fixture("chat"), "e\u{301}", Some("雪 e\u{301}"));
            let bad = text_cases::replace(
                &good,
                if nickname { forbidden } else { "A" },
                Some(if nickname { "hello" } else { forbidden }),
            );
            assert_eq!(
                receive(&mut ingress, 1, &bad, 0),
                Outcome::Dropped(Drop::Malformed)
            );
            assert_eq!(ingress.counters().unverified, 0);
            assert_eq!(
                state(receive(&mut ingress, 1, &good, 1000)),
                State::Unverified
            );
        }
    }
}

fn link(generation: u64) -> LinkHandle {
    LinkHandle {
        instance_nonce: 7,
        generation,
    }
}
fn fixture(name: &str) -> Vec<u8> {
    let line = include_str!("../../vectors/base/logical.txt")
        .lines()
        .find(|l| l.split_whitespace().next() == Some(name))
        .unwrap();
    let hex = line.split_whitespace().nth(3).unwrap();
    hex.as_bytes()
        .chunks_exact(2)
        .map(|x| u8::from_str_radix(std::str::from_utf8(x).unwrap(), 16).unwrap())
        .collect()
}
fn whole(body: &[u8]) -> Vec<u8> {
    let mut raw = vec![0, 0];
    raw.extend_from_slice(&(body.len() as u16).to_be_bytes());
    raw.extend_from_slice(body);
    raw
}
fn unique(mut body: Vec<u8>, id: u64) -> Vec<u8> {
    body[4..12].copy_from_slice(&id.to_be_bytes());
    body
}
fn setup(links: u64) -> Ingress {
    let mut ingress = Ingress::new(7, 8, 0).unwrap();
    for generation in 1..=links {
        ingress.register(&link(generation), 512, 0).unwrap();
    }
    ingress
}
fn receive(ingress: &mut Ingress, generation: u64, body: &[u8], now: u64) -> Outcome {
    let raw = whole(body);
    ingress
        .receive(
            &link(generation),
            raw.len() as u64,
            &raw,
            now,
            &mut [0; 1035],
        )
        .unwrap()
}
fn state(outcome: Outcome) -> State {
    let Outcome::Complete { state, .. } = outcome else {
        panic!("{outcome:?}")
    };
    state
}

#[test]
fn charge_before_copy_reassembly_and_work_and_atomic_oversize() {
    let mut ingress = setup(1);
    assert_eq!(
        ingress
            .enqueue(&link(1), u64::MAX, &[], 0)
            .unwrap()
            .unwrap_err(),
        Drop::Budget
    );
    let raw = whole(&fixture("reaction"));
    for _ in 0..15 {
        ingress
            .receive(&link(1), raw.len() as u64, &raw, 0, &mut [0; 1035])
            .unwrap();
    }
    let before = ingress.reassembly_counters();
    assert_eq!(
        ingress
            .receive(&link(1), raw.len() as u64, &raw, 0, &mut [0; 1035])
            .unwrap(),
        Outcome::Dropped(Drop::Budget)
    );
    assert_eq!(ingress.reassembly_counters(), before);
    assert_eq!(ingress.counters().admitted_frames, 15);
    assert_eq!(ingress.counters().reserved_work_units, 0);
    assert_eq!(ingress.reservations().staging, 0);
    assert_eq!(ingress.counters().offered_bytes, u64::MAX);
    assert_eq!(
        ingress
            .enqueue(&link(1), 513, &[], 1000)
            .unwrap()
            .unwrap_err(),
        Drop::Size
    );
    assert_eq!(ingress.reservations().staging, 0);
}

#[test]
fn bounded_staging_tokens_and_disconnect_cleanup() {
    let mut ingress = setup(8);
    let raw = whole(&fixture("reaction"));
    let mut tokens = Vec::new();
    for generation in 1..=8 {
        for _ in 0..2 {
            tokens.push(
                ingress
                    .enqueue(&link(generation), raw.len() as u64, &raw, 0)
                    .unwrap()
                    .unwrap(),
            );
        }
    }
    assert_eq!(ingress.reservations().staging, 16);
    assert_eq!(
        ingress
            .enqueue(&link(1), raw.len() as u64, &raw, 0)
            .unwrap()
            .unwrap_err(),
        Drop::Staging
    );
    let token = tokens[0];
    ingress.process(token, 0, &mut [0; 1035]).unwrap();
    assert_eq!(ingress.process(token, 0, &mut [0; 1035]), Err(Error::Stale));
    ingress.disconnect(&link(2), 0).unwrap();
    assert_eq!(
        ingress.process(tokens[2], 0, &mut [0; 1035]),
        Err(Error::Stale)
    );
    assert_eq!(ingress.reservations().staging, 13);
    assert!(ingress.reservations().staging_bytes <= 16 * 1024);
}

#[test]
fn malformed_first_valid_second_pending_and_exact_replay() {
    for name in [
        "chat",
        "friend-included",
        "organizer-included",
        "encrypted-chat-121",
    ] {
        let mut ingress = setup(2);
        let good = fixture(name);
        let mut bad = good.clone();
        bad[0] = 2;
        assert_eq!(
            receive(&mut ingress, 1, &bad, 0),
            Outcome::Dropped(Drop::Malformed)
        );
        assert_eq!(
            receive(&mut ingress, 1, &bad, 0),
            Outcome::Dropped(Drop::RejectedVariant)
        );
        let expected = if name == "chat" {
            State::Unverified
        } else {
            State::Pending
        };
        assert_eq!(state(receive(&mut ingress, 2, &good, 0)), expected);
        let mut ttl = good.clone();
        ttl[3] = 6;
        assert_eq!(
            state(receive(&mut ingress, 1, &ttl, 0)),
            if name == "chat" {
                State::Duplicate
            } else {
                State::PendingDuplicate
            }
        );
        assert_eq!(ingress.reservations().accepted, usize::from(name == "chat"));
        assert_eq!(ingress.reservations().pending, usize::from(name != "chat"));
        assert_eq!(ingress.counters().reserved_work_units, 0);
    }
}

#[test]
fn same_claimed_id_does_not_hide_different_valid_variant() {
    let mut ingress = setup(1);
    let mut opaque = fixture("chat");
    opaque[1] = 255;
    assert_eq!(state(receive(&mut ingress, 1, &opaque, 0)), State::Opaque);
    assert_eq!(
        state(receive(&mut ingress, 1, &fixture("chat"), 0)),
        State::Unverified
    );
    let mut pending = fixture("friend-included");
    assert_eq!(state(receive(&mut ingress, 1, &pending, 0)), State::Pending);
    *pending.last_mut().unwrap() ^= 1; // Distinct structurally valid signature, same claimed ID.
    assert_eq!(state(receive(&mut ingress, 1, &pending, 0)), State::Pending);
    assert_eq!(ingress.reservations().pending, 2);
}

#[test]
fn pending_absolute_expiry_no_refresh_and_no_trusted_transition() {
    let mut ingress = setup(1);
    let body = fixture("friend-included");
    assert_eq!(state(receive(&mut ingress, 1, &body, 0)), State::Pending);
    assert_eq!(
        state(receive(&mut ingress, 1, &body, 29_999)),
        State::PendingDuplicate
    );
    ingress.advance(30_000).unwrap();
    assert_eq!(ingress.reservations().pending, 0);
    assert_eq!(ingress.reservations().accepted, 0);
    assert_eq!(
        state(receive(&mut ingress, 1, &body, 30_000)),
        State::Pending
    );
    assert!(ingress.pending_bytes(0).is_some());
    ingress.disconnect(&link(1), 30_000).unwrap();
    assert_eq!(ingress.reservations().pending, 0);
}

#[test]
fn claimed_sender_classes_shared_across_links_and_channels() {
    let mut ingress = setup(2);
    let confessions =
        meshchat_core::channel::Channel::Public(meshchat_core::channel::Public::Confessions).id();
    for id in 0..5 {
        let mut body = unique(fixture("chat"), id);
        if id % 2 == 1 {
            body[20..24].copy_from_slice(&confessions);
        }
        assert_eq!(
            state(receive(&mut ingress, 1 + id % 2, &body, 0)),
            State::Unverified
        );
    }
    assert_eq!(
        receive(&mut ingress, 2, &unique(fixture("chat"), 5), 0),
        Outcome::Dropped(Drop::Budget)
    );
    assert_eq!(
        state(receive(
            &mut ingress,
            2,
            &unique(fixture("chat"), 6),
            12_000
        )),
        State::Unverified
    );
    assert_eq!(ingress.reservations().senders, 1);
    // Invalids cannot reset sender credit, and rejections are not accepted dedup.
    assert_eq!(
        receive(&mut ingress, 1, &unique(fixture("chat"), 5), 12_000),
        Outcome::Dropped(Drop::Budget)
    );
    assert_eq!(
        state(receive(
            &mut ingress,
            1,
            &unique(fixture("chat"), 5),
            24_000
        )),
        State::Unverified
    );
}

#[test]
fn unknown_and_control_buckets_are_link_and_node_scoped() {
    let mut ingress = setup(8);
    for generation in 1..=4 {
        for id in 0..5 {
            assert_eq!(
                state(receive(
                    &mut ingress,
                    generation,
                    &unique(fixture("unknown-empty"), generation * 10 + id),
                    0
                )),
                State::Opaque
            );
        }
    }
    assert_eq!(
        receive(&mut ingress, 5, &unique(fixture("unknown-empty"), 50), 0),
        Outcome::Dropped(Drop::Budget)
    );
    assert_eq!(
        state(receive(
            &mut ingress,
            5,
            &unique(fixture("unknown-empty"), 50),
            1000
        )),
        State::Opaque
    );
    let mut ingress = setup(1);
    for name in ["announce", "event-info", "credential-offer"] {
        for id in 0..2 {
            assert!(matches!(
                receive(&mut ingress, 1, &unique(fixture(name), id), 0),
                Outcome::Complete { .. }
            ));
        }
        assert_eq!(
            receive(&mut ingress, 1, &unique(fixture(name), 2), 0),
            Outcome::Dropped(Drop::Budget)
        );
    }
    assert_eq!(
        state(receive(&mut ingress, 1, &fixture("credential-request"), 0)),
        State::Unverified
    );
    assert_eq!(
        receive(
            &mut ingress,
            1,
            &unique(fixture("credential-request"), 9),
            0
        ),
        Outcome::Dropped(Drop::Budget)
    );
}

#[test]
fn session_roles_separate_and_continuations_do_not_buy_a_new_walk() {
    let mut ingress = setup(3);
    assert!(ingress.new_requesting_session(&link(1), 0).unwrap());
    assert!(ingress.new_requesting_session(&link(2), 0).unwrap());
    assert!(!ingress.new_requesting_session(&link(3), 0).unwrap());
    let request = fixture("sync-request");
    for generation in 1..=3 {
        let body = unique(request.clone(), generation);
        let encoder = framing::Encoder::logical(&body, 512, 1).unwrap();
        let mut result = Outcome::Incomplete;
        for index in 0..encoder.frame_count() {
            let mut frame = [0; 512];
            let len = encoder.frame(index, &mut frame).unwrap();
            result = ingress
                .receive(
                    &link(generation),
                    len as u64,
                    &frame[..len],
                    0,
                    &mut [0; 1035],
                )
                .unwrap();
        }
        if generation <= 2 {
            assert!(matches!(result, Outcome::Complete { .. }));
        } else {
            assert_eq!(result, Outcome::Dropped(Drop::Budget));
        }
    }
    // The state above reserves admission only, never reports session completion.
    assert_eq!(ingress.counters().reserved_work_units, 0);
}

#[test]
fn crypto_bundles_atomic_concurrency_and_disconnect_cannot_free_running_work() {
    let mut ingress = setup(3);
    let a = ingress.begin_work(&link(1), 20, 0).unwrap().unwrap();
    let b = ingress.begin_work(&link(2), 20, 0).unwrap().unwrap();
    assert!(ingress.begin_work(&link(3), 1, 0).unwrap().is_none());
    ingress.disconnect(&link(1), 50).unwrap();
    assert!(ingress.begin_work(&link(3), 1, 50).unwrap().is_none());
    assert_eq!(ingress.finish_work(a), Err(Error::Stale));
    let c = ingress.begin_work(&link(3), 1, 50).unwrap().unwrap();
    ingress.finish_work(c).unwrap();
    ingress.finish_work(b).unwrap();
    assert_eq!(ingress.counters().reserved_work_units, 41);
    assert!(ingress.begin_work(&link(2), 20, 50).unwrap().is_none());
    assert_eq!(ingress.counters().reserved_work_units, 41); // No partial charge.
}

#[test]
fn reconnect_retains_node_buckets_addresses_and_retired_dedup() {
    let mut ingress = setup(4);
    let body = fixture("reaction");
    for generation in 1..=4 {
        for _ in 0..15 {
            receive(&mut ingress, generation, &body, 0);
        }
    }
    ingress.disconnect(&link(1), 0).unwrap();
    ingress.register(&link(5), 512, 0).unwrap();
    assert_eq!(
        receive(&mut ingress, 5, &body, 0),
        Outcome::Dropped(Drop::Budget)
    );
    assert_eq!(
        state(receive(&mut ingress, 5, &body, 125)),
        State::Duplicate
    );
    for id in 0..6 {
        assert!(ingress.connection_attempt([id; 16], 125).unwrap());
    }
    assert!(!ingress.connection_attempt([7; 16], 125).unwrap());
    assert!(!ingress.connection_attempt([8; 16], 10_124).unwrap());
    assert!(ingress.connection_attempt([8; 16], 10_125).unwrap());
    assert_eq!(ingress.counters().connection_attempts, 7);
}

#[test]
fn fixed_reservations_and_600_second_rotating_sender_flood() {
    for link_count in [1, 8] {
        let mut ingress = setup(link_count);
        let baseline = ingress.reservations();
        let mut raw = whole(&fixture("reaction"));
        let mut id = 0u64;
        // 100 values/second per established attacker link, for 600 seconds.
        for tick in 0..60_000u64 {
            let now = tick * 10;
            for generation in 1..=link_count {
                id += 1;
                raw[8..16].copy_from_slice(&id.to_be_bytes()); // message ID includes outer header
                raw[16..24].copy_from_slice(&id.to_be_bytes()); // rotating claimed sender
                ingress
                    .receive(
                        &link(generation),
                        raw.len() as u64,
                        &raw,
                        now,
                        &mut [0; 1035],
                    )
                    .unwrap();
            }
        }
        let c = ingress.counters();
        let r = ingress.reservations();
        assert_eq!(c.offered_frames, 60_000 * link_count);
        assert!(c.admitted_frames <= if link_count == 1 { 615 } else { 4860 });
        assert_eq!(c.reserved_work_units, 0);
        assert!(r.senders <= 2460 && r.accepted <= 4096 && r.staging == 0);
        assert_eq!(r.sender_bytes, baseline.sender_bytes);
        assert!(
            r.sender_bytes <= 1024 * 1024
                && r.accepted_bytes <= 384 * 1024
                && r.pending_bytes <= 48 * 1024
        );
        println!(
            "links={link_count} seconds=600 offered={} admitted={} sender_peak={} accepted_peak={} sender_reserved={}",
            c.offered_frames, c.admitted_frames, c.peak_senders, c.peak_accepted, r.sender_bytes
        );
    }
}

#[test]
fn monotonic_time_and_generation_checks_fail_closed() {
    let mut ingress = setup(1);
    ingress.advance(1000).unwrap();
    assert_eq!(
        ingress.enqueue(&link(1), 1, &[0], 999).unwrap_err(),
        Error::Time
    );
    assert_eq!(ingress.advance(u64::MAX), Err(Error::Time));
    ingress.disconnect(&link(1), 1000).unwrap();
    assert_eq!(ingress.register(&link(1), 512, 1000), Err(Error::Stale));
    assert_eq!(
        ingress.register(&link(2), 145, 1000),
        Err(Error::Configuration)
    );
    ingress.register(&link(2), 146, 1000).unwrap();
    assert_eq!(ingress.set_link_limit(0), Err(Error::Configuration));
    assert!(!ingress.capacity_changed(&link(2), 146, 1000).unwrap());
    assert!(ingress.capacity_changed(&link(2), 512, 1000).unwrap());
    assert_eq!(
        ingress.enqueue(&link(2), 1, &[0], 1000).unwrap_err(),
        Error::Stale
    );
    ingress.register(&link(3), 512, 1000).unwrap();
}

#[test]
fn pending_queue_caps_eviction_and_partition_reclamation_preserve_live_peers() {
    let mut ingress = setup(8);
    for generation in 1..=8 {
        for index in 0..8 {
            let id = generation * 100 + index;
            let mut body = unique(fixture("encrypted-chat-121"), id);
            body[12..20].copy_from_slice(&id.to_be_bytes());
            let _ = receive(&mut ingress, generation, &body, 0);
            assert!(ingress.reservations().pending <= 32);
        }
    }
    assert_eq!(ingress.reservations().pending, 32);
    assert_eq!(ingress.reservations().accepted, 0);
    assert!(ingress.reservations().pending_bytes <= 48 * 1024);
    let mut ingress = setup(8);
    for generation in 1..=8 {
        assert_eq!(
            state(receive(
                &mut ingress,
                generation,
                &unique(fixture("reaction"), generation),
                0
            )),
            State::Unverified
        );
    }
    ingress.disconnect(&link(8), 0).unwrap();
    ingress.register(&link(9), 512, 0).unwrap();
    assert_eq!(ingress.partition_generation(7), Some(9));
    assert_eq!(
        state(receive(&mut ingress, 9, &unique(fixture("reaction"), 1), 0)),
        State::Duplicate
    );
    assert_eq!(
        state(receive(&mut ingress, 9, &unique(fixture("reaction"), 8), 0)),
        State::Unverified
    );
}

#[test]
fn rejected_fragment_context_on_one_link_cannot_poison_another() {
    let mut ingress = Ingress::new(7, 8, 0).unwrap();
    ingress.register(&link(1), 146, 0).unwrap();
    ingress.register(&link(2), 146, 0).unwrap();
    let body = fixture("max-chat");
    let encoder = framing::Encoder::logical(&body, 146, 1).unwrap();
    let frames: Vec<_> = (0..encoder.frame_count())
        .map(|i| {
            let mut buffer = [0; 512];
            let len = encoder.frame(i, &mut buffer).unwrap();
            buffer[..len].to_vec()
        })
        .collect();
    let mut bad = frames[0].clone();
    bad[15] = 0; // Rejected group on link1.
    assert_eq!(
        ingress
            .receive(&link(1), bad.len() as u64, &bad, 0, &mut [0; 1035])
            .unwrap(),
        Outcome::Dropped(Drop::Malformed)
    );
    let _ = ingress
        .receive(
            &link(1),
            frames[0].len() as u64,
            &frames[0],
            0,
            &mut [0; 1035],
        )
        .unwrap();
    let mut outcome = Outcome::Incomplete;
    for frame in frames {
        outcome = ingress
            .receive(&link(2), frame.len() as u64, &frame, 0, &mut [0; 1035])
            .unwrap();
    }
    assert_eq!(state(outcome), State::Unverified);
}
