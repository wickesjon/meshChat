//! Storage-boundary reproduction for MC-029. The SQLite double executes the
//! production SQL; it is not SQLCipher or cryptographic-authentication evidence.
#[allow(dead_code)]
#[path = "../friends/database.rs"]
mod database;

use meshchat_core::storage::{AcceptResult, EncryptedStore, HistoryItem, SqlDatabase, SqlValue};

fn newest_arrival_is_retained(count: u64) {
    let db = database::Database::new();
    let store =
        EncryptedStore::open(Box::new(db.clone()), vec![29; 16], true, Some(200_000)).unwrap();
    let peer = vec![30; 64];
    let mut subject = vec![2];
    subject.extend_from_slice(&peer);
    for id in 0..count {
        // The protected acceptance owner supplies already-authenticated content.
        // Every timestamp is inside the allowed window, but each newer local
        // arrival claims an older sender time. No clock value is taken from it.
        let item = HistoryItem {
            conversation: peer.clone(),
            direct: true,
            direction: 0,
            logical_type: 1,
            message_id: id.to_be_bytes().to_vec(),
            timestamp: 200_000 - i64::try_from(id).unwrap(),
            body: b"synthetic already-authenticated history fixture".to_vec(),
            provenance: subject.clone(),
        };
        assert_eq!(
            store
                .accept_authenticated(
                    item,
                    subject.clone(),
                    id.to_be_bytes().to_vec(),
                    Some(200_000)
                )
                .unwrap(),
            AcceptResult::Accepted
        );
    }
    let last = (count - 1).to_be_bytes().to_vec();
    let persisted = db
        .query(
            "SELECT count(*) FROM history WHERE direct=1 AND conversation=? AND message_id=?"
                .into(),
            vec![
                SqlValue::Bytes {
                    value: peer.clone(),
                },
                SqlValue::Bytes {
                    value: last.clone(),
                },
            ],
            1,
        )
        .unwrap();
    assert!(
        matches!(persisted[0].cells[0], SqlValue::Integer { value: 1 }),
        "the conversation cap discarded a newly accepted message based on its sender timestamp"
    );
    let rows = store.history(peer, true, 100).unwrap();
    assert!(
        rows.iter().any(|row| row.message_id == last),
        "the history page omitted the newest local arrival before the UI could sort it"
    );
    assert_eq!(
        rows.first().unwrap().message_id,
        (count - 1).to_be_bytes(),
        "newest-first history must use local acceptance order, not the sender timestamp"
    );
}

#[test]
fn accepted_messages_keep_local_arrival_order() {
    newest_arrival_is_retained(2);
}

#[test]
fn history_limit_does_not_hide_the_newest_arrival() {
    newest_arrival_is_retained(101);
}

#[test]
fn conversation_cap_does_not_discard_the_newest_arrival() {
    newest_arrival_is_retained(1001);
}
