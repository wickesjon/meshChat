#[path = "../friends/database.rs"]
mod database;
use meshchat_core::native_transport::{
    NativeTransport, TransportEvent, TransportRole, TransportTraffic,
};
use meshchat_core::{
    LinkHandle, codec, identity::IdentityKeySession, native_channels::*,
    native_transport::TransportIntake, storage::EncryptedStore,
};
use std::sync::Arc;
fn node(seed: u8) -> (database::Database, Arc<EncryptedStore>, NativeChannels) {
    let identity = IdentityKeySession::import_unlocked(vec![seed; 64], vec![seed; 16])
        .unwrap()
        .public_identity()
        .unwrap();
    let db = database::Database::new();
    let store = EncryptedStore::open(
        Box::new(db.clone()),
        identity.generation.clone(),
        true,
        Some(200000),
    )
    .unwrap();
    (db, store, NativeChannels::new(identity, 0).unwrap())
}
fn chat(n: &NativeChannels, name: &str, text: &str, now: u64) -> Vec<u8> {
    n.compose(name.into(), "Alice".into(), 0x23, text.into(), 7, now)
        .unwrap()
}
fn accept(
    n: &NativeChannels,
    store: &Arc<EncryptedStore>,
    bytes: Vec<u8>,
    own: bool,
    now: u64,
) -> bool {
    n.accept(
        store.clone(),
        ChannelReceipt {
            link: Some(LinkHandle {
                instance_nonce: 1,
                generation: 1,
            }),
            bytes,
            intake: TransportIntake::Unverified,
            own,
            wall: 200000 + (now / 1000) as i64,
            now,
        },
    )
    .unwrap()
}
#[test]
fn channel_links_are_inert_canonical_proposals() {
    let expected = channel_info("melodic|techno|valley".into()).unwrap();
    for uri in [
        "meshfest://j/melodic-techno-valley",
        "MESHFEST://J/MELODIC-techno-VALLEY",
        "https://meshfest.app/j/melodic-techno-valley",
    ] {
        let proposal = channel_link(uri.into()).unwrap();
        assert_eq!(proposal.name, expected.name);
        assert_eq!(proposal.id, expected.id);
    }
    for uri in [
        "meshfest://j/madeup-techno-valley",
        "meshfest://j/melodic-techno-valley/",
        "meshfest://j/melodic-techno-valley?join=1",
        "meshfest://j/melodic-techno-valley#fragment",
        "meshfest://j/melodic%2Dtechno-valley",
        "https://meshfest.app:443/j/melodic-techno-valley",
        "https://user@meshfest.app/j/melodic-techno-valley",
        "https://meshfest.app/J/melodic-techno-valley",
        "https://evil.example/j/melodic-techno-valley",
        "meshfest://friend/invalid/Alice",
        "meshfest://staff/invalid/invalid",
    ] {
        assert!(channel_link(uri.into()).is_err());
    }
    assert!(channel_link("x".repeat(2049)).is_err());
}
#[test]
fn public_and_private_channels_use_canonical_core_rules() {
    let c = channel_info(" MELODIC|techno|VALLEY ".into()).unwrap();
    assert!(c.private);
    assert_ne!(c.glyph, "wave");
    assert_eq!(
        c.id,
        channel_info("melodic|techno|valley".into()).unwrap().id
    );
    assert!(channel_info("madeup|techno|valley".into()).is_err());
    let words = channel_words();
    assert_eq!(words.descriptors.len(), 20);
    assert_eq!(channel_info("#Confessions".into()).unwrap().glyph, "fire");
}
#[test]
fn text_limits_anonymous_identity_and_sender_buckets_are_real() {
    let (_, _, n) = node(111);
    let a = chat(&n, "#confessions", &"🎉".repeat(70), 0);
    let b = chat(&n, "#confessions", "second", 0);
    let pa = codec::parse(&a, codec::Context::Live).unwrap();
    let pb = codec::parse(&b, codec::Context::Live).unwrap();
    assert_ne!(pa.header().sender_id, pb.header().sender_id);
    assert_ne!(pa.header().message_id, pb.header().message_id);
    match pa.payload() {
        codec::Payload::Chat {
            avatar,
            nickname,
            cosmetics,
            ..
        } => {
            assert_eq!(avatar, 0);
            assert_eq!(nickname, "Anonymous");
            assert_eq!(cosmetics, [0; 4]);
        }
        _ => panic!(),
    }
    assert!(
        n.compose("#general".into(), "Alice".into(), 1, "🎉".repeat(71), 0, 0)
            .is_err()
    );
    assert!(
        n.compose(
            "#general".into(),
            "Alice".into(),
            1,
            "a\u{202e}b".into(),
            0,
            0
        )
        .is_err()
    );
    for _ in 0..3 {
        chat(&n, "#general", "hello", 0);
    }
    assert_eq!(n.wait_ms("#confessions".into(), false, 0).unwrap(), 12000);
    assert!(matches!(
        n.compose("#general".into(), "Alice".into(), 1, "full".into(), 0, 0),
        Err(ChannelError::Limited)
    ));
    assert_eq!(n.wait_ms("#general".into(), false, 11999).unwrap(), 1);
    chat(&n, "#general", "refilled", 12000);
    assert!(n.wait_ms("#general".into(), false, 1).is_err());
}
#[test]
fn safe_native_presentation_persists_in_local_arrival_order() {
    let (db, store, a) = node(112);
    let (_, _, b) = node(113);
    let raw = b
        .compose(
            "#general".into(),
            "✓ Official Alice".into(),
            0x23,
            "<script>https://example.org</script>".into(),
            u32::MAX,
            0,
        )
        .unwrap();
    assert!(accept(&a, &store, raw.clone(), false, 0));
    assert!(!accept(&a, &store, raw, false, 0));
    accept(
        &a,
        &store,
        chat(&b, "#general", "newer local arrival", 1000),
        false,
        1000,
    );
    let reopened = EncryptedStore::open(Box::new(db), vec![112; 16], false, Some(200001)).unwrap();
    let messages = a
        .history(reopened, "#general".into(), "Alice".into())
        .unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].nickname, "Alice");
    assert!(messages[0].confusable);
    assert_eq!(messages[0].text, "<script>https://example.org</script>");
    assert_eq!(messages[1].text, "newer local arrival");
    assert_eq!(messages[0].claimed_timestamp, u32::MAX);
    let raw = chat(&b, "#general", "wall clock moved back", 2000);
    a.accept(
        store.clone(),
        ChannelReceipt {
            link: None,
            bytes: raw,
            intake: TransportIntake::Unverified,
            own: false,
            wall: 1,
            now: 2000,
        },
    )
    .unwrap();
    assert_eq!(
        a.history(store, "#general".into(), "Alice".into())
            .unwrap()
            .last()
            .unwrap()
            .text,
        "wall clock moved back"
    );
    let (_, foreign, _) = node(114);
    assert!(
        a.history(foreign, "#general".into(), "Alice".into())
            .is_err()
    );
}
#[test]
fn reaction_replacement_removal_and_orphan_expiry_survive_restart() {
    let (db, store, a) = node(115);
    let (_, _, b) = node(116);
    let raw = chat(&b, "#general", "target", 0);
    let id = raw[4..12].to_vec();
    let reaction = a
        .reaction("#general".into(), id.clone(), 1, false, 0)
        .unwrap();
    assert!(!accept(&a, &store, reaction, false, 0));
    accept(&a, &store, raw, false, 1000);
    let view = a
        .history(store.clone(), "#general".into(), "Alice".into())
        .unwrap();
    assert_eq!(view[0].reactions[1], 1);
    assert_eq!(view[0].own_reaction, Some(1));
    accept(
        &a,
        &store,
        a.reaction("#general".into(), id.clone(), 3, false, 2000)
            .unwrap(),
        true,
        2000,
    );
    let reopened = EncryptedStore::open(Box::new(db), vec![115; 16], false, Some(200002)).unwrap();
    let view = a
        .history(reopened, "#general".into(), "Alice".into())
        .unwrap();
    assert_eq!(view[0].reactions[1], 0);
    assert_eq!(view[0].reactions[3], 1);
    accept(
        &a,
        &store,
        a.reaction("#general".into(), id, 3, true, 3000).unwrap(),
        true,
        3000,
    );
    assert_eq!(
        a.history(store.clone(), "#general".into(), "Alice".into())
            .unwrap()[0]
            .reactions,
        vec![0; 9]
    );
    let late = chat(&b, "#general", "late target", 3000);
    let orphan = a
        .reaction("#general".into(), late[4..12].to_vec(), 2, false, 3000)
        .unwrap();
    assert!(!accept(&a, &store, orphan, false, 3000));
    accept(&a, &store, late, false, 123000);
    assert_eq!(
        a.history(store, "#general".into(), "Alice".into())
            .unwrap()
            .last()
            .unwrap()
            .reactions,
        vec![0; 9]
    );
}
#[test]
fn intake_classification_never_displays_ciphertext_or_sync() {
    let (_, store, n) = node(117);
    let raw = chat(&n, "#general", "held", 0);
    for intake in [
        TransportIntake::Opaque,
        TransportIntake::Duplicate,
        TransportIntake::DeferredSync,
        TransportIntake::Pending,
    ] {
        assert!(
            !n.accept(
                store.clone(),
                ChannelReceipt {
                    link: None,
                    bytes: raw.clone(),
                    intake,
                    own: false,
                    wall: 200000,
                    now: 0
                }
            )
            .unwrap()
        );
    }
    assert!(
        n.history(store.clone(), "#general".into(), "Alice".into())
            .unwrap()
            .is_empty()
    );
    let signed_hex = include_str!("../../vectors/crypto/friend-v1.tsv")
        .lines()
        .find(|line| line.starts_with("peer_chat\t"))
        .unwrap()
        .split_once('\t')
        .unwrap()
        .1;
    let mut signed: Vec<u8> = signed_hex
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect();
    *signed.last_mut().unwrap() ^= 1;
    assert!(
        !n.accept(
            store.clone(),
            ChannelReceipt {
                link: None,
                bytes: signed.clone(),
                intake: TransportIntake::Pending,
                own: false,
                wall: 200000,
                now: 0
            }
        )
        .unwrap()
    );
    assert!(
        !n.accept(
            store.clone(),
            ChannelReceipt {
                link: None,
                bytes: signed,
                intake: TransportIntake::Unverified,
                own: false,
                wall: 200000,
                now: 0
            }
        )
        .unwrap()
    );
    assert!(
        n.history(store, "#general".into(), "Alice".into())
            .unwrap()
            .is_empty()
    );
    let announce = n.announce("Alice".into(), 1, 2, 200000).unwrap();
    assert!(matches!(
        codec::parse(&announce, codec::Context::Live)
            .unwrap()
            .payload(),
        codec::Payload::Announce { .. }
    ));
}

