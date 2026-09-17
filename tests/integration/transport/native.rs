#[allow(dead_code)]
#[path = "../friends/database.rs"]
mod database;
use meshchat_core::{
    LinkHandle, SendPath, codec, identity::IdentityKeySession, native_transport::*,
    storage::EncryptedStore,
};
use std::sync::Arc;

fn owner(seed: u8) -> (NativeTransport, Arc<IdentityKeySession>) {
    let identity = IdentityKeySession::import_unlocked(vec![seed; 64], vec![seed; 16]).unwrap();
    let store = EncryptedStore::open(
        Box::new(database::Database::new()),
        vec![seed; 16],
        true,
        Some(200_000),
    )
    .unwrap();
    (
        NativeTransport::new(store, identity.public_identity().unwrap(), seed.into(), 0).unwrap(),
        identity,
    )
}
fn ready(owner: &NativeTransport, role: TransportRole, tx: u16, rx: u16, now: u64) -> LinkHandle {
    let permit = owner.admit_connection(vec![4; 16], now).unwrap();
    owner.native_ready(permit, role, tx, rx, now).unwrap().link
}
fn handshake(a: &NativeTransport, b: &NativeTransport, al: &LinkHandle, bl: &LinkHandle) {
    let af = a.tick(0).unwrap().sends.remove(0);
    let bf = b.tick(0).unwrap().sends.remove(0);
    assert_eq!(af.path, SendPath::Write);
    assert_eq!(bf.path, SendPath::Notify);
    assert_eq!(af.bytes.len(), 59);
    // HELLO may arrive before the local native completion callback.
    assert!(
        b.receive(bl.clone(), 59, af.bytes, 0)
            .unwrap()
            .events
            .is_empty()
    );
    assert!(
        a.receive(al.clone(), 59, bf.bytes, 0)
            .unwrap()
            .events
            .is_empty()
    );
    for (o, l, t) in [(a, al, af.token), (b, bl, bf.token)] {
        assert!(matches!(
            o.complete(l.clone(), t, true, 0).unwrap().events[0],
            TransportEvent::Admitted { .. }
        ));
    }
}
fn chat(id: u8, length: usize) -> Vec<u8> {
    let mut payload = 200_000_u32.to_be_bytes().to_vec();
    payload.extend_from_slice(&[0, 1, b'A', 0, 0, 0, 0]);
    payload.extend_from_slice(&(length as u16).to_be_bytes());
    payload.extend(std::iter::repeat_n(b'x', length));
    let mut raw = [0; 1024];
    let len = codec::serialize(
        codec::Header {
            kind: 1,
            flags: 0,
            ttl: 7,
            message_id: [id; 8],
            sender_id: [id; 8],
            channel_id: meshchat_core::channel::Channel::Public(
                meshchat_core::channel::Public::General,
            )
            .id(),
        },
        &payload,
        codec::Context::Live,
        &mut raw,
    )
    .unwrap();
    raw[..len].to_vec()
}

#[test]
fn real_handshake_and_bidirectional_whole_fragmented_exchange() {
    let (a, ap) = owner(1);
    let (b, bp) = owner(2);
    let al = ready(&a, TransportRole::Central, 512, 182, 0);
    let bl = ready(&b, TransportRole::Peripheral, 146, 512, 0);
    handshake(&a, &b, &al, &bl);
    a.prepare_proof(al.clone(), ap, 0).unwrap();
    b.prepare_proof(bl.clone(), bp, 0).unwrap();
    for (sender, receiver, sl, rl) in [(&a, &b, &al, &bl), (&b, &a, &bl, &al)] {
        let proof = sender.tick(1000).unwrap().sends.remove(0);
        assert_eq!(proof.bytes.len(), 71);
        assert!(
            receiver
                .receive(rl.clone(), 71, proof.bytes, 1000)
                .unwrap()
                .events
                .is_empty()
        );
        sender
            .complete(sl.clone(), proof.token, true, 1000)
            .unwrap();
    }
    let mut now = 2000;
    for (sender, receiver, sl, rl, id, length) in [
        (&a, &b, &al, &bl, 3, 5),
        (&b, &a, &bl, &al, 4, 256),
        (&a, &b, &al, &bl, 5, 256),
        (&b, &a, &bl, &al, 6, 5),
    ] {
        let raw = chat(id, length);
        sender
            .enqueue(
                sl.clone(),
                raw.clone(),
                TransportTraffic::Own,
                id.into(),
                now,
            )
            .unwrap();
        let mut delivered = Vec::new();
        let mut frames = 0;
        loop {
            let send = sender.tick(now).unwrap().sends.remove(0);
            frames += 1;
            assert!(send.bytes.len() <= 146);
            let received = receiver
                .receive(rl.clone(), send.bytes.len() as u64, send.bytes, now)
                .unwrap();
            delivered.extend(received.events);
            let effects = sender.complete(sl.clone(), send.token, true, now).unwrap();
            assert!(sender.tick(now + 500).unwrap().sends.is_empty());
            now += 1000;
            if effects.events.iter().any(|e| {
                matches!(
                    e,
                    TransportEvent::Finished {
                        status: TransportStatus::NativeComplete,
                        ..
                    }
                )
            }) {
                break;
            }
        }
        assert_eq!(frames, if length == 256 { 3 } else { 1 });
        assert_eq!(
            delivered,
            vec![TransportEvent::Received {
                link: rl.clone(),
                bytes: raw,
                intake: TransportIntake::Unverified
            }]
        );
    }
}

