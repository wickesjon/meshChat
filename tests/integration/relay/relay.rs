use meshchat_core::{
    LinkHandle,
    framing::{Encoder, Reassembler},
    power::*,
    relay::*,
};
fn link(generation: u64) -> LinkHandle {
    LinkHandle {
        instance_nonce: 7,
        generation,
    }
}
fn fixture(name: &str) -> Vec<u8> {
    let l = include_str!("../../vectors/base/logical.txt")
        .lines()
        .find(|l| l.split_whitespace().next() == Some(name))
        .unwrap();
    l.split_whitespace()
        .nth(3)
        .unwrap()
        .as_bytes()
        .chunks_exact(2)
        .map(|x| u8::from_str_radix(std::str::from_utf8(x).unwrap(), 16).unwrap())
        .collect()
}
fn body(name: &str, id: u64) -> Vec<u8> {
    let mut b = fixture(name);
    b[4..12].copy_from_slice(&id.to_be_bytes());
    b
}
fn setup(n: u64, c: usize, suppression: bool) -> Relay {
    let mut r = Relay::new(7, Platform::Android, suppression, 0).unwrap();
    if n > 6 {
        r.set_power(Mode::Beacon, 3, 0).unwrap();
    }
    for i in 1..=n {
        r.register(&link(i), c, 0).unwrap();
    }
    r
}
fn enqueue(
    r: &mut Relay,
    l: u64,
    b: &[u8],
    traffic: Traffic,
    id: u64,
    now: u64,
    events: &mut Vec<ResultEvent>,
) -> Result<Option<u64>, Error> {
    r.enqueue(
        &link(l),
        b,
        Request {
            traffic,
            cookie: id,
            random: 0,
        },
        now,
        &mut |e| events.push(e),
    )
}
fn send(r: &mut Relay, now: u64, events: &mut Vec<ResultEvent>) -> Option<(Send, Vec<u8>)> {
    let mut out = [0; 512];
    r.poll(now, &mut out, &mut |e| events.push(e))
        .unwrap()
        .map(|s| {
            let b = out[..s.len].to_vec();
            (s, b)
        })
}
fn done(
    r: &mut Relay,
    s: &Send,
    success: bool,
    now: u64,
    events: &mut Vec<ResultEvent>,
) -> Result<(), Error> {
    r.complete(s.attempt, success, now, &mut |e| events.push(e))
}
#[test]
fn ttl_coverage_is_per_peer_and_full_variants_survive() {
    let mut r = setup(3, 512, true);
    let mut events = vec![];
    let b = body("max-chat", 1);
    assert!(r.observe(Some(&link(1)), &b, 0).unwrap());
    for l in 1..=3 {
        enqueue(&mut r, l, &b, Traffic::Forwarded, l, 0, &mut events).unwrap();
    }
    assert!(!r.observe(Some(&link(2)), &b, 20).unwrap());
    let (s, raw) = send(&mut r, 80, &mut events).unwrap();
    assert_eq!(s.link, link(3));
    assert_eq!(raw[7], 6);
    assert_eq!(
        events
            .iter()
            .filter(|e| e.status == Status::Suppressed)
            .count(),
        2
    );
    done(&mut r, &s, true, 100, &mut events).unwrap();
    let mut variant = b.clone();
    *variant.last_mut().unwrap() = b'y';
    assert!(r.observe(Some(&link(1)), &variant, 101).unwrap());
    enqueue(&mut r, 2, &variant, Traffic::Forwarded, 4, 101, &mut events).unwrap();
    assert!(send(&mut r, 181, &mut events).is_some());
    let mut exhausted = b.clone();
    exhausted[3] = 1;
    assert_eq!(
        enqueue(
            &mut r,
            3,
            &exhausted,
            Traffic::Forwarded,
            5,
            181,
            &mut events
        )
        .unwrap(),
        None
    );
    exhausted[3] = 0;
    assert_eq!(
        enqueue(
            &mut r,
            3,
            &exhausted,
            Traffic::Forwarded,
            6,
            181,
            &mut events
        ),
        Err(Error::Invalid)
    );
    let mut over = b;
    over[3] = 255;
    enqueue(&mut r, 3, &over, Traffic::Forwarded, 7, 181, &mut events).unwrap();
    let (_, raw) = send(&mut r, 1080, &mut events).unwrap();
    assert_eq!(raw[7], 6);
}
#[test]
fn fragments_are_contiguous_paced_and_retry_once_with_stale_completion_rejected() {
    let mut r = setup(1, 146, false);
    let mut events = vec![];
    enqueue(
        &mut r,
        1,
        &body("max-chat", 1),
        Traffic::Own,
        1,
        0,
        &mut events,
    )
    .unwrap();
    let (s, raw) = send(&mut r, 0, &mut events).unwrap();
    assert_eq!(s.frames, 3);
    assert_eq!(s.fragment, 0);
    done(&mut r, &s, false, 20, &mut events).unwrap();
    assert!(send(&mut r, 999, &mut events).is_none());
    let (retry, retried) = send(&mut r, 1000, &mut events).unwrap();
    assert_eq!(raw, retried);
    assert!(retry.retry);
    assert_eq!(done(&mut r, &s, true, 1000, &mut events), Err(Error::Stale));
    done(&mut r, &retry, true, 1020, &mut events).unwrap();
    enqueue(
        &mut r,
        1,
        &body("reaction", 2),
        Traffic::Own,
        2,
        1020,
        &mut events,
    )
    .unwrap();
    for t in [2000, 3000] {
        let (s, _) = send(&mut r, t, &mut events).unwrap();
        assert_eq!(s.cookie, 1);
        assert_eq!(s.fragment, (t / 1000 - 1) as u8);
        done(&mut r, &s, true, t + 20, &mut events).unwrap();
    }
    let (s, _) = send(&mut r, 4000, &mut events).unwrap();
    assert_eq!(s.cookie, 2);
    done(&mut r, &s, false, 4020, &mut events).unwrap();
    let (s, _) = send(&mut r, 5000, &mut events).unwrap();
    done(&mut r, &s, false, 5020, &mut events).unwrap();
    assert!(
        events
            .iter()
            .any(|e| e.cookie == 2 && e.status == Status::NativeFailed)
    );
    assert_eq!(r.counters().attempts, 6);
    assert_eq!(r.counters().retries, 2);
}
#[test]
fn all_capacities_use_actual_encoder_and_reassembler() {
    for c in [146, 182, 512] {
        let mut r = setup(1, c, false);
        let mut rx = Reassembler::new(7, 1, 0).unwrap();
        rx.register(&link(1), c).unwrap();
        let b = body("max-chat", 1);
        let expected = Encoder::logical(&b, c, 1).unwrap().frame_count();
        let mut events = vec![];
        enqueue(&mut r, 1, &b, Traffic::Own, 1, 0, &mut events).unwrap();
        let mut out = [0; 1035];
        for i in 0..expected {
            let t = i as u64 * 1000;
            let (s, raw) = send(&mut r, t, &mut events).unwrap();
            let result = rx.ingest_admitted(&link(1), &raw, t, &mut out).unwrap();
            if i + 1 == expected {
                assert_eq!(result.unwrap().len, b.len());
                assert_eq!(&out[..b.len()], b);
            } else {
                assert!(result.is_none());
            }
            done(&mut r, &s, true, t + 20, &mut events).unwrap();
        }
    }
}
#[test]
fn bounded_own_queue_visible_refusal_eviction_order_and_reservations() {
    let mut r = setup(6, 512, false);
    let mut events = vec![];
    for i in 0..128 {
        enqueue(
            &mut r,
            i / 32 + 1,
            &body("reaction", i + 1),
            Traffic::Own,
            i,
            0,
            &mut events,
        )
        .unwrap();
    }
    assert_eq!(
        enqueue(
            &mut r,
            5,
            &body("reaction", 200),
            Traffic::Own,
            200,
            0,
            &mut events
        ),
        Err(Error::Full)
    );
    let res = r.reservations();
    assert_eq!(res.objects, 128);
    assert!(res.outbound_bytes <= 192 * 1024);
    assert!(res.outbound_bytes_per_link <= 48 * 1024);
    assert!(res.recent_bytes <= 16 * 1024);
    assert_eq!(r.counters().peak_link_objects, 32);
    r.disconnect(&link(1), 1, &mut |e| events.push(e)).unwrap();
    assert_eq!(
        events
            .iter()
            .filter(|e| e.status == Status::Disconnected)
            .count(),
        32
    );
    r.register(&link(7), 512, 1).unwrap();
    for i in 0..32 {
        enqueue(
            &mut r,
            7,
            &body(if i == 10 { "reaction" } else { "max-chat" }, 1000 + i),
            Traffic::Forwarded,
            1000 + i,
            1,
            &mut events,
        )
        .unwrap();
    }
    enqueue(
        &mut r,
        7,
        &body("max-chat", 2000),
        Traffic::Own,
        2000,
        1,
        &mut events,
    )
    .unwrap();
    assert!(
        events
            .iter()
            .any(|e| e.cookie == 1010 && e.status == Status::Evicted)
    );
    enqueue(
        &mut r,
        7,
        &body("max-chat", 2001),
        Traffic::Own,
        2001,
        1,
        &mut events,
    )
    .unwrap();
    assert!(
        events
            .iter()
            .any(|e| e.cookie == 1000 && e.status == Status::Evicted)
    );
    assert_eq!(r.reservations().objects, 128);
}
#[test]
fn backpressure_absolute_deadlines_and_disconnect_do_not_refresh_work() {
    let mut r = setup(1, 146, false);
    let mut events = vec![];
    enqueue(
        &mut r,
        1,
        &body("max-chat", 1),
        Traffic::Own,
        1,
        0,
        &mut events,
    )
    .unwrap();
    r.ready(&link(1), false, 0).unwrap();
    assert!(send(&mut r, 29_999, &mut events).is_none());
    r.ready(&link(1), true, 30_000).unwrap();
    assert!(send(&mut r, 30_000, &mut events).is_none());
    assert_eq!(events[0].status, Status::Expired);
    enqueue(
        &mut r,
        1,
        &body("max-chat", 2),
        Traffic::Own,
        2,
        30_000,
        &mut events,
    )
    .unwrap();
    let (s, _) = send(&mut r, 30_000, &mut events).unwrap();
    assert_eq!(
        done(&mut r, &s, true, 60_000, &mut events),
        Err(Error::Stale)
    );
    assert_eq!(events.last().unwrap().status, Status::Expired);
    enqueue(
        &mut r,
        1,
        &body("max-chat", 3),
        Traffic::Own,
        3,
        60_000,
        &mut events,
    )
    .unwrap();
    let (s, _) = send(&mut r, 60_000, &mut events).unwrap();
    r.disconnect(&link(1), 60_010, &mut |e| events.push(e))
        .unwrap();
    r.register(&link(2), 182, 60_010).unwrap();
    assert_eq!(
        done(&mut r, &s, true, 60_020, &mut events),
        Err(Error::Stale)
    );
    assert_eq!(r.register(&link(1), 512, 60_020), Err(Error::Stale));
    assert_eq!(r.ready(&link(2), true, 60_019), Err(Error::Time));
}
#[test]
fn fair_classes_sync_announce_alternation_and_coalescing() {
    let mut r = setup(1, 512, false);
    let mut events = vec![];
    let announce = body("announce", 1); // fixture name checked below
    let marker = [0, 1, 0, 0, 5, 0, 0, 0, 0, 0, 0];
    for i in 0..4 {
        enqueue(
            &mut r,
            1,
            &body("reaction", 10 + i),
            Traffic::Own,
            10 + i,
            0,
            &mut events,
        )
        .unwrap();
        enqueue(
            &mut r,
            1,
            &body("reaction", 20 + i),
            Traffic::Forwarded,
            20 + i,
            0,
            &mut events,
        )
        .unwrap();
        enqueue(
            &mut r,
            1,
            &body("max-chat", 30 + i),
            Traffic::Forwarded,
            30 + i,
            0,
            &mut events,
        )
        .unwrap();
        enqueue(
            &mut r,
            1,
            &marker,
            Traffic::Transport(1),
            40 + i,
            0,
            &mut events,
        )
        .unwrap();
    }
    enqueue(&mut r, 1, &announce, Traffic::Local, 50, 0, &mut events).unwrap();
    enqueue(
        &mut r,
        1,
        &body("announce", 2),
        Traffic::Local,
        51,
        0,
        &mut events,
    )
    .unwrap();
    assert!(
        events
            .iter()
            .any(|e| e.cookie == 50 && e.status == Status::Coalesced)
    );
    let mut hello = vec![0; 54];
    hello[0] = 1;
    hello[2..4].copy_from_slice(&512u16.to_be_bytes());
    hello[4..6].copy_from_slice(&512u16.to_be_bytes());
    hello[6] = 1;
    enqueue(&mut r, 1, &hello, Traffic::Transport(2), 60, 0, &mut events).unwrap();
    let mut order = vec![];
    for t in 0..18 {
        let (s, _) = send(&mut r, t * 1000, &mut events).unwrap();
        order.push(s.kind);
        done(&mut r, &s, true, t * 1000 + 20, &mut events).unwrap();
    }
    assert!(order.contains(&Kind::Control));
    assert!(order.contains(&Kind::Own));
    assert!(order.contains(&Kind::Chat));
    assert!(order.contains(&Kind::Reaction));
    let fifth: Vec<_> = order
        .into_iter()
        .filter(|k| matches!(k, Kind::Sync | Kind::Announce))
        .collect();
    assert_eq!(&fifth[..2], &[Kind::Sync, Kind::Announce]);
}
#[test]
fn recent_digest_is_bounded_expires_and_false_positive_is_only_relay_hint() {
    let mut r = setup(2, 512, true);
    let mut events = vec![];
    let b = body("max-chat", 1);
    for i in 0..250 {
        r.observe(None, &body("max-chat", i + 10), 0).unwrap();
    }
    assert_eq!(r.counters().peak_recent, 200);
    assert_ne!(r.digest(0).unwrap(), [0; 256]);
    assert_eq!(r.digest(60_000).unwrap(), [0; 256]);
    let mut announcement = body("announce", 300);
    let n = announcement.len();
    announcement[n - 2..].copy_from_slice(&256u16.to_be_bytes());
    announcement.extend_from_slice(&[255; 256]);
    let len = (announcement.len() - 26) as u16;
    announcement[24..26].copy_from_slice(&len.to_be_bytes());
    r.observe(Some(&link(2)), &announcement, 60_000).unwrap();
    enqueue(&mut r, 2, &b, Traffic::Forwarded, 1, 60_000, &mut events).unwrap();
    assert!(send(&mut r, 60_080, &mut events).is_none());
    assert_eq!(events.last().unwrap().status, Status::Suppressed);
    enqueue(&mut r, 2, &b, Traffic::Forwarded, 2, 120_000, &mut events).unwrap();
    assert!(send(&mut r, 120_080, &mut events).is_some()); // filter expiry removes advisory omission
}
#[test]
fn node_buckets_survive_reconnect_and_saver_transition_no_extra_credit() {
    let mut r = setup(8, 512, false);
    let mut events = vec![];
    // Reconnect each full batch at one monotonic instant; link bursts cannot refill node.
    let mut generations: Vec<u64> = (1..=8).collect();
    let mut admitted = 0;
    for batch in 0..10 {
        for (i, g) in generations.iter_mut().enumerate() {
            enqueue(
                &mut r,
                *g,
                &body("reaction", 100 + batch * 8 + i as u64),
                Traffic::Own,
                0,
                0,
                &mut events,
            )
            .unwrap();
            if let Some((s, _)) = send(&mut r, 0, &mut events) {
                admitted += 1;
                done(&mut r, &s, true, 0, &mut events).unwrap();
            }
            r.disconnect(&link(*g), 0, &mut |e| events.push(e)).unwrap();
            *g = 9 + batch * 8 + i as u64;
            r.register(&link(*g), 512, 0).unwrap();
        }
    }
    assert_eq!(admitted, 60);
    assert_eq!(r.counters().attempts, 60);
    for g in &generations[3..] {
        r.disconnect(&link(*g), 0, &mut |e| events.push(e)).unwrap();
    }
    r.set_power(Mode::Saver, 0, 0).unwrap();
    r.set_power(Mode::Normal, 1, 0).unwrap();
    enqueue(
        &mut r,
        generations[0],
        &body("reaction", 999),
        Traffic::Own,
        0,
        0,
        &mut events,
    )
    .unwrap();
    assert!(send(&mut r, 0, &mut events).is_none());
    assert!(send(&mut r, 125, &mut events).is_some());
}
#[test]
fn forwarding_budget_and_six_hundred_second_flood_stay_bounded() {
    let mut r = setup(3, 512, false);
    r.set_power(Mode::Saver, 0, 0).unwrap();
    let mut events = vec![];
    let mut successes = 0;
    let mut last = [None; 3];
    for now in (0..600_000).step_by(100) {
        for l in 1..=3 {
            let _ = enqueue(
                &mut r,
                l,
                &body("max-chat", now * 8 + l),
                Traffic::Forwarded,
                now,
                now,
                &mut events,
            );
        }
        while let Some((s, _)) = send(&mut r, now, &mut events) {
            let i = s.link.generation as usize - 1;
            if let Some(t) = last[i] {
                assert!(now - t >= 1000);
            }
            last[i] = Some(now);
            successes += 1;
            done(&mut r, &s, true, now, &mut events).unwrap();
        }
        assert!(r.reservations().objects <= 96);
        events.clear();
    }
    assert!(successes <= 440);
    assert!(successes >= 430);
    assert_eq!(r.counters().forwarded_by_mode[1], successes);
    assert!(r.counters().peak_link_objects <= 32);
    assert!(r.counters().peak_node_objects <= 96);
}
#[test]
fn stable_round_robin_rotates_eligible_links() {
    let mut r = setup(3, 512, false);
    let mut events = vec![];
    for l in 1..=3 {
        for id in 0..3 {
            enqueue(
                &mut r,
                l,
                &body("reaction", l * 10 + id),
                Traffic::Own,
                id,
                0,
                &mut events,
            )
            .unwrap();
        }
    }
    for t in 0..3 {
        let mut order = vec![];
        for _ in 0..3 {
            let (s, _) = send(&mut r, t * 1000, &mut events).unwrap();
            order.push(s.link.generation);
            done(&mut r, &s, true, t * 1000, &mut events).unwrap();
        }
        assert_eq!(order, vec![1, 2, 3]);
        assert!(send(&mut r, t * 1000, &mut events).is_none());
    }
}
#[test]
fn auto_hysteresis_manual_modes_beacon_guard_and_platform_limits() {
    let mut p = Policy::new(Platform::Android, 0);
    let mut input = Inputs {
        battery_percent: 30,
        charging: false,
        visible_peers: 4,
        auto_beacon: false,
    };
    assert_eq!(p.update(input, 0).unwrap().mode, Mode::Normal);
    assert_eq!(p.update(input, 59_999).unwrap().mode, Mode::Normal);
    assert_eq!(p.update(input, 60_000).unwrap().mode, Mode::Saver);
    input.visible_peers = 3;
    assert_eq!(p.update(input, 60_001).unwrap().mode, Mode::Saver);
    input.visible_peers = 4;
    p.update(input, 70_000).unwrap();
    input.visible_peers = 3;
    p.update(input, 80_000).unwrap();
    assert_eq!(p.update(input, 139_999).unwrap().mode, Mode::Saver);
    assert_eq!(p.update(input, 140_000).unwrap().mode, Mode::Normal);
    let saver = p.set(Setting::Saver, input, 140_001).unwrap();
    assert_eq!(
        (
            saver.max_links,
            saver.announce_ms,
            saver.scan_on_ms,
            saver.scan_off_ms
        ),
        (3, 60_000, 10_000, 50_000)
    );
    input.battery_percent = 31;
    let b = p.set(Setting::Beacon, input, 140_002).unwrap();
    assert_eq!(b.max_links, 8);
    assert!(!b.infra);
    input.battery_percent = 30;
    let b = p.update(input, 140_003).unwrap();
    assert_eq!(b.mode, Mode::Normal);
    assert!(b.beacon_battery_exit);
    let mut ios = Policy::new(Platform::Ios, 0);
    assert!(ios.set(Setting::Beacon, input, 0).is_err());
    input.charging = true;
    input.auto_beacon = true;
    assert_eq!(ios.update(input, 0).unwrap().max_links, 4);
    assert_eq!(ios.update(input, 60_000).unwrap().mode, Mode::Normal);
    let mut r = setup(4, 512, false);
    assert_eq!(r.set_power(Mode::Saver, 0, 0), Err(Error::Links));
    r.disconnect(&link(4), 0, &mut |_| {}).unwrap();
    r.set_power(Mode::Saver, 0, 0).unwrap();
    assert_eq!(r.register(&link(5), 512, 0), Err(Error::Links));
}
#[test]
fn thirty_reaction_variants_share_target_reservations_across_all_egresses() {
    let mut r = setup(2, 512, false);
    let mut events = vec![];
    for id in 1..=30 {
        let b = body("reaction", id);
        r.observe(Some(&link(1)), &b, 0).unwrap();
        for l in 1..=2 {
            enqueue(&mut r, l, &b, Traffic::Forwarded, id, 0, &mut events).unwrap();
        }
    }
    assert_eq!(r.reservations().objects, 60);
    assert!(r.reservations().target_bytes <= 64 * 1024);
    assert_eq!(
        enqueue(
            &mut r,
            2,
            &body("reaction", 31),
            Traffic::Forwarded,
            31,
            0,
            &mut events
        ),
        Err(Error::Full)
    );
    // A different target has an independent allowance.
    let mut another = body("reaction", 32);
    another[26..34].copy_from_slice(&99u64.to_be_bytes());
    enqueue(&mut r, 2, &another, Traffic::Forwarded, 32, 0, &mut events).unwrap();
    // No refund on disconnect/eviction and no per-egress multiplication.
    r.disconnect(&link(1), 1, &mut |e| events.push(e)).unwrap();
    r.register(&link(3), 512, 1).unwrap();
    assert_eq!(
        enqueue(
            &mut r,
            3,
            &body("reaction", 31),
            Traffic::Forwarded,
            31,
            1,
            &mut events
        ),
        Err(Error::Full)
    );
    r.advance(900_000, &mut |e| events.push(e)).unwrap();
    enqueue(
        &mut r,
        3,
        &body("reaction", 31),
        Traffic::Forwarded,
        31,
        900_000,
        &mut events,
    )
    .unwrap();
}
#[test]
fn deficit_quanta_weight_continuously_ready_small_objects() {
    let mut r = setup(1, 512, false);
    let mut events = vec![];
    for id in 0..16 {
        enqueue(
            &mut r,
            1,
            &body("reaction", id + 100),
            Traffic::Own,
            id,
            0,
            &mut events,
        )
        .unwrap();
    }
    for id in 0..8 {
        enqueue(
            &mut r,
            1,
            &body("reaction", id + 200),
            Traffic::Forwarded,
            id + 100,
            0,
            &mut events,
        )
        .unwrap();
    }
    let mut order = vec![];
    for t in 0..20 {
        let (s, _) = send(&mut r, t * 1000, &mut events).unwrap();
        order.push(s.kind);
        done(&mut r, &s, true, t * 1000, &mut events).unwrap();
    }
    assert_eq!(&order[..16], &[Kind::Own; 16]);
    assert_eq!(&order[16..], &[Kind::Reaction; 4]);
}

