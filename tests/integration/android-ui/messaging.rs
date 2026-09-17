//! Real core crypto + native transport + production SQL against the SQLite
//! double. SQLCipher/native protected-adapter evidence is collected separately.
#[allow(dead_code)]
#[path = "../friends/database.rs"]
mod database;
use meshchat_core::{
    LinkHandle, identity::IdentityKeySession, native_messaging::*, native_transport::*,
    storage::EncryptedStore,
};
use std::sync::Arc;
struct Device {
    core: NativeTransport,
    store: Arc<EncryptedStore>,
    key: Arc<IdentityKeySession>,
}
impl Device {
    fn new(seed: u8) -> Self {
        let key = IdentityKeySession::import_unlocked(vec![seed; 64], vec![seed; 16]).unwrap();
        let store = EncryptedStore::open(
            Box::new(database::Database::new()),
            vec![seed; 16],
            true,
            Some(200_000),
        )
        .unwrap();
        let core = NativeTransport::new(
            store.clone(),
            key.public_identity().unwrap(),
            u64::from(seed),
            0,
        )
        .unwrap();
        Self { core, store, key }
    }
    fn code(&self) -> FriendProposal {
        friend_code(self.key.public_identity().unwrap(), "Sarah / & é".into()).unwrap()
    }
    fn pin(&self, other: &Self) -> FriendCard {
        self.core
            .confirm_friend(
                self.store.clone(),
                self.key.clone(),
                other.code().uri,
                "Sarah".into(),
                None,
                0,
            )
            .unwrap();
        self.cards(0).remove(0)
    }
    fn cards(&self, now: u64) -> Vec<FriendCard> {
        self.core.friend_cards(self.store.clone(), now).unwrap()
    }
    fn ready(&self, role: TransportRole, now: u64) -> LinkHandle {
        let permit = self.core.admit_connection(vec![9; 16], now).unwrap();
        self.core
            .native_ready(permit, role, 512, 512, now)
            .unwrap()
            .link
    }
    fn send(&self, pin: &FriendCard, text: &str, now: u64) -> MessageSubmission {
        self.core
            .send_direct(
                self.store.clone(),
                self.key.clone(),
                pin.handle.clone(),
                DirectContent::Chat { text: text.into() },
                now + 3,
                now,
                200_000,
            )
            .unwrap()
    }
    fn history(&self, pin: &FriendCard, now: u64) -> Vec<DirectMessage> {
        self.core
            .direct_history(self.store.clone(), pin.keys.clone(), now, 200_000)
            .unwrap()
    }
    fn accept(&self, link: &LinkHandle, bytes: Vec<u8>, now: u64) -> AuthenticationResult {
        self.core
            .authenticate_message(
                self.store.clone(),
                self.key.clone(),
                link.clone(),
                bytes,
                now,
                200_000,
            )
            .unwrap()
    }
}
fn connect(a: &Device, b: &Device, now: u64) -> (LinkHandle, LinkHandle) {
    let al = a.ready(TransportRole::Central, now);
    let bl = b.ready(TransportRole::Peripheral, now);
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
fn wire(a: &Device, al: &LinkHandle, now: u64) -> Vec<u8> {
    let mut raw = vec![];
    for step in 0..8 {
        let now = now + step * 1000;
        let frame = a.core.tick(now).unwrap().sends.remove(0);
        assert!(
            a.core
                .message_needs_authorization(al.clone(), frame.token)
                .unwrap()
        );
        a.core
            .authorize_message_egress(a.store.clone(), al.clone(), frame.token)
            .unwrap();
        match meshchat_core::framing::parse_frame(&frame.bytes, 146).unwrap() {
            meshchat_core::framing::Frame::Logical(bytes) => raw.extend_from_slice(bytes),
            meshchat_core::framing::Frame::Fragment(f) => raw.extend_from_slice(f.slice),
            _ => panic!("logical expected"),
        }
        let effects = a.core.complete(al.clone(), frame.token, true, now).unwrap();
        if effects
            .events
            .iter()
            .any(|e| matches!(e, TransportEvent::Finished { .. }))
        {
            return raw;
        }
    }
    panic!("bounded send failed")
}
fn intake(b: &Device, bl: &LinkHandle, raw: &[u8], now: u64) -> TransportIntake {
    let mut frame = vec![0, 0];
    frame.extend_from_slice(&(raw.len() as u16).to_be_bytes());
    frame.extend_from_slice(raw);
    let batch = b
        .core
        .receive(bl.clone(), frame.len() as u64, frame, now)
        .unwrap();
    match &batch.events[0] {
        TransportEvent::Received { intake, .. } => intake.clone(),
        _ => panic!("expected intake"),
    }
}
#[test]
fn code_is_inert_full_tuple_fingerprint_and_explicit_pin_survives_restart() {
    let a = Device::new(1);
    let b = Device::new(2);
    let proposal = friend_proposal(b.code().uri).unwrap();
    assert_eq!(proposal.nickname, "Sarah / & é");
    assert_eq!(proposal.keys.len(), 64);
    assert_eq!(proposal.fingerprint.len(), 79);
    assert!(a.cards(0).is_empty());
    for raw in [
        "meshfest://friend/AAAA/x",
        "https://evil.test/friend/x/y",
        "meshfest://friend/AAAA/x?y",
    ] {
        assert!(friend_proposal(raw.into()).is_err())
    }
    let pin = a.pin(&b);
    assert!(!pin.fresh);
    let reopened =
        NativeTransport::new(a.store.clone(), a.key.public_identity().unwrap(), 99, 0).unwrap();
    assert_eq!(
        reopened.friend_cards(a.store.clone(), 0).unwrap()[0].keys,
        proposal.keys
    );
}
#[test]
fn encrypted_chat_reaction_invalid_before_valid_replay_and_restart() {
    let a = Device::new(1);
    let b = Device::new(2);
    let ab = a.pin(&b);
    let ba = b.pin(&a);
    let (al, bl) = connect(&a, &b, 0);
    let sent = a.send(&ab, "Secret hello", 1000);
    assert!(sent.queued);
    let raw = wire(&a, &al, 1000);
    assert!(!raw.windows(6).any(|s| s == b"Secret"));
    let mut invalid = raw.clone();
    *invalid.last_mut().unwrap() ^= 1;
    assert_eq!(intake(&b, &bl, &invalid, 1000), TransportIntake::Pending);
    assert!(!b.accept(&bl, invalid, 1000).authenticated);
    assert!(b.history(&ba, 1000).is_empty());
    assert_eq!(intake(&b, &bl, &raw, 2000), TransportIntake::Pending);
    assert!(b.accept(&bl, raw.clone(), 2000).changed);
    assert_eq!(b.history(&ba, 2000)[0].text, "Secret hello");
    intake(&b, &bl, &raw, 3000);
    assert!(!b.accept(&bl, raw, 3000).changed);
    assert_eq!(b.history(&ba, 3000).len(), 1);
    b.core
        .send_direct(
            b.store.clone(),
            b.key.clone(),
            ba.handle.clone(),
            DirectContent::Reaction {
                target: sent.id.clone(),
                remove: false,
                code: 1,
            },
            7,
            4000,
            200_000,
        )
        .unwrap();
    let reaction = wire(&b, &bl, 4000);
    intake(&a, &al, &reaction, 4000);
    assert!(a.accept(&al, reaction, 4000).authenticated);
    assert_eq!(a.history(&ab, 4000)[0].reactions[1], 1);
    let restarted =
        NativeTransport::new(a.store.clone(), a.key.public_identity().unwrap(), 99, 0).unwrap();
    let history = restarted
        .direct_history(a.store.clone(), ab.keys, 0, 200_000)
        .unwrap();
    assert_eq!(history[0].text, "Secret hello");
    assert_eq!(history[0].reactions[1], 1);
}
#[test]
fn missing_pin_never_decrypts_or_falls_back_and_unstaged_bytes_have_no_authority() {
    let a = Device::new(1);
    let b = Device::new(2);
    let ab = a.pin(&b);
    let (al, bl) = connect(&a, &b, 0);
    a.send(&ab, "Hidden", 1000);
    let raw = wire(&a, &al, 1000);
    assert!(!b.accept(&bl, raw.clone(), 1000).authenticated);
    intake(&b, &bl, &raw, 2000);
    assert!(!b.accept(&bl, raw, 2000).authenticated);
    assert!(b.cards(2000).is_empty());
}
#[test]
fn removal_during_queued_send_fails_egress_even_if_other_owner_changed_store() {
    let a = Device::new(1);
    let b = Device::new(2);
    let ab = a.pin(&b);
    b.pin(&a);
    let (al, _) = connect(&a, &b, 0);
    a.send(&ab, "Must not leave", 1000);
    let frame = a.core.tick(1000).unwrap().sends.remove(0);
    let other =
        NativeTransport::new(a.store.clone(), a.key.public_identity().unwrap(), 99, 0).unwrap();
    other
        .change_friend(a.store.clone(), ab.handle.clone(), false, 0)
        .unwrap();
    assert!(
        a.core
            .authorize_message_egress(a.store.clone(), al, frame.token)
            .is_err()
    );
    assert!(
        a.core
            .send_direct(
                a.store.clone(),
                a.key.clone(),
                ab.handle,
                DirectContent::Chat {
                    text: "still no".into()
                },
                4,
                2000,
                200_000
            )
            .is_err()
    );
}
#[test]
fn replacement_requires_stopped_links_and_retains_separate_history() {
    let a = Device::new(1);
    let b = Device::new(2);
    let c = Device::new(3);
    let ab = a.pin(&b);
    b.pin(&a);
    let (al, bl) = connect(&a, &b, 0);
    a.send(&ab, "Old identity", 1000);
    assert!(
        a.core
            .change_friend(a.store.clone(), ab.handle.clone(), true, 1000)
            .is_err()
    );
    a.core.disconnect(al, 1000).unwrap();
    b.core.disconnect(bl, 1000).unwrap();
    a.core
        .change_friend(a.store.clone(), ab.handle.clone(), true, 1000)
        .unwrap();
    assert!(a.cards(1000)[0].replacing);
    a.core
        .confirm_friend(
            a.store.clone(),
            a.key.clone(),
            c.code().uri,
            "New device".into(),
            Some(ab.handle.clone()),
            1000,
        )
        .unwrap();
    let ac = a.cards(1000).remove(0);
    assert_ne!(ac.keys, ab.keys);
    assert_eq!(a.history(&ab, 1000)[0].text, "Old identity");
    assert!(a.history(&ac, 1000).is_empty());
}
#[test]
fn proof_presence_expires_without_nickname_or_rssi_authority() {
    let a = Device::new(1);
    let b = Device::new(2);
    a.pin(&b);
    b.pin(&a);
    let (al, bl) = connect(&a, &b, 0);
    a.core.prepare_proof(al.clone(), a.key.clone(), 0).unwrap();
    b.core.prepare_proof(bl.clone(), b.key.clone(), 0).unwrap();
    for (sender, receiver, sl, rl) in [(&a, &b, &al, &bl), (&b, &a, &bl, &al)] {
        let p = sender.core.tick(1000).unwrap().sends.remove(0);
        receiver
            .core
            .receive(rl.clone(), p.bytes.len() as u64, p.bytes, 1000)
            .unwrap();
        sender
            .core
            .complete(sl.clone(), p.token, true, 1000)
            .unwrap();
    }
    assert!(a.cards(1000)[0].fresh);
    assert!(!a.cards(60_000)[0].fresh);
    assert_eq!(a.cards(60_000)[0].response_age_ms, Some(59_000));
    a.core.disconnect(al, 60_000).unwrap();
    assert!(!a.cards(60_001)[0].fresh);
}

#[test]
fn signed_channel_history_uses_pinned_petname_and_unsigned_copycats_warn() {
    use meshchat_core::native_channels::{ChannelReceipt, NativeChannels};
    let a = Device::new(1);
    let b = Device::new(2);
    a.pin(&b);
    b.pin(&a);
    let (al, bl) = connect(&a, &b, 0);
    let ac = Arc::new(NativeChannels::new(a.key.public_identity().unwrap(), 0).unwrap());
    let bc = Arc::new(NativeChannels::new(b.key.public_identity().unwrap(), 0).unwrap());
    let raw = ac
        .compose(
            "melodic|techno|valley".into(),
            "Impersonated name".into(),
            0x23,
            "Verified body".into(),
            200000,
            1000,
        )
        .unwrap();
    a.core
        .send_signed(a.store.clone(), a.key.clone(), raw, 8, 1000, 200000)
        .unwrap();
    let signed = wire(&a, &al, 1000);
    assert_eq!(signed[2] & 7, 2);
    intake(&b, &bl, &signed, 3000);
    assert!(b.accept(&bl, signed.clone(), 3000).authenticated);
    assert!(
        !bc.accept(
            b.store.clone(),
            ChannelReceipt {
                link: Some(bl),
                bytes: signed,
                intake: TransportIntake::Unverified,
                own: false,
                now: 3000,
                wall: 200000
            }
        )
        .unwrap()
    );
    assert!(
        bc.history(
            b.store.clone(),
            "melodic|techno|valley".into(),
            "Bob".into()
        )
        .unwrap()
        .is_empty()
    );
    let rows = b
        .core
        .messaging_channel_history(
            b.store.clone(),
            bc.clone(),
            "melodic|techno|valley".into(),
            "Bob".into(),
        )
        .unwrap();
    assert_eq!(rows[0].verified_petname.as_deref(), Some("Sarah"));
    assert_eq!(rows[0].nickname, "Sarah");
    let unsigned = ac
        .compose(
            "#general".into(),
            "Sarah".into(),
            1,
            "Copied claim".into(),
            200000,
            4000,
        )
        .unwrap();
    assert!(
        bc.accept(
            b.store.clone(),
            ChannelReceipt {
                link: None,
                bytes: unsigned,
                intake: TransportIntake::Unverified,
                own: false,
                now: 4000,
                wall: 200000
            }
        )
        .unwrap()
    );
    let rows = b
        .core
        .messaging_channel_history(b.store.clone(), bc, "#general".into(), "Bob".into())
        .unwrap();
    assert!(!rows[0].signed);
    assert!(rows[0].verified_petname.is_none());
    assert!(rows[0].claim_warning.is_some());
    assert!(!b.cards(4000)[0].fresh);
}

#[test]
fn pending_omitted_key_recovers_after_included_key_and_invalidated_provider_refuses() {
    use meshchat_core::native_channels::NativeChannels;
    let a = Device::new(1);
    let b = Device::new(2);
    let (al, bl) = connect(&a, &b, 0);
    let channel = NativeChannels::new(a.key.public_identity().unwrap(), 0).unwrap();
    let raw = channel
        .compose(
            "#general".into(),
            "Alice".into(),
            1,
            "First full key".into(),
            200000,
            1000,
        )
        .unwrap();
    a.core
        .send_signed(a.store.clone(), a.key.clone(), raw, 8, 1000, 200000)
        .unwrap();
    let included = wire(&a, &al, 1000); // Deliberately lost on receiver side.
    let raw = channel
        .compose(
            "#general".into(),
            "Alice".into(),
            1,
            "Missing key".into(),
            200000,
            5000,
        )
        .unwrap();
    a.core
        .send_signed(a.store.clone(), a.key.clone(), raw, 9, 5000, 200000)
        .unwrap();
    let omitted = wire(&a, &al, 5000);
    assert_eq!(
        included.len() - "First full key".len(),
        omitted.len() - "Missing key".len() + 32
    );
    intake(&b, &bl, &omitted, 7000);
    assert!(!b.accept(&bl, omitted, 7000).authenticated);
    intake(&b, &bl, &included, 8000);
    assert!(b.accept(&bl, included, 8000).authenticated);
    let mut changed = false;
    for i in 0..8 {
        changed |= b
            .core
            .retry_messages(b.store.clone(), b.key.clone(), 9000 + i, 200000)
            .unwrap();
    }
    assert!(changed);
    assert_eq!(
        b.store
            .history(
                meshchat_core::channel::parse_name("#general")
                    .unwrap()
                    .id()
                    .to_vec(),
                false,
                100
            )
            .unwrap()
            .len(),
        2
    );
    b.key.invalidate().unwrap();
    assert!(
        b.core
            .authenticate_message(b.store.clone(), b.key.clone(), bl, vec![], 10000, 200000)
            .is_err()
    );
}
