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
fn fragments_retries_and_callbacks_are_not_message_delivery() {
    let (a, _) = owner(1);
    let (b, _) = owner(2);
    let al = ready(&a, TransportRole::Central, 146, 146, 0);
    let bl = ready(&b, TransportRole::Peripheral, 146, 146, 0);
    handshake(&a, &b, &al, &bl);
    a.reset_contribution_stats(0).unwrap();
    b.reset_contribution_stats(0).unwrap();
    a.enqueue(al.clone(), chat(3, 280), TransportTraffic::Forwarded, 3, 1)
        .unwrap();
    assert_eq!(a.contribution_stats(1).unwrap().completed_objects, 0);
    let failed = a.tick(1000).unwrap().sends.remove(0);
    a.complete(al.clone(), failed.token, false, 1000).unwrap();
    assert_eq!(a.contribution_stats(1000).unwrap().completed_frames, 0);
    for at in [2000, 3000, 4000] {
        let frame = a.tick(at).unwrap().sends.remove(0);
        b.receive(bl.clone(), frame.bytes.len() as u64, frame.bytes, at)
            .unwrap();
        a.complete(al.clone(), frame.token, true, at).unwrap();
        assert!(a.complete(al.clone(), frame.token, true, at).is_err());
    }
    let sent = a.contribution_stats(4000).unwrap();
    let received = b.contribution_stats(4000).unwrap();
    assert_eq!(sent.scheduled_frames, 4);
    assert_eq!(sent.completed_frames, 3);
    assert_eq!(sent.completed_objects, 1);
    assert_eq!(sent.relayed_chat_copies, 1);
    assert_eq!(received.received_frames, 3);
    assert_eq!(received.received_packets, 1);
    assert_eq!(received.received_chat_packets, 1);
    assert_eq!(sent.completed_bytes, received.received_bytes);
    a.enqueue(al.clone(), chat(4, 5), TransportTraffic::Own, 4, 4100)
        .unwrap();
    a.reset_contribution_stats(4100).unwrap();
    assert!(
        a.tick(4100).unwrap().sends.is_empty(),
        "reset must not refill send credits"
    );
    let after = a.contribution_stats(4100).unwrap();
    assert_eq!(after.elapsed_ms, 0);
    assert_eq!(after.completed_frames, 0);
    let next = a.tick(5000).unwrap().sends.remove(0);
    a.complete(al, next.token, true, 5000).unwrap();
    assert_eq!(a.contribution_stats(5000).unwrap().completed_objects, 1);
    assert_eq!(a.contribution_stats(5000).unwrap().relayed_chat_copies, 0);
}
#[test]
fn rejected_input_power_freshness_and_restart_are_explicit() {
    let (a, _) = owner(5);
    let (b, _) = owner(6);
    let al = ready(&a, TransportRole::Central, 146, 146, 0);
    let bl = ready(&b, TransportRole::Peripheral, 146, 146, 0);
    handshake(&a, &b, &al, &bl);
    b.reset_contribution_stats(0).unwrap();
    b.receive(bl, 3, vec![255, 0, 0], 1).unwrap();
    let s = b.contribution_stats(1).unwrap();
    assert_eq!(s.received_frames, 1);
    assert_eq!(s.received_packets, 0);
    assert_eq!(s.battery_percent, None);
    b.update_power(Some(TransportPowerSetting::Saver), 40, true, 0, 2)
        .unwrap();
    let s = b.contribution_stats(2).unwrap();
    assert_eq!(s.battery_percent, Some(40));
    assert!(s.charging);
    assert_eq!(s.power_mode, "Saver");
    b.reset_contribution_stats(3).unwrap();
    assert_eq!(b.contribution_stats(3).unwrap().battery_percent, Some(40));
    assert_eq!(b.contribution_stats(60003).unwrap().battery_percent, None);
    let (fresh, _) = owner(7);
    assert_eq!(fresh.contribution_stats(0).unwrap().received_frames, 0);
}
#[test]
fn sharing_has_only_selected_aggregate_categories_and_qualifiers() {
    use meshchat_core::native_stats::{TransportStats, contribution_share_text};
    let s = TransportStats {
        elapsed_ms: 120000,
        received_frames: 999,
        completed_frames: 888,
        relayed_chat_copies: 777,
        power_mode: "PRIVATE_SENTINEL".into(),
        ..Default::default()
    };
    let text = contribution_share_text(s.clone(), false, false, true);
    assert!(text.contains("777"));
    assert!(!text.contains("999"));
    assert!(!text.contains("888"));
    assert!(!text.contains("PRIVATE_SENTINEL"));
    assert!(text.contains("Delivery unknown"));
    let text = contribution_share_text(s, true, true, false);
    assert!(text.contains("999"));
    assert!(text.contains("888"));
    assert!(!text.contains("777"));
    assert!(text.contains("not unique messages"));
}