#[test]
fn infra_is_only_charging_android_beacon_not_an_ordinary_power_hint() {
    for platform in [Platform::Android, Platform::Ios] {
        for mode in [Mode::Normal, Mode::Saver, Mode::Beacon] {
            assert!(!parameters(platform, mode, false, false).infra);
            assert_eq!(
                parameters(platform, mode, true, false).infra,
                platform == Platform::Android && mode == Mode::Beacon
            );
        }
    }
    let mut policy = Policy::new(Platform::Android, 0);
    let input = Inputs {
        battery_percent: 80,
        charging: true,
        visible_peers: 4,
        auto_beacon: true,
    };
    assert!(!policy.update(input, 0).unwrap().infra);
    assert!(policy.update(input, 60_000).unwrap().infra);
    assert!(
        !policy
            .update(
                Inputs {
                    charging: false,
                    ..input
                },
                60_001
            )
            .unwrap()
            .infra
    );
}

#[test]
fn ios_queue_full_retains_deadline_and_charges_every_attempt() {
    let mut r = Relay::new(7, Platform::Ios, false, 0).unwrap();
    r.register(&link(1), 146, 0).unwrap();
    let mut events = Vec::new();
    enqueue(
        &mut r,
        1,
        &body("chat", 100),
        Traffic::Own,
        100,
        0,
        &mut events,
    )
    .unwrap();
    let first = send(&mut r, 0, &mut events).unwrap();
    r.backpressure(first.0.attempt, 0, &mut |e| events.push(e))
        .unwrap();
    assert!(send(&mut r, 1000, &mut events).is_none());
    for now in [1000, 2000, 29_000] {
        r.ready(&link(1), true, now).unwrap();
        let next = send(&mut r, now, &mut events).unwrap();
        assert_eq!(next.1, first.1);
        r.backpressure(next.0.attempt, now, &mut |e| events.push(e))
            .unwrap();
    }
    assert_eq!(r.counters().attempts, 4);
    assert!(events.is_empty());
    r.advance(30_000, &mut |e| events.push(e)).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].status, Status::Expired);
    r.ready(&link(1), true, 30_000).unwrap();
    assert!(send(&mut r, 30_000, &mut events).is_none());
    assert!(
        r.backpressure(first.0.attempt, 30_000, &mut |_| {})
            .is_err()
    );
}