#[test]
fn unknown_small_and_asymmetric_capacities_cannot_start_protocol() {
    let (owner, _) = owner(3);
    for (tx, rx) in [(0, 512), (512, 0), (145, 512), (512, 145), (513, 512)] {
        let token = owner.admit_connection(vec![tx as u8; 16], 0).unwrap();
        assert_eq!(
            owner
                .native_ready(token, TransportRole::Central, tx, rx, 0)
                .unwrap_err(),
            TransportError::Invalid
        );
        assert!(owner.tick(0).unwrap().sends.is_empty());
    }
    let token = owner.admit_connection(vec![7; 16], 0).unwrap();
    let link = owner
        .native_ready(token, TransportRole::Central, 146, 512, 0)
        .unwrap()
        .link;
    assert!(
        owner
            .native_ready(token, TransportRole::Central, 146, 512, 0)
            .is_err()
    );
    assert_eq!(
        owner
            .enqueue(link.clone(), chat(3, 5), TransportTraffic::Own, 3, 0)
            .unwrap_err(),
        TransportError::Stale
    );
    let raw = chat(4, 5);
    let mut frame = vec![0, 0, 0, raw.len() as u8];
    frame.extend(raw);
    assert!(
        owner
            .receive(link, frame.len() as u64, frame, 0)
            .unwrap()
            .events
            .is_empty()
    );
}

#[test]
fn native_refusal_is_paced_once_and_stalled_callback_closes_generation() {
    let (owner, _) = owner(4);
    let link = ready(&owner, TransportRole::Central, 182, 182, 0);
    let first = owner.tick(0).unwrap().sends.remove(0);
    owner.complete(link.clone(), first.token, false, 0).unwrap();
    assert!(owner.tick(999).unwrap().sends.is_empty());
    let retry = owner.tick(1000).unwrap().sends.remove(0);
    assert_eq!(retry.bytes, first.bytes);
    assert_ne!(retry.token, first.token);
    assert_eq!(
        owner
            .complete(link.clone(), first.token, true, 1000)
            .unwrap_err(),
        TransportError::Stale
    );
    assert!(
        owner
            .complete(link.clone(), retry.token, false, 1000)
            .unwrap()
            .events
            .contains(&TransportEvent::Closed { link: link.clone() })
    );
    assert_eq!(
        owner
            .complete(link.clone(), retry.token, true, 1000)
            .unwrap_err(),
        TransportError::Stale
    );
    let next = ready(&owner, TransportRole::Central, 146, 146, 1000);
    let pending = owner.tick(1000).unwrap().sends.remove(0);
    assert!(
        owner
            .tick(6000)
            .unwrap()
            .events
            .contains(&TransportEvent::Closed { link: next.clone() })
    );
    assert_eq!(
        owner.complete(next, pending.token, true, 6000).unwrap_err(),
        TransportError::Stale
    );
}