#[test]
fn orphan_reactions_are_bounded_per_link_per_node_and_removed_on_disconnect() {
    for (count, spread, disconnect) in [(33u64, false, false), (257, true, false), (1, false, true)]
    {
        let (_, store, n) = node(119);
        let raw = chat(&n, "#general", "target", 0);
        let template = n
            .reaction("#general".into(), raw[4..12].to_vec(), 1, false, 0)
            .unwrap();
        for i in 0..count {
            let mut bytes = template.clone();
            bytes[4..12].copy_from_slice(&i.to_be_bytes());
            // Each orphan targets a different message; the oldest targets raw.
            if i != 0 {
                bytes[26..34].copy_from_slice(&i.to_be_bytes());
            }
            assert!(
                !n.accept(
                    store.clone(),
                    ChannelReceipt {
                        link: Some(LinkHandle {
                            instance_nonce: 1,
                            generation: if spread { i / 32 + 1 } else { 1 }
                        }),
                        bytes,
                        intake: TransportIntake::Unverified,
                        own: false,
                        wall: 200000,
                        now: 0,
                    }
                )
                .unwrap()
            );
        }
        if disconnect {
            n.disconnected(
                LinkHandle {
                    instance_nonce: 1,
                    generation: 1,
                },
                0,
            )
            .unwrap();
        }
        accept(&n, &store, raw, false, 1);
        assert_eq!(
            n.history(store, "#general".into(), "Alice".into()).unwrap()[0].reactions,
            vec![0; 9]
        );
    }
}

