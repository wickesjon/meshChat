#[path = "../friends/database.rs"]
#[allow(dead_code)]
mod database;
use meshchat_core::{
    codec::{self, Context, Payload},
    identity::IdentityKeySession,
    native_channels::{ChannelReceipt, NativeChannels},
    native_transport::TransportIntake,
    storage::EncryptedStore,
};

fn node() -> NativeChannels {
    let identity = IdentityKeySession::import_unlocked(vec![1; 64], vec![1; 16])
        .unwrap()
        .public_identity()
        .unwrap();
    NativeChannels::new(identity, 0).unwrap()
}

#[test]
fn existing_fixed_fields_carry_only_cosmetics_and_can_be_cleared() {
    let n = node();
    for (supporter, rgb, expected) in [
        (true, Some(0x12abef), [3, 0x12, 0xab, 0xef]),
        (true, None, [2, 0, 0, 0]),
        (false, None, [0, 0, 0, 0]),
    ] {
        n.set_cosmetics(supporter, rgb).unwrap();
        let chat = n
            .compose("#general".into(), "Alice".into(), 1, "test".into(), 2, 0)
            .unwrap();
        let announce = n.announce("Alice".into(), 1, 0, 2).unwrap();
        for bytes in [chat, announce] {
            let packet = codec::parse(&bytes, Context::Live).unwrap();
            assert_eq!(packet.header().flags, 0);
            let (Payload::Chat { cosmetics, .. } | Payload::Announce { cosmetics, .. }) =
                packet.payload()
            else {
                panic!()
            };
            assert_eq!(cosmetics, expected);
        }
    }
    assert!(n.set_cosmetics(true, Some(0x1000000)).is_err());
}

#[test]
fn anonymous_output_and_hostile_anonymous_input_hide_cosmetics() {
    let n = node();
    n.set_cosmetics(true, Some(0x12abef)).unwrap();
    let mut bytes = n
        .compose(
            "#confessions".into(),
            "Alice".into(),
            0x23,
            "test".into(),
            2,
            0,
        )
        .unwrap();
    if let Payload::Chat {
        avatar, cosmetics, ..
    } = codec::parse(&bytes, Context::Live).unwrap().payload()
    {
        assert_eq!(avatar, 0);
        assert_eq!(cosmetics, [0; 4]);
    } else {
        panic!()
    }
    bytes[30] = 0x23;
    let at = 32 + usize::from(bytes[31]);
    bytes[at..at + 4].copy_from_slice(&[3, 0x12, 0xab, 0xef]);
    let db = database::Database::new();
    let store = EncryptedStore::open(Box::new(db), vec![1; 16], true, Some(200000)).unwrap();
    assert!(
        n.accept(
            store.clone(),
            ChannelReceipt {
                link: None,
                bytes,
                intake: TransportIntake::Unverified,
                own: false,
                wall: 200000,
                now: 0
            }
        )
        .unwrap()
    );
    let rows = n
        .history(store, "#confessions".into(), "Alice".into())
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].avatar, 0);
    assert!(!rows[0].supporter_hint);
    assert_eq!(rows[0].nickname_rgb, None);
}

#[test]
fn remote_hint_does_not_grant_trust_and_absent_color_ignores_rgb() {
    let n = node();
    n.set_cosmetics(true, None).unwrap();
    let mut bytes = n
        .compose("#general".into(), "Alice".into(), 1, "test".into(), 2, 0)
        .unwrap();
    let at = 32 + usize::from(bytes[31]);
    bytes[at..at + 4].copy_from_slice(&[0xfe, 0xff, 0xff, 0xff]);
    let db = database::Database::new();
    let store = EncryptedStore::open(Box::new(db), vec![1; 16], true, Some(200000)).unwrap();
    n.accept(
        store.clone(),
        ChannelReceipt {
            link: None,
            bytes,
            intake: TransportIntake::Unverified,
            own: false,
            wall: 200000,
            now: 0,
        },
    )
    .unwrap();
    let row = n
        .history(store, "#general".into(), "Bob".into())
        .unwrap()
        .remove(0);
    assert!(row.supporter_hint);
    assert_eq!(row.nickname_rgb, None);
    assert!(!row.signed);
    assert_eq!(row.verified_petname, None);
}