#[test]
fn capacity_permission_disconnect_and_clock_changes_cannot_reuse_work() {
    let (owner, _) = owner(5);
    let link = ready(&owner, TransportRole::Peripheral, 512, 512, 0);
    let pending = owner.tick(0).unwrap().sends.remove(0);
    // Native capacity/permission/radio changes use this same teardown boundary.
    owner.disconnect(link.clone(), 1).unwrap();
    let next = ready(&owner, TransportRole::Peripheral, 146, 146, 1);
    assert_ne!(link, next);
    assert_eq!(
        owner.complete(link, pending.token, true, 1).unwrap_err(),
        TransportError::Stale
    );
    owner.tick(2).unwrap();
    assert_eq!(owner.tick(1).unwrap_err(), TransportError::Unavailable);
    assert_eq!(owner.tick(3).unwrap_err(), TransportError::Unavailable);
}

#[test]
fn connection_admission_and_pending_links_are_bounded_and_expire() {
    let (owner, _) = owner(6);
    let mut tokens = Vec::new();
    for address in 1..=6 {
        tokens.push(owner.admit_connection(vec![address; 16], 0).unwrap());
    }
    assert_eq!(
        owner.admit_connection(vec![7; 16], 0).unwrap_err(),
        TransportError::Busy
    );
    owner.cancel_connection(tokens[0], 0).unwrap();
    // Releasing the slot does not refund process connection credit.
    assert_eq!(
        owner.admit_connection(vec![7; 16], 0).unwrap_err(),
        TransportError::Busy
    );
    owner.tick(30_000).unwrap();
    assert_eq!(
        owner
            .native_ready(tokens[1], TransportRole::Central, 146, 146, 30_000)
            .unwrap_err(),
        TransportError::Stale
    );
    assert!(owner.admit_connection(vec![7; 16], 30_000).is_ok());
}

#[test]
fn power_hysteresis_shrinks_setup_and_ready_links_without_refunding_admission() {
    let (a, _) = owner(70);
    let mut links = Vec::new();
    for address in 0..4 {
        let token = a.admit_connection(vec![address; 16], 0).unwrap();
        links.push(
            a.native_ready(token, TransportRole::Central, 146, 146, 0)
                .unwrap()
                .link,
        );
    }
    let p1 = a.admit_connection(vec![10; 16], 0).unwrap();
    let p2 = a.admit_connection(vec![11; 16], 0).unwrap();
    let saver = a
        .update_power(Some(TransportPowerSetting::Saver), 50, false, 6, 0)
        .unwrap();
    assert!(saver.saver);
    assert_eq!(saver.link_limit, 3);
    assert_eq!(saver.cancelled_admissions, vec![p2, p1]);
    assert!(saver.effects.events.contains(&TransportEvent::Closed {
        link: links[3].clone()
    }));
    assert_eq!(
        a.native_ready(p1, TransportRole::Central, 146, 146, 0)
            .unwrap_err(),
        TransportError::Stale
    );
    for _ in 0..10 {
        a.update_power(Some(TransportPowerSetting::Normal), 50, false, 6, 0)
            .unwrap();
        assert_eq!(
            a.admit_connection(vec![99; 16], 0).unwrap_err(),
            TransportError::Busy
        );
        a.update_power(Some(TransportPowerSetting::Saver), 50, false, 6, 0)
            .unwrap();
    }
    let (b, _) = owner(71);
    assert!(!b.update_power(None, 19, false, 0, 0).unwrap().saver);
    assert!(!b.update_power(None, 19, false, 0, 59_999).unwrap().saver);
    assert!(b.update_power(None, 19, false, 0, 60_000).unwrap().saver);
    assert!(b.update_power(None, 19, true, 0, 60_001).unwrap().saver);
    assert!(b.update_power(None, 19, true, 0, 120_000).unwrap().saver);
    let normal = b.update_power(None, 19, true, 0, 120_001).unwrap();
    assert!(!normal.saver);
    assert_eq!(normal.battery_tier, 3);
    assert_eq!(
        b.update_power(None, 101, false, 0, 120_001).unwrap_err(),
        TransportError::Invalid
    );
}

