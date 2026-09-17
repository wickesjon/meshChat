#[allow(dead_code)]
#[path = "../../integration/friends/database.rs"]
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
fn six_logical_hours_preserve_bounded_cache_and_scheduler() {
    let (a, _) = owner(1);
    let (b, _) = owner(2);
    a.configure_beacon(true, false).unwrap();
    assert!(a.update_power(None, 90, true, 0, 0).unwrap().infra);
    let al = ready(&a, TransportRole::Central, 512, 512, 0);
    let bl = ready(&b, TransportRole::Peripheral, 512, 512, 0);
    handshake(&a, &b, &al, &bl);
    let (c, _) = owner(6);
    let ac = ready(&a, TransportRole::Central, 512, 512, 0);
    let cl = ready(&c, TransportRole::Peripheral, 512, 512, 0);
    handshake(&a, &c, &ac, &cl);
    let mut peak = 0;
    for second in (5u64..=21_600).step_by(5) {
        let now = second * 1000;
        let mut bytes = chat(8, 16);
        bytes[4..12].copy_from_slice(&second.to_be_bytes());
        bytes[12..20].copy_from_slice(&(second % 3).to_be_bytes());
        let mut frame = vec![0, 0];
        frame.extend((bytes.len() as u16).to_be_bytes());
        frame.extend(&bytes);
        let effects = a
            .receive(al.clone(), frame.len() as u64, frame, now)
            .unwrap();
        assert!(effects.events.iter().any(|e| matches!(
            e,
            TransportEvent::Received {
                intake: TransportIntake::Unverified,
                ..
            }
        )));
        a.enqueue(ac.clone(), bytes, TransportTraffic::Forwarded, 3, now)
            .unwrap();
        for send in a.tick(now + 50).unwrap().sends {
            a.complete(send.link, send.token, true, now + 50).unwrap();
        }
        let status = a.beacon_status(now + 50).unwrap();
        peak = peak.max(status.cached_messages);
        assert!(status.cached_messages <= 720);
        assert!(
            status.cache_bytes <= 5 * 1024 * 1024
                && status.allocated_cache_bytes <= 6 * 1024 * 1024
        );
        assert!(status.queued_objects <= 256);
        assert!(status.active && status.infra);
    }
    let status = a.beacon_status(21_600_050).unwrap();
    assert_eq!(peak, 720);
    assert_eq!(status.cached_messages, 720);
    assert!(status.relay_frame_attempts > 4_000);
    let power = a.update_power(None, 31, false, 1, 21_600_051).unwrap();
    assert!(power.beacon && !power.infra);
    let power = a.update_power(None, 30, false, 1, 21_600_052).unwrap();
    assert!(!power.beacon && power.beacon_battery_exit);
    let status = a.beacon_status(21_600_052).unwrap();
    assert!(status.cached_messages <= 180 && status.cached_messages > 0);
    assert!(status.allocated_cache_bytes <= 320 * 1024);
    assert_eq!(a.observations(21_600_052).unwrap()[0].link, al);
}

#[test]
fn auto_power_edges_manual_latch_and_runtime_eight_link_ceiling() {
    let (a, _) = owner(3);
    a.configure_beacon(false, true).unwrap();
    assert!(a.update_power(None, 20, true, 0, 0).unwrap().beacon);
    let tokens: Vec<_> = (0..8)
        .map(|n| {
            a.admit_connection(vec![n; 16], if n < 6 { 0 } else { (n as u64 - 5) * 10_000 })
                .unwrap()
        })
        .collect();
    assert_eq!(
        a.admit_connection(vec![9; 16], 20_000).unwrap_err(),
        TransportError::Busy
    );
    let power = a.update_power(None, 60, false, 0, 20001).unwrap();
    assert!(!power.beacon && !power.infra);
    assert_eq!(power.cancelled_admissions.len(), 2);
    assert_eq!(power.link_limit, 6);
    assert!(a.update_power(None, 20, true, 0, 20002).unwrap().beacon);
    assert_eq!(
        a.update_power(None, 20, false, 0, 20003)
            .unwrap()
            .link_limit,
        6
    );
    assert!(a.update_power(None, 20, true, 0, 20004).unwrap().beacon);
    a.configure_beacon(true, false).unwrap();
    let power = a.update_power(None, 30, false, 0, 20005).unwrap();
    assert!(power.beacon_battery_exit && !power.beacon);
    assert!(!a.update_power(None, 80, true, 0, 20006).unwrap().beacon);
    a.configure_beacon(true, false).unwrap();
    assert!(a.update_power(None, 80, true, 0, 20007).unwrap().beacon);
    for token in tokens {
        let _ = a.cancel_connection(token, 20008);
    }
}

