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
