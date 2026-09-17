#[allow(dead_code)]
#[path = "../friends/database.rs"]
mod database;
use ed25519_dalek::{Signer, SigningKey};
use meshchat_core::{
    LinkHandle, identity::IdentityKeySession, native_organizer::*, native_transport::*,
    storage::EncryptedStore,
};
use sha2::{Digest, Sha256};
use std::sync::Arc;
fn key(n: u8) -> SigningKey {
    SigningKey::from_bytes(&[n; 32])
}
fn hint(k: &SigningKey) -> [u8; 8] {
    Sha256::digest(k.verifying_key().as_bytes())[..8]
        .try_into()
        .unwrap()
}
fn root(n: u8, expiry: u32) -> String {
    let k = key(n);
    let mut b = [0; 101];
    b[0] = 1;
    b[1..33].copy_from_slice(k.verifying_key().as_bytes());
    b[33..37].copy_from_slice(&expiry.to_be_bytes());
    let mut signed = b"meshfest/event-root/v1\0".to_vec();
    signed.extend_from_slice(&b[..37]);
    b[37..].copy_from_slice(&k.sign(&signed).to_bytes());
    format!("meshfest://event/{}/Festival", b64(&b))
}
fn cred(r: u8, s: u8, before: u32, after: u32) -> Vec<u8> {
    let root = key(r);
    let staff = key(s);
    let mut out = vec![1];
    out.extend_from_slice(&hint(&root));
    out.extend_from_slice(staff.verifying_key().as_bytes());
    out.extend_from_slice(&before.to_be_bytes());
    out.extend_from_slice(&after.to_be_bytes());
    out.push(3);
    out.extend_from_slice(b"Ops");
    let mut signed = b"meshfest/credential/v1\0".to_vec();
    signed.extend_from_slice(&(out.len() as u16).to_be_bytes());
    signed.extend_from_slice(&out);
    out.extend_from_slice(&root.sign(&signed).to_bytes());
    out
}
fn b64(raw: &[u8]) -> String {
    const ABC: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::new();
    let mut acc = 0u32;
    let mut bits = 0;
    for b in raw {
        acc = (acc << 8) | u32::from(*b);
        bits += 8;
        while bits >= 6 {
            bits -= 6;
            out.push(ABC[((acc >> bits) & 63) as usize] as char);
        }
    }
    if bits > 0 {
        out.push(ABC[((acc << (6 - bits)) & 63) as usize] as char);
    }
    out
}