#[test]
fn native_duplicate_resolution_waits_for_both_proofs_and_keeps_common_winner() {
    let (a, ap) = owner(72);
    let (b, bp) = owner(73);
    let mut pairs = Vec::new();
    for (ar, br) in [
        (TransportRole::Central, TransportRole::Peripheral),
        (TransportRole::Peripheral, TransportRole::Central),
    ] {
        let al = ready(&a, ar, 146, 146, 0);
        let bl = ready(&b, br, 146, 146, 0);
        let ah = a.tick(0).unwrap().sends.remove(0);
        let bh = b.tick(0).unwrap().sends.remove(0);
        a.receive(al.clone(), bh.bytes.len() as u64, bh.bytes, 0)
            .unwrap();
        b.receive(bl.clone(), ah.bytes.len() as u64, ah.bytes, 0)
            .unwrap();
        a.complete(al.clone(), ah.token, true, 0).unwrap();
        b.complete(bl.clone(), bh.token, true, 0).unwrap();
        pairs.push((al, bl));
    }
    assert_eq!(a.observations(0).unwrap().len(), 2); // HELLO claims cannot consolidate.
    for (al, bl) in &pairs {
        a.prepare_proof(al.clone(), ap.clone(), 0).unwrap();
        b.prepare_proof(bl.clone(), bp.clone(), 0).unwrap();
    }
    let af = a.tick(1000).unwrap().sends;
    let bf = b.tick(1000).unwrap().sends;
    for f in &af {
        let remote = &pairs.iter().find(|(al, _)| *al == f.link).unwrap().1;
        b.receive(remote.clone(), f.bytes.len() as u64, f.bytes.clone(), 1000)
            .unwrap();
    }
    for f in &bf {
        let remote = &pairs.iter().find(|(_, bl)| *bl == f.link).unwrap().0;
        a.receive(remote.clone(), f.bytes.len() as u64, f.bytes.clone(), 1000)
            .unwrap();
    }
    assert_eq!(a.observations(1000).unwrap().len(), 2); // Local proof completion is required too.
    for f in af {
        a.complete(f.link, f.token, true, 1000).unwrap();
    }
    for f in bf {
        b.complete(f.link, f.token, true, 1000).unwrap();
    }
    let live_a = a.observations(1000).unwrap();
    let live_b = b.observations(1000).unwrap();
    assert_eq!((live_a.len(), live_b.len()), (1, 1));
    assert!(
        pairs
            .iter()
            .any(|(al, bl)| *al == live_a[0].link && *bl == live_b[0].link)
    );
    assert!(a.observations(1001).unwrap()[0].last_valid_ms.is_none()); // Proof is not CHAT/ANNOUNCE.
}

fn whole(raw: &[u8]) -> Vec<u8> {
    let mut frame = vec![0, 0];
    frame.extend_from_slice(&(raw.len() as u16).to_be_bytes());
    frame.extend_from_slice(raw);
    frame
}
#[test]
fn connection_hints_require_admitted_clear_content_and_digest_expires() {
    let (a, _) = owner(74);
    let (b, _) = owner(75);
    let al = ready(&a, TransportRole::Central, 512, 512, 0);
    let bl = ready(&b, TransportRole::Peripheral, 512, 512, 0);
    handshake(&a, &b, &al, &bl);
    a.receive(al.clone(), 5000, vec![], 1).unwrap();
    assert_eq!(a.observations(1).unwrap()[0].last_valid_ms, None);
    let raw = chat(1, 4);
    let frame = whole(&raw);
    a.receive(al.clone(), frame.len() as u64, frame, 1000)
        .unwrap();
    assert_eq!(a.observations(1000).unwrap()[0].last_valid_ms, Some(1000));
    let mut payload = 200_000_u32.to_be_bytes().to_vec();
    payload.extend_from_slice(&[0, 0, 0, 1, b'A', 0, 0, 0, 0, 1, 0]);
    payload.extend_from_slice(&[0; 256]);
    let mut raw = [0; 1024];
    let len = codec::serialize(
        codec::Header {
            kind: 2,
            flags: 0,
            ttl: 1,
            message_id: [2; 8],
            sender_id: [2; 8],
            channel_id: [0; 4],
        },
        &payload,
        codec::Context::Live,
        &mut raw,
    )
    .unwrap();
    let frame = whole(&raw[..len]);
    a.receive(al.clone(), frame.len() as u64, frame, 2000)
        .unwrap();
    let observed = a.observations(2000).unwrap();
    assert_eq!(observed[0].novelty, Some(1000));
    assert_eq!(observed[0].last_valid_ms, Some(2000));
    assert_eq!(a.observations(62_000).unwrap()[0].novelty, None);
    a.disconnect(al, 62_000).unwrap();
    assert!(a.observations(62_000).unwrap().is_empty());
}