#[test]
fn bounded_cache_is_served_only_after_admitted_request_and_respects_ttl() {
    let (a, _) = owner(4);
    let (b, _) = owner(5);
    a.configure_beacon(true, false).unwrap();
    a.update_power(None, 100, true, 0, 0).unwrap();
    let al = ready(&a, TransportRole::Central, 512, 512, 0);
    let bl = ready(&b, TransportRole::Peripheral, 512, 512, 0);
    handshake(&a, &b, &al, &bl);
    for id in 10..22 {
        let raw = chat(id, 10);
        let mut f = vec![0, 0];
        f.extend((raw.len() as u16).to_be_bytes());
        f.extend(raw);
        a.receive(al.clone(), f.len() as u64, f, 1000).unwrap();
    }
    assert_eq!(a.beacon_status(1000).unwrap().cached_messages, 12);
    // One real fragmented SYNC request, empty held bloom. First page is four
    // newest CHAT; the existing Sessions requires a bound continuation cursor.
    let mut payload = vec![0, 1, 0, 0, 0, 0, 0, 0];
    payload.extend([0; 512]);
    let mut raw = [0; 1024];
    let len = codec::serialize(
        codec::Header {
            kind: 3,
            flags: 0,
            ttl: 1,
            message_id: [99; 8],
            sender_id: [55; 8],
            channel_id: [0; 4],
        },
        &payload,
        codec::Context::Live,
        &mut raw,
    )
    .unwrap();
    b.enqueue(
        bl.clone(),
        raw[..len].to_vec(),
        TransportTraffic::Local,
        99,
        2000,
    )
    .unwrap();
    let mut request_events = Vec::new();
    for now in (2000..7000).step_by(1000) {
        for send in b.tick(now).unwrap().sends {
            request_events.extend(
                a.receive(al.clone(), send.bytes.len() as u64, send.bytes, now)
                    .unwrap()
                    .events,
            );
            b.complete(bl.clone(), send.token, true, now).unwrap();
        }
    }
    assert!(request_events.iter().any(|event| matches!(event,
        TransportEvent::Received { bytes, intake: TransportIntake::Unverified, .. } if bytes == &raw[..len])));
    assert_eq!(a.beacon_status(6000).unwrap().cached_messages, 12);
    let mut items = Vec::new();
    for now in (7000..20_000).step_by(1000) {
        for send in a.tick(now).unwrap().sends {
            for event in b
                .receive(bl.clone(), send.bytes.len() as u64, send.bytes, now)
                .unwrap()
                .events
            {
                if let TransportEvent::Received {
                    bytes,
                    intake: TransportIntake::DeferredSync,
                    ..
                } = event
                {
                    items.push(bytes);
                }
            }
            a.complete(al.clone(), send.token, true, now).unwrap();
        }
    }
    assert_eq!(items.len(), 5);
    for (i, item) in items[..4].iter().enumerate() {
        assert_eq!(item[4], 0);
        assert_eq!(item[11 + 4], 21 - i as u8);
        assert_eq!(item[11 + 3], 7); // Stored TTL is unchanged; never a live forward.
    }
    assert_eq!(items[4][4], 3); // Page marker; no unbounded replay.
    assert_eq!(a.beacon_status(20_000).unwrap().cached_messages, 12);
}

#[test]
fn announce_hint_tracks_external_power_and_ios_refuses_beacon() {
    use meshchat_core::native_channels::NativeChannels;
    let (a, identity) = owner(7);
    let channels = NativeChannels::new(identity.public_identity().unwrap(), 0).unwrap();
    let raw = channels.announce("Relay".into(), 1, 0, 200_000).unwrap();
    a.configure_beacon(true, false).unwrap();
    a.update_power(None, 80, true, 0, 0).unwrap();
    assert_eq!(a.beacon_announce(raw.clone()).unwrap()[32], 7);
    a.update_power(None, 80, false, 0, 1).unwrap();
    assert_eq!(a.beacon_announce(raw.clone()).unwrap()[32], 3);
    a.update_power(None, 30, false, 0, 2).unwrap();
    assert_eq!(a.beacon_announce(raw).unwrap()[32], 1);
    let store = EncryptedStore::open(
        Box::new(database::Database::new()),
        vec![7; 16],
        true,
        Some(200_000),
    )
    .unwrap();
    let ios = NativeTransport::new_ios(store, identity.public_identity().unwrap(), 8, 0).unwrap();
    assert_eq!(
        ios.configure_beacon(true, true).unwrap_err(),
        TransportError::Invalid
    );
}