#[test]
fn storage_failure_does_not_claim_persistence_or_prevent_retry() {
    let (db, store, n) = node(118);
    assert!(db.path.is_file());
    let bytes = chat(&n, "#general", "retry", 0);
    let receipt = ChannelReceipt {
        link: None,
        bytes,
        intake: TransportIntake::Unverified,
        own: true,
        wall: 200000,
        now: 0,
    };
    db.fail(Some("INSERT INTO history"));
    assert!(n.accept(store.clone(), receipt.clone()).is_err());
    db.fail(None);
    assert!(n.accept(store, receipt).unwrap());
}

fn radio(seed: u8, store: Arc<EncryptedStore>) -> NativeTransport {
    let public = IdentityKeySession::import_unlocked(vec![seed; 64], vec![seed; 16])
        .unwrap()
        .public_identity()
        .unwrap();
    NativeTransport::new(store, public, u64::from(seed), 0).unwrap()
}
fn connect(a: &NativeTransport, b: &NativeTransport, address: u8) -> (LinkHandle, LinkHandle) {
    let al = a
        .native_ready(
            a.admit_connection(vec![address; 16], 0).unwrap(),
            TransportRole::Central,
            512,
            512,
            0,
        )
        .unwrap()
        .link;
    let bl = b
        .native_ready(
            b.admit_connection(vec![address; 16], 0).unwrap(),
            TransportRole::Peripheral,
            182,
            512,
            0,
        )
        .unwrap()
        .link;
    let af = a.tick(0).unwrap().sends.remove(0);
    let bf = b.tick(0).unwrap().sends.remove(0);
    a.receive(al.clone(), bf.bytes.len() as u64, bf.bytes, 0)
        .unwrap();
    b.receive(bl.clone(), af.bytes.len() as u64, af.bytes, 0)
        .unwrap();
    a.complete(al.clone(), af.token, true, 0).unwrap();
    b.complete(bl.clone(), bf.token, true, 0).unwrap();
    (al, bl)
}
fn transfer(
    a: &NativeTransport,
    b: &NativeTransport,
    target: &LinkHandle,
    now: u64,
) -> (Vec<u8>, TransportIntake) {
    let mut received = None;
    for frame in a.tick(now).unwrap().sends {
        for event in b
            .receive(target.clone(), frame.bytes.len() as u64, frame.bytes, now)
            .unwrap()
            .events
        {
            if let TransportEvent::Received { bytes, intake, .. } = event {
                received = Some((bytes, intake));
            }
        }
        a.complete(frame.link, frame.token, true, now).unwrap();
    }
    received.expect("real transport intake")
}
#[test]
fn three_node_channel_chat_and_reaction_relay_use_real_core_and_persist() {
    let (_, sa, ca) = node(121);
    let (_, sb, _) = node(122);
    let (db, sc, cc) = node(123);
    let a = radio(121, sa);
    let b = radio(122, sb);
    let c = radio(123, sc.clone());
    let (ab, ba) = connect(&a, &b, 1);
    let (bc, cb) = connect(&b, &c, 2);
    let raw = chat(&ca, "melodic|techno|valley", "Across the mesh", 1000);
    let id = raw[4..12].to_vec();
    a.enqueue(ab.clone(), raw, TransportTraffic::Own, 9, 1000)
        .unwrap();
    let (middle, _) = transfer(&a, &b, &ba, 1000);
    // The middle node need not subscribe or store this channel to relay it.
    b.enqueue(bc.clone(), middle, TransportTraffic::Forwarded, 10, 1000)
        .unwrap();
    let (raw, intake) = transfer(&b, &c, &cb, 2000);
    assert_eq!(raw[3], 6);
    assert!(
        cc.accept(
            sc.clone(),
            ChannelReceipt {
                link: Some(cb.clone()),
                bytes: raw,
                intake,
                own: false,
                wall: 200002,
                now: 2000
            }
        )
        .unwrap()
    );
    let reaction = cc
        .reaction("melodic|techno|valley".into(), id, 3, false, 3000)
        .unwrap();
    c.enqueue(
        cb.clone(),
        reaction.clone(),
        TransportTraffic::Own,
        11,
        3000,
    )
    .unwrap();
    let (middle, _) = transfer(&c, &b, &bc, 3000);
    b.enqueue(ba, middle, TransportTraffic::Forwarded, 12, 3000)
        .unwrap();
    let (reaction_back, _) = transfer(&b, &a, &ab, 4000);
    assert_eq!(reaction_back[3], 6);
    accept(&cc, &sc, reaction, true, 4000);
    let reopened = EncryptedStore::open(Box::new(db), vec![123; 16], false, Some(200004)).unwrap();
    let view = cc
        .history(reopened, "melodic|techno|valley".into(), "Alice".into())
        .unwrap();
    assert_eq!(view[0].text, "Across the mesh");
    assert_eq!(view[0].reactions[3], 1);
}
