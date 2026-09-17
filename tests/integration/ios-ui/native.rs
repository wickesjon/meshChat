#[allow(dead_code)]
#[path = "../friends/database.rs"]
mod database;
use meshchat_core::{
    LinkHandle, SendPath, codec, identity::IdentityKeySession, native_channels::NativeChannels,
    native_transport::*, storage::EncryptedStore,
};
use std::sync::Arc;

fn owner(
    seed: u8,
) -> (
    NativeTransport,
    Arc<IdentityKeySession>,
    Arc<EncryptedStore>,
    Arc<NativeChannels>,
) {
    let identity = IdentityKeySession::import_unlocked(vec![seed; 64], vec![seed; 16]).unwrap();
    let store = EncryptedStore::open(
        Box::new(database::Database::new()),
        vec![seed; 16],
        true,
        Some(200_000),
    )
    .unwrap();
    (
        NativeTransport::new(
            store.clone(),
            identity.public_identity().unwrap(),
            seed.into(),
            0,
        )
        .unwrap(),
        identity.clone(),
        store,
        Arc::new(NativeChannels::new(identity.public_identity().unwrap(), 0).unwrap()),
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

fn drain(
    node: &(
        NativeTransport,
        Arc<IdentityKeySession>,
        Arc<EncryptedStore>,
        Arc<NativeChannels>,
    ),
    now: u64,
) -> catchup::CatchupProgress {
    node.0
        .process_catchup(node.2.clone(), node.1.clone(), node.3.clone(), now, 200_000)
        .unwrap()
}
fn exchange(a: &NativeTransport, b: &NativeTransport, al: &LinkHandle, bl: &LinkHandle, now: u64) {
    for frame in a.tick(now).unwrap().sends {
        b.receive(bl.clone(), frame.bytes.len() as u64, frame.bytes, now)
            .unwrap();
        a.complete(al.clone(), frame.token, true, now).unwrap();
    }
    for frame in b.tick(now).unwrap().sends {
        a.receive(al.clone(), frame.bytes.len() as u64, frame.bytes, now)
            .unwrap();
        b.complete(bl.clone(), frame.token, true, now).unwrap();
    }
}
#[test]
fn native_request_pages_history_without_live_relay_or_delivery_claims() {
    let a = owner(51);
    let b = owner(52);
    let al = ready(&a.0, TransportRole::Central, 512, 512, 0);
    let bl = ready(&b.0, TransportRole::Peripheral, 512, 512, 0);
    handshake(&a.0, &b.0, &al, &bl);
    // Actual admitted live traffic populates the serving node's bounded cache.
    for id in 1..=6 {
        let raw = chat(id, 20);
        let encoder = meshchat_core::framing::Encoder::logical(&raw, 512, 0).unwrap();
        let mut frame = [0; 512];
        let len = encoder.frame(0, &mut frame).unwrap();
        a.0.receive(al.clone(), len as u64, frame[..len].to_vec(), id as u64)
            .unwrap();
    }
    b.0.request_catchup(bl.clone(), 10).unwrap();
    assert!(matches!(
        b.0.request_catchup(bl.clone(), 10),
        Err(TransportError::Busy)
    ));
    let mut arrived = 0;
    for now in (500..=20_000).step_by(500) {
        exchange(&a.0, &b.0, &al, &bl, now);
        arrived += drain(&b, now).arrivals.len();
    }
    let progress = drain(&b, 20_000);
    assert_eq!(progress.active, 0);
    assert_eq!(progress.complete, 1);
    assert_eq!(arrived, 6);
    let rows =
        b.3.history(b.2.clone(), "#general".into(), "Local".into())
            .unwrap();
    assert_eq!(rows.len(), 6);
    assert!(
        rows.iter()
            .all(|r| !r.signed && r.verified_petname.is_none())
    );
    let stats = b.0.contribution_stats(20_000).unwrap();
    assert_eq!(stats.received_chat_packets, 0);
    assert_eq!(stats.relayed_chat_copies, 0);
    assert_eq!(
        b.0.beacon_status(20_000).unwrap().cached_messages,
        0,
        "stored content never becomes a live forward cache entry"
    );
}
fn stored_frame(raw: &[u8], session: u16) -> Vec<u8> {
    let mut item = session.to_be_bytes().to_vec();
    item.extend([0, 0, 0, 0, 0, 0, 0]);
    item.extend((raw.len() as u16).to_be_bytes());
    item.extend(raw);
    let encoder = meshchat_core::framing::Encoder::transport(1, &item, 512, 0).unwrap();
    let mut frame = [0; 512];
    let len = encoder.frame(0, &mut frame).unwrap();
    frame[..len].to_vec()
}
#[test]
fn unsolicited_stale_and_zero_ttl_stored_values_obey_session_admission() {
    let a = owner(53);
    let b = owner(54);
    let al = ready(&a.0, TransportRole::Central, 512, 512, 0);
    let bl = ready(&b.0, TransportRole::Peripheral, 512, 512, 0);
    handshake(&a.0, &b.0, &al, &bl);
    let mut raw = chat(9, 8);
    raw[3] = 0;
    let frame = stored_frame(&raw, 1);
    b.0.receive(bl.clone(), frame.len() as u64, frame.clone(), 1)
        .unwrap();
    assert!(drain(&b, 1).arrivals.is_empty());
    b.0.request_catchup(bl.clone(), 2).unwrap();
    // Native first-attempt callbacks start the absolute requester deadline.
    for now in [500, 1000] {
        for frame in b.0.tick(now).unwrap().sends {
            b.0.complete(bl.clone(), frame.token, true, now).unwrap();
        }
    }
    b.0.receive(bl.clone(), frame.len() as u64, frame, 1001)
        .unwrap();
    assert_eq!(drain(&b, 1001).arrivals.len(), 1);
    assert_eq!(
        b.3.history(b.2.clone(), "#general".into(), "Local".into())
            .unwrap()
            .len(),
        1
    );
    let other = owner(55);
    assert!(
        b.0.process_catchup(other.2, other.1, other.3, 1002, 200_000)
            .is_err()
    );
    b.0.disconnect(bl.clone(), 1003).unwrap();
    assert!(b.0.request_catchup(bl, 1004).is_err());
    assert_eq!(drain(&b, 1004).active, 0);
}
#[test]
fn untransmitted_request_expires_and_reset_does_not_refill_request_budget() {
    let a = owner(56);
    let b = owner(57);
    let al = ready(&a.0, TransportRole::Central, 146, 146, 0);
    let bl = ready(&b.0, TransportRole::Peripheral, 146, 146, 0);
    handshake(&a.0, &b.0, &al, &bl);
    b.0.request_catchup(bl.clone(), 1).unwrap();
    b.0.reset_contribution_stats(2).unwrap();
    assert!(matches!(
        b.0.request_catchup(bl.clone(), 2),
        Err(TransportError::Busy)
    ));
    let expired = drain(&b, 30_001);
    assert_eq!(expired.active, 0);
    assert_eq!(expired.incomplete, 1);
    assert!(matches!(
        b.0.request_catchup(bl, 30_001),
        Err(TransportError::Busy)
    ));
}

#[test]
fn catchup_signed_and_encrypted_history_uses_real_pin_and_crypto_owners() {
    use meshchat_core::native_messaging::{DirectContent, friend_code};
    let a = owner(61);
    let b = owner(62);
    let ac = friend_code(a.1.public_identity().unwrap(), "Alice".into()).unwrap();
    let bc = friend_code(b.1.public_identity().unwrap(), "Bob".into()).unwrap();
    a.0.confirm_friend(a.2.clone(), a.1.clone(), bc.uri, "Bob".into(), None, 0)
        .unwrap();
    b.0.confirm_friend(b.2.clone(), b.1.clone(), ac.uri, "Alice".into(), None, 0)
        .unwrap();
    let pin = a.0.friend_cards(a.2.clone(), 0).unwrap().remove(0);
    let al = ready(&a.0, TransportRole::Central, 512, 512, 0);
    let bl = ready(&b.0, TransportRole::Peripheral, 512, 512, 0);
    handshake(&a.0, &b.0, &al, &bl);
    a.0.send_direct(
        a.2.clone(),
        a.1.clone(),
        pin.handle,
        DirectContent::Chat {
            text: "Encrypted historical message".into(),
        },
        3,
        1,
        200_000,
    )
    .unwrap();
    let raw =
        a.3.compose(
            "melodic|techno|valley".into(),
            "Alice".into(),
            1,
            "Signed historical message".into(),
            200_000,
            2,
        )
        .unwrap();
    a.0.send_signed(a.2.clone(), a.1.clone(), raw, 4, 2, 200_000)
        .unwrap();
    // Successful local completions with no delivery to B leave only A's cache.
    for now in (500..=8000).step_by(500) {
        for frame in a.0.tick(now).unwrap().sends {
            a.0.authorize_message_egress(a.2.clone(), al.clone(), frame.token)
                .unwrap();
            a.0.complete(al.clone(), frame.token, true, now).unwrap();
        }
    }
    assert!(
        b.0.direct_history(b.2.clone(), ac.keys.clone(), 8000, 200_000)
            .unwrap()
            .is_empty()
    );
    b.0.request_catchup(bl.clone(), 8001).unwrap();
    for now in (8500..=25000).step_by(500) {
        exchange(&a.0, &b.0, &al, &bl, now);
        drain(&b, now);
    }
    let rows =
        b.0.direct_history(b.2.clone(), ac.keys, 25000, 200_000)
            .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].text, "Encrypted historical message");
    let signed =
        b.0.messaging_channel_history(
            b.2.clone(),
            b.3.clone(),
            "melodic|techno|valley".into(),
            "Bob".into(),
        )
        .unwrap();
    assert_eq!(signed.len(), 1);
    assert!(signed[0].signed);
    assert_eq!(signed[0].verified_petname.as_deref(), Some("Alice"));
    assert_eq!(drain(&b, 25000).complete, 1);
}