#[test]
fn public_friend_vector_requires_real_signature_before_connection_activity() {
    let (a, _) = owner(78);
    let (b, _) = owner(79);
    let al = ready(&a, TransportRole::Central, 512, 512, 0);
    let bl = ready(&b, TransportRole::Peripheral, 512, 512, 0);
    handshake(&a, &b, &al, &bl);
    // Node/OpenSSL-produced committed vector, not a transcript made by this test.
    let hex = include_str!("../../vectors/crypto/friend-v1.tsv")
        .lines()
        .find_map(|line| line.strip_prefix("peer_chat\t"))
        .unwrap();
    let raw: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect();
    let mut corrupted = raw.clone();
    *corrupted.last_mut().unwrap() ^= 1;
    let frame = whole(&corrupted);
    let rejected = a.receive(al.clone(), frame.len() as u64, frame, 0).unwrap();
    assert!(matches!(
        rejected.events[0],
        TransportEvent::Received {
            intake: TransportIntake::Pending,
            ..
        }
    ));
    assert_eq!(a.observations(0).unwrap()[0].first_valid_ms, None);
    let frame = whole(&raw);
    let valid = a
        .receive(al.clone(), frame.len() as u64, frame, 1000)
        .unwrap();
    // Activity eligibility is separate from the content owner's acceptance.
    assert!(matches!(
        valid.events[0],
        TransportEvent::Received {
            intake: TransportIntake::Pending,
            ..
        }
    ));
    assert_eq!(a.observations(1000).unwrap()[0].first_valid_ms, Some(1000));
    let frame = whole(&raw);
    a.receive(al, frame.len() as u64, frame, 2000).unwrap();
    let observed = a.observations(2000).unwrap();
    assert_eq!(observed[0].first_valid_ms, Some(1000));
    assert_eq!(observed[0].last_valid_ms, Some(2000));
}