struct Device {
    core: NativeTransport,
    store: Arc<EncryptedStore>,
}
impl Device {
    fn new(seed: u8) -> Self {
        let key = IdentityKeySession::import_unlocked(vec![seed; 64], vec![seed; 16]).unwrap();
        let store = EncryptedStore::open(
            Box::new(database::Database::new()),
            vec![seed; 16],
            true,
            Some(200000),
        )
        .unwrap();
        let core = NativeTransport::new(
            store.clone(),
            key.public_identity().unwrap(),
            seed.into(),
            0,
        )
        .unwrap();
        Self { core, store }
    }
    fn adopt(&self) {
        self.core
            .confirm_event(self.store.clone(), root(9, 202000), 0, 200000)
            .unwrap()
    }
    fn staff(&self, n: u8) -> Arc<NativeStaffSession> {
        self.core
            .import_staff_key(
                self.store.clone(),
                format!(
                    "meshfest://staff/{}/{}",
                    b64(&cred(9, n, 199990, 201000)),
                    b64(&[n; 32])
                ),
                1000,
                200000,
            )
            .unwrap()
    }
    fn rows(&self, now: u64, wall: i64) -> Vec<EventMessage> {
        self.core
            .event_messages(self.store.clone(), wall, now)
            .unwrap()
    }
}
fn connect(a: &Device, b: &Device, now: u64) -> (LinkHandle, LinkHandle) {
    let ready = |d: &Device, role| {
        let p = d.core.admit_connection(vec![9; 16], now).unwrap();
        d.core.native_ready(p, role, 512, 512, now).unwrap().link
    };
    let al = ready(a, TransportRole::Central);
    let bl = ready(b, TransportRole::Peripheral);
    let af = a.core.tick(now).unwrap().sends.remove(0);
    let bf = b.core.tick(now).unwrap().sends.remove(0);
    b.core
        .receive(bl.clone(), af.bytes.len() as u64, af.bytes, now)
        .unwrap();
    a.core
        .receive(al.clone(), bf.bytes.len() as u64, bf.bytes, now)
        .unwrap();
    a.core.complete(al.clone(), af.token, true, now).unwrap();
    b.core.complete(bl.clone(), bf.token, true, now).unwrap();
    (al, bl)
}
fn pump(a: &Device, b: &Device, bl: &LinkHandle, start: u64) -> Vec<Vec<u8>> {
    let mut logical = vec![];
    for step in 0..50 {
        let now = start + step * 100;
        let effects = a.core.tick(now).unwrap();
        for frame in effects.sends {
            if a.core
                .message_needs_authorization(frame.link.clone(), frame.token)
                .unwrap()
            {
                assert!(
                    a.core
                        .authorize_organizer_egress(
                            a.store.clone(),
                            frame.link.clone(),
                            frame.token,
                            200000
                        )
                        .unwrap()
                );
            }
            for event in b
                .core
                .receive(bl.clone(), frame.bytes.len() as u64, frame.bytes, now)
                .unwrap()
                .events
            {
                if let TransportEvent::Received { bytes, .. } = event {
                    logical.push(bytes);
                }
            }
            a.core.complete(frame.link, frame.token, true, now).unwrap();
        }
    }
    logical
}
#[test]
fn native_signed_updates_cross_nonadopting_bridge_and_expire() {
    let a = Device::new(1);
    let relay = Device::new(2);
    let b = Device::new(3);
    a.adopt();
    b.adopt();
    let session = a.staff(7);
    let (_, rl) = connect(&a, &relay, 1000);
    let (rb, bl) = connect(&relay, &b, 1000);
    let sent = a
        .core
        .post_event(
            a.store.clone(),
            session.clone(),
            "Stage".into(),
            "Gate opens".into(),
            1,
            Some(200500),
            10,
            2000,
            200000,
        )
        .unwrap();
    assert!(sent.queued);
    session.invalidate();
    let raws = pump(&a, &relay, &rl, 2000);
    assert!(
        relay
            .core
            .event_cards(relay.store.clone(), 200000)
            .unwrap()
            .is_empty()
    );
    assert!(!raws.is_empty(), "sender produced no logical packets");
    for raw in raws {
        relay
            .core
            .enqueue(rb.clone(), raw, TransportTraffic::Forwarded, 11, 7000)
            .unwrap();
    }
    let arrived = pump(&relay, &b, &bl, 7500);
    assert!(
        arrived.iter().any(|r| r[1] == 1),
        "forwarded chat absent; kinds {:?}",
        arrived.iter().map(|r| r[1]).collect::<Vec<_>>()
    );
    for i in 0..4 {
        b.core
            .organizer_tick(b.store.clone(), None, 13000 + i, 200000)
            .unwrap();
    }
    let rows = b.rows(14000, 200000);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].staff_label.as_deref(), Some("Ops"));
    assert!(rows[0].pinned);
    assert!(
        relay
            .rows(14000, 200000)
            .iter()
            .all(|r| r.staff_label.is_none())
    );
    assert!(!b.rows(14001, 200501)[0].pinned);
    let key = event_proposal(root(9, 202000)).unwrap().key;
    b.core.remove_event(b.store.clone(), key, 200501).unwrap();
    b.core
        .confirm_event(b.store.clone(), root(9, 202000), 14002, 200501)
        .unwrap();
    assert!(b.rows(14002, 200501)[0].staff_label.is_none());
    assert!(b.rows(14003, 201001)[0].staff_label.is_none());
}
#[test]
fn adoption_import_and_egress_recheck_fail_closed() {
    let a = Device::new(5);
    let b = Device::new(6);
    let uri = format!(
        "meshfest://staff/{}/{}",
        b64(&cred(9, 7, 199990, 201000)),
        b64(&[7; 32])
    );
    assert!(
        a.core
            .import_staff_key(a.store.clone(), uri.clone(), 0, 200000)
            .is_err()
    );
    a.adopt();
    let session = a.staff(7);
    let wrong = format!(
        "meshfest://staff/{}/{}",
        b64(&cred(9, 7, 199990, 201000)),
        b64(&[8; 32])
    );
    assert!(
        a.core
            .import_staff_key(a.store.clone(), wrong, 1000, 200000)
            .is_err()
    );
    let (al, _) = connect(&a, &b, 1000);
    a.core
        .post_event(
            a.store.clone(),
            session,
            "Stage".into(),
            "Hello".into(),
            1,
            None,
            21,
            2000,
            200000,
        )
        .unwrap();
    let frame = a.core.tick(2000).unwrap().sends.remove(0);
    assert!(
        a.core
            .authorize_message_egress(a.store.clone(), al.clone(), frame.token)
            .is_err()
    );
    a.core.forget_staff_operations().unwrap();
    assert!(
        a.core
            .authorize_organizer_egress(a.store.clone(), al, frame.token, 200000)
            .is_err()
    );
    assert_eq!(
        a.core.event_cards(a.store.clone(), 200000).unwrap().len(),
        1
    );
}
#[test]
fn generate_native_organizer_fixtures() {
    let wall = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as u32;
    let directory =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.work/mc030/fixtures");
    std::fs::create_dir_all(&directory).unwrap();
    let staff = |seed: u8, before: u32, after: u32| {
        format!(
            "meshfest://staff/{}/{}",
            b64(&cred(9, seed, before, after)),
            b64(&[seed; 32])
        )
    };
    let content = format!(
        "event\t{}\nstaff\t{}\nexpired\t{}\nother\t{}\n",
        root(9, wall + 7200),
        staff(7, wall - 10, wall + 3600),
        staff(7, wall - 100, wall - 1),
        root(8, wall + 7200)
    );
    std::fs::write(directory.join("events.tsv"), content).unwrap();
}