fn ios_owner(seed: u8) -> NativeTransport {
    let identity = IdentityKeySession::import_unlocked(vec![seed; 64], vec![seed; 16]).unwrap();
    let store = EncryptedStore::open(
        Box::new(database::Database::new()),
        vec![seed; 16],
        true,
        Some(200_000),
    )
    .unwrap();
    NativeTransport::new_ios(store, identity.public_identity().unwrap(), seed.into(), 0).unwrap()
}
#[test]
fn ios_caps_readiness_and_backpressure_are_separate_from_android_completion() {
    let a = ios_owner(80);
    let (b, _) = owner(81);
    let al = ready(&a, TransportRole::Central, 182, 512, 0);
    let bl = ready(&b, TransportRole::Peripheral, 512, 512, 0);
    handshake(&a, &b, &al, &bl);
    assert_eq!(
        a.update_power(Some(TransportPowerSetting::Normal), 100, true, 0, 0)
            .unwrap()
            .link_limit,
        4
    );
    a.enqueue(al.clone(), chat(90, 5), TransportTraffic::Own, 90, 1000)
        .unwrap();
    a.set_writable(al.clone(), false, 1000).unwrap();
    assert!(a.tick(5000).unwrap().sends.is_empty());
    a.readiness(al.clone(), 5000).unwrap();
    let first = a.tick(5000).unwrap().sends.remove(0);
    a.backpressure(al.clone(), first.token, 5000).unwrap();
    assert_eq!(
        a.complete(al.clone(), first.token, true, 5000).unwrap_err(),
        TransportError::Stale
    );
    assert!(a.tick(10_000).unwrap().events.is_empty()); // no Android five-second timeout
    assert!(a.tick(10_000).unwrap().sends.is_empty());
    a.readiness(al.clone(), 10_000).unwrap();
    let second = a.tick(10_000).unwrap().sends.remove(0);
    assert_eq!(second.bytes, first.bytes);
    a.backpressure(al.clone(), second.token, 10_000).unwrap();
    a.readiness(al.clone(), 11_000).unwrap();
    let third = a.tick(11_000).unwrap().sends.remove(0);
    assert_eq!(third.bytes, first.bytes); // multiple queue-fulls are not native failures
    assert!(
        a.complete(al.clone(), third.token, true, 11_000)
            .unwrap()
            .events
            .iter()
            .any(|e| matches!(
                e,
                TransportEvent::Finished {
                    cookie: 90,
                    status: TransportStatus::NativeComplete,
                    ..
                }
            ))
    );
    assert!(a.tick(25_999).unwrap().events.is_empty());
    assert!(
        a.tick(26_000)
            .unwrap()
            .events
            .contains(&TransportEvent::Closed { link: al.clone() })
    );
    assert_eq!(a.readiness(al, 26_000).unwrap_err(), TransportError::Stale);
}
#[test]
fn ios_capacity_includes_setup_and_power_shrink_preserves_admission() {
    let a = ios_owner(82);
    let permits: Vec<_> = (0..4)
        .map(|i| a.admit_connection(vec![i; 16], 0).unwrap())
        .collect();
    assert_eq!(
        a.admit_connection(vec![9; 16], 0).unwrap_err(),
        TransportError::Busy
    );
    let p = a
        .update_power(Some(TransportPowerSetting::Saver), 10, false, 4, 0)
        .unwrap();
    assert_eq!(p.link_limit, 3);
    assert_eq!(p.cancelled_admissions, vec![permits[3]]);
    a.update_power(Some(TransportPowerSetting::Normal), 100, true, 0, 0)
        .unwrap();
    a.admit_connection(vec![10; 16], 0).unwrap();
    assert_eq!(
        a.admit_connection(vec![11; 16], 0).unwrap_err(),
        TransportError::Busy
    );
}
#[test]
fn ios_property_polling_does_not_refresh_liveness_but_inbound_does() {
    let a = ios_owner(83);
    let (b, _) = owner(84);
    let al = ready(&a, TransportRole::Peripheral, 146, 512, 0);
    let bl = ready(&b, TransportRole::Central, 512, 512, 0);
    // Roles need no alternate wire format; exchange explicitly in these roles.
    let af = a.tick(0).unwrap().sends.remove(0);
    let bf = b.tick(0).unwrap().sends.remove(0);
    a.receive(al.clone(), bf.bytes.len() as u64, bf.bytes, 0)
        .unwrap();
    b.receive(bl.clone(), af.bytes.len() as u64, af.bytes, 0)
        .unwrap();
    a.complete(al.clone(), af.token, true, 0).unwrap();
    b.complete(bl, bf.token, true, 0).unwrap();
    let raw = chat(85, 5);
    let mut frame = vec![0, 0, 0, raw.len() as u8];
    frame.extend(raw);
    a.receive(al.clone(), frame.len() as u64, frame, 14_000)
        .unwrap();
    a.set_writable(al.clone(), true, 28_999).unwrap();
    assert!(a.tick(28_999).unwrap().events.is_empty());
    assert!(
        a.tick(29_000)
            .unwrap()
            .events
            .contains(&TransportEvent::Closed { link: al })
    );
}

#[test]
fn native_sync_wrappers_remain_deferred_and_use_the_paced_bounded_scheduler() {
    let a = ios_owner(86);
    let (b, _) = owner(87);
    let al = ready(&a, TransportRole::Central, 182, 512, 0);
    let bl = ready(&b, TransportRole::Peripheral, 512, 512, 0);
    handshake(&a, &b, &al, &bl);
    assert!(a.enqueue_sync(al.clone(), vec![0; 10], 3, 0).is_err());
    let raw = chat(88, 256);
    let mut body = vec![0, 1, 0, 0, 0, 0, 0, 0, 0];
    body.extend_from_slice(&(raw.len() as u16).to_be_bytes());
    body.extend(raw);
    a.enqueue_sync(al.clone(), body.clone(), 88, 1000).unwrap();
    let mut received = Vec::new();
    for now in [1000, 2000, 3000] {
        let frame = a.tick(now).unwrap().sends.remove(0);
        received.extend(
            b.receive(bl.clone(), frame.bytes.len() as u64, frame.bytes, now)
                .unwrap()
                .events,
        );
        a.complete(al.clone(), frame.token, true, now).unwrap();
    }
    assert_eq!(
        received,
        vec![TransportEvent::Received {
            link: bl,
            bytes: body,
            intake: TransportIntake::DeferredSync
        }]
    );
}
