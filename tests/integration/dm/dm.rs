#[path = "../friends/database.rs"]
mod database;
use database::Database;
use meshchat_core::{
    LinkHandle, codec,
    dm::*,
    friends::{Friends, Role, SendToken},
    identity::{IdentityKeySession, PublicIdentity},
    ingress::Ingress,
    links::UnconfirmedProposal,
    storage::{AcceptResult, EncryptedStore, SqlDatabase, SqlValue},
};
use std::sync::Arc;
const WALL: i64 = 200_000;
fn identity(n: u8) -> Arc<IdentityKeySession> {
    IdentityKeySession::import_unlocked(vec![n; 64], vec![n; 16]).unwrap()
}
fn proposal(public: &PublicIdentity) -> UnconfirmedProposal {
    let mut bundle = [1; 65];
    bundle[1..33].copy_from_slice(&public.signing_key);
    bundle[33..].copy_from_slice(&public.agreement_key);
    UnconfirmedProposal::Friend {
        bundle,
        nickname: "Claim".into(),
    }
}
struct Node {
    db: Database,
    store: Arc<EncryptedStore>,
    key: Arc<IdentityKeySession>,
    friends: Friends,
    dm: Dms,
    ingress: Ingress,
    link: LinkHandle,
    pin: SendToken,
    peer: [u8; 64],
}
impl Node {
    fn new(n: u8, other: u8) -> Self {
        Self::new_at(n, other, WALL)
    }
    fn new_at(n: u8, other: u8, wall: i64) -> Self {
        let key = identity(n);
        let own = key.public_identity().unwrap();
        let remote = identity(other).public_identity().unwrap();
        let db = Database::new();
        let store = EncryptedStore::open(
            Box::new(db.clone()),
            own.generation.clone(),
            true,
            Some(wall),
        )
        .unwrap();
        let mut friends = Friends::open(&store, &own, 0).unwrap();
        let pin = friends
            .confirm(&store, &key, proposal(&remote), "Friend", None)
            .unwrap();
        let peer = friends.validate_send(&store, &pin).unwrap();
        let mut ingress = Ingress::new(u64::from(n), 8, 0).unwrap();
        let link = LinkHandle {
            instance_nonce: u64::from(n),
            generation: 1,
        };
        let local = friends
            .start_link(
                &mut ingress,
                link.clone(),
                Role::Central,
                (512, 512),
                0,
                || Ok([n; 16]),
            )
            .unwrap();
        friends.hello_transmitted(&link, 0).unwrap();
        let mut hello = local;
        hello[1] = 1;
        hello[6..22].fill(other);
        hello[22..].copy_from_slice(&remote.signing_key);
        let mut frame = vec![2, 0, 0, 55, 2];
        frame.extend_from_slice(&hello);
        friends
            .receive(&mut ingress, &link, frame.len() as u64, &frame, 0)
            .unwrap();
        Self {
            db,
            store,
            key,
            friends,
            dm: Dms::new(&own).unwrap(),
            ingress,
            link,
            pin,
            peer,
        }
    }
    fn send(&mut self, id: u64, content: Content, now: u64, wall: i64) -> Vec<u8> {
        self.dm
            .send(
                &mut self.ingress,
                &mut self.friends,
                &self.store,
                &self.key,
                &self.link,
                &self.pin,
                id.to_be_bytes(),
                content,
                now,
                Some(wall),
            )
            .unwrap()
            .unwrap()
            .bytes()
            .to_vec()
    }
    fn admit(&mut self, raw: &[u8], now: u64) -> Option<Job> {
        let mut frame = vec![0, 0];
        frame.extend_from_slice(&(raw.len() as u16).to_be_bytes());
        frame.extend_from_slice(raw);
        self.friends
            .receive(
                &mut self.ingress,
                &self.link,
                frame.len() as u64,
                &frame,
                now,
            )
            .unwrap();
        self.retry(raw, now)
    }
    fn retry(&mut self, raw: &[u8], now: u64) -> Option<Job> {
        let pos = (0..32).find(|p| self.ingress.pending_bytes(*p) == Some(raw))?;
        self.dm
            .prepare(
                &mut self.ingress,
                &mut self.friends,
                &self.store,
                &self.link,
                pos,
                now,
            )
            .unwrap()
    }
    fn finish(&mut self, job: Job, now: u64, wall: Option<i64>) -> Result<Received, Error> {
        self.dm.complete(
            &mut self.ingress,
            &mut self.friends,
            &self.store,
            &self.key,
            job,
            now,
            wall,
        )
    }
    fn receive(&mut self, raw: &[u8], now: u64) -> Received {
        let job = self.admit(raw, now).unwrap();
        self.finish(job, now, Some(WALL)).unwrap()
    }
    fn reactions(&mut self, target: u64, now: u64) -> Vec<(u8, u8)> {
        self.dm
            .reactions(
                &self.store,
                &mut self.friends,
                &self.pin,
                target.to_be_bytes(),
                now,
                Some(WALL),
            )
            .unwrap()
    }
}
#[test]
fn production_roundtrip_reaction_replacement_removal_and_restart() {
    let mut a = Node::new(2, 3);
    let mut b = Node::new(3, 2);
    let raw = a.send(1, Content::Chat("secret synthetic".into()), 1, WALL);
    assert!(!raw.windows(16).any(|v| v == b"secret synthetic"));
    let received = b.receive(&raw, 1);
    assert_eq!(
        received.content,
        Some(Content::Chat("secret synthetic".into()))
    );
    let reply = b.send(2, Content::Chat("reply".into()), 1000, WALL);
    assert!(a.receive(&reply, 1000).content.is_some());
    let reaction = a.send(
        3,
        Content::Reaction {
            target: 2u64.to_be_bytes(),
            remove: false,
            code: 5,
        },
        2000,
        WALL,
    );
    b.receive(&reaction, 2000);
    assert_eq!(b.reactions(2, 2001), vec![(0, 5)]);
    let reaction = a.send(
        4,
        Content::Reaction {
            target: 2u64.to_be_bytes(),
            remove: false,
            code: 6,
        },
        3000,
        WALL,
    );
    b.receive(&reaction, 3000);
    assert_eq!(b.reactions(2, 3001), vec![(0, 6)]);
    b.dm = Dms::new(&b.key.public_identity().unwrap()).unwrap();
    assert_eq!(b.reactions(2, 3002), vec![(0, 6)]);
    let reaction = a.send(
        5,
        Content::Reaction {
            target: 2u64.to_be_bytes(),
            remove: true,
            code: 6,
        },
        4000,
        WALL,
    );
    b.receive(&reaction, 4000);
    assert!(b.reactions(2, 4001).is_empty());
    assert_eq!(
        b.store.history(b.peer.to_vec(), true, 100).unwrap().len(),
        5
    );
}
#[test]
fn invalid_first_valid_second_and_persistent_replay_after_history_deletion() {
    let mut a = Node::new(2, 3);
    let mut b = Node::new(3, 2);
    let raw = a.send(1, Content::Chat("secret".into()), 1, WALL);
    let mut bad = raw.clone();
    *bad.last_mut().unwrap() ^= 1;
    let job = b.admit(&bad, 1).unwrap();
    assert!(matches!(
        b.finish(job, 1, Some(WALL)),
        Err(Error::Authentication)
    ));
    let good = b.receive(&raw, 2);
    assert_eq!(good.result, AcceptResult::Accepted);
    assert!(b.admit(&raw, 3).is_none());
    b.store.delete_history(b.peer.to_vec(), true).unwrap();
    let replay = b.receive(&raw, 900_004);
    assert_eq!(replay.result, AcceptResult::Replay);
    assert!(replay.content.is_none());
    assert!(
        b.store
            .history(b.peer.to_vec(), true, 100)
            .unwrap()
            .is_empty()
    );
}
#[test]
fn orphan_applies_once_within_deadline_and_never_revives_after_restart() {
    let mut a = Node::new(2, 3);
    let mut b = Node::new(3, 2);
    let reaction = a.send(
        1,
        Content::Reaction {
            target: 9u64.to_be_bytes(),
            remove: false,
            code: 7,
        },
        1,
        WALL,
    );
    assert!(b.receive(&reaction, 1).content.is_none());
    assert_eq!(b.dm.orphan_count(), 1);
    let chat = a.send(9, Content::Chat("target".into()), 1000, WALL);
    b.receive(&chat, 1000);
    assert_eq!(b.reactions(9, 1001), vec![(0, 7)]);
    assert_eq!(b.dm.orphan_count(), 0);
    let reaction = a.send(
        2,
        Content::Reaction {
            target: 10u64.to_be_bytes(),
            remove: false,
            code: 2,
        },
        2000,
        WALL,
    );
    b.receive(&reaction, 2000);
    let chat = a.send(10, Content::Chat("late".into()), 3000, WALL);
    b.receive(&chat, 122_000);
    assert!(b.reactions(10, 122_001).is_empty());
    let reaction = a.send(
        3,
        Content::Reaction {
            target: 11u64.to_be_bytes(),
            remove: false,
            code: 3,
        },
        123_000,
        WALL,
    );
    b.receive(&reaction, 123_000);
    b.dm = Dms::new(&b.key.public_identity().unwrap()).unwrap();
    let chat = a.send(11, Content::Chat("restart".into()), 124_000, WALL);
    b.receive(&chat, 124_000);
    assert!(b.reactions(11, 124_001).is_empty());
}
#[test]
fn epoch_skew_unknown_peer_and_changed_pin_do_not_downgrade() {
    let mut a = Node::new_at(2, 3, WALL - 7200);
    let mut b = Node::new(3, 2);
    let old = a.send(2, Content::Chat("outside tag".into()), 0, WALL - 7200);
    let raw = a.send(1, Content::Chat("one hour".into()), 1, WALL - 3600);
    assert!(b.receive(&raw, 1).content.is_some());
    let job = b.admit(&old, 1000).unwrap();
    assert!(matches!(b.finish(job, 1000, Some(WALL)), Err(Error::Tag)));
    let raw = a.send(3, Content::Chat("pending change".into()), 2000, WALL);
    let job = b.admit(&raw, 2000).unwrap();
    b.friends.begin_replacement(&b.store, &b.pin).unwrap();
    assert!(b.finish(job, 2000, Some(WALL)).is_err());
    let mut stranger = Node::new(4, 3);
    assert!(stranger.admit(&raw, 1).is_none());
    assert_eq!(
        unique_tuple([[1; 64], [2; 64]].into_iter())
            .unwrap_err()
            .to_string(),
        "ambiguous friend hints"
    );
}
#[test]
fn store_failure_clock_and_two_concurrent_jobs_recover() {
    let mut a = Node::new(2, 3);
    let mut b = Node::new(3, 2);
    let one = a.send(1, Content::Chat("one".into()), 1, WALL);
    let two = a.send(2, Content::Chat("two".into()), 2, WALL);
    let j1 = b.admit(&one, 1).unwrap();
    let j2 = b.admit(&two, 2).unwrap();
    assert!(b.admit(&one, 3).is_none());
    b.db.fail(Some("INSERT INTO history"));
    assert!(b.finish(j1, 3, Some(WALL)).is_err());
    b.db.fail(None);
    assert!(b.finish(j2, 4, None).is_err());
    let j1 = b.retry(&one, 1000).unwrap();
    assert!(b.finish(j1, 1000, Some(WALL)).unwrap().content.is_some());
    let j2 = b.retry(&two, 2000).unwrap();
    assert!(b.finish(j2, 2000, Some(WALL)).unwrap().content.is_some());
}

#[test]
fn independent_vectors_padding_boundaries_malformed_plaintext_and_forgery() {
    let vectors: Vec<_> = include_str!("../../vectors/crypto/dm-v1.tsv")
        .lines()
        .map(|l| {
            let (k, v) = l.split_once('\t').unwrap();
            (k, database::unhex(v))
        })
        .collect();
    for (name, raw) in vectors {
        let mut receiver = if name.starts_with("chat_3_") {
            Node::new(2, 3)
        } else {
            Node::new(3, 2)
        };
        let packet = codec::parse(&raw, codec::Context::Live).unwrap();
        assert_eq!(packet.header().flags, 1);
        let job = receiver.admit(&raw, 1).unwrap();
        let result = receiver.finish(job, 1, Some(WALL));
        if name.starts_with("chat_") {
            let expected = name.rsplit('_').next().unwrap().parse::<usize>().unwrap();
            assert_eq!(
                result.unwrap().content,
                Some(Content::Chat("a".repeat(expected)))
            );
        } else if name == "reaction" {
            assert!(result.unwrap().content.is_none());
            assert_eq!(receiver.dm.orphan_count(), 1);
        } else {
            assert!(result.is_err(), "{name}");
            assert!(
                receiver
                    .store
                    .history(receiver.peer.to_vec(), true, 100)
                    .unwrap()
                    .is_empty()
            );
        }
    }
}
#[test]
fn every_immutable_header_byte_envelope_and_ciphertext_is_bound() {
    let raw = database::unhex(
        include_str!("../../vectors/crypto/dm-v1.tsv")
            .lines()
            .next()
            .unwrap()
            .split_once('\t')
            .unwrap()
            .1,
    );
    let mut b = Node::new(3, 2);
    let indices = (0..raw.len()).filter(|i| *i != 3);
    for (n, index) in indices.enumerate() {
        let mut changed = raw.clone();
        changed[index] ^= 1;
        let now = (n as u64 + 1) * 1000;
        if let Some(job) = b.admit(&changed, now) {
            assert!(b.finish(job, now, Some(WALL)).is_err(), "byte {index}");
        }
    }
    assert!(
        b.store
            .history(b.peer.to_vec(), true, 100)
            .unwrap()
            .is_empty()
    );
    let mut ttl = raw;
    ttl[3] = 1;
    assert!(b.receive(&ttl, 200_000).content.is_some());
}
#[test]
fn restart_keeps_reactions_and_full_ledger_refuses_acceptance() {
    let mut a = Node::new(2, 3);
    let mut b = Node::new(3, 2);
    let raw = a.send(1, Content::Chat("target".into()), 1, WALL);
    b.receive(&raw, 1);
    let reaction = a.send(
        2,
        Content::Reaction {
            target: 1u64.to_be_bytes(),
            remove: false,
            code: 4,
        },
        1000,
        WALL,
    );
    b.receive(&reaction, 1000);
    let path = b.db.path.clone();
    let public = b.key.public_identity().unwrap();
    let pin = b.pin.clone();
    drop(b);
    let db = Database::reopen(path);
    let store = EncryptedStore::open(
        Box::new(db.clone()),
        public.generation.clone(),
        false,
        Some(WALL),
    )
    .unwrap();
    let mut friends = Friends::open(&store, &public, 0).unwrap();
    let mut dm = Dms::new(&public).unwrap();
    assert_eq!(
        dm.reactions(
            &store,
            &mut friends,
            &pin,
            1u64.to_be_bytes(),
            0,
            Some(WALL)
        )
        .unwrap(),
        vec![(0, 4)]
    );
    let mut b = Node::new(3, 2);
    b.db.execute("WITH RECURSIVE n(x) AS (VALUES(0) UNION ALL SELECT x+1 FROM n WHERE x<99999) INSERT INTO ledger(subject,direction,logical_type,message_id,digest,timestamp) SELECT ?,0,1,CAST(printf('%08d',x) AS BLOB),?,? FROM n".into(),vec![SqlValue::Bytes{value:vec![2;65]},SqlValue::Bytes{value:vec![2;32]},SqlValue::Integer{value:WALL}]).unwrap();
    let job = b.admit(&raw, 1).unwrap();
    assert!(matches!(
        b.finish(job, 1, Some(WALL)),
        Err(Error::Storage(
            meshchat_core::storage::StorageError::Capacity
        ))
    ));
    assert!(
        b.store
            .history(b.peer.to_vec(), true, 100)
            .unwrap()
            .is_empty()
    );
}
#[test]
fn budget_recovery_pending_eviction_and_orphan_caps() {
    let mut a = Node::new(2, 3);
    let mut b = Node::new(3, 2);
    let raw = a.send(1, Content::Chat("budget".into()), 1, WALL);
    let permit = b.ingress.begin_work(&b.link, 20, 1).unwrap().unwrap();
    b.ingress.finish_work(permit).unwrap();
    assert!(b.admit(&raw, 1).is_none());
    let job = b.retry(&raw, 1000).unwrap();
    assert_eq!(
        b.finish(job, 1000, Some(WALL)).unwrap().result,
        AcceptResult::Accepted
    );
    let held = b.ingress.begin_work(&b.link, 11, 1000).unwrap().unwrap();
    b.ingress.finish_work(held).unwrap();
    // Different public IDs still cannot replenish the shared work bucket.
    for n in 2..12 {
        let mut missing = raw.clone();
        missing[11] = n;
        missing[12] ^= 32;
        b.admit(&missing, 1000);
    }
    assert!(
        (0..32)
            .filter(|p| b.ingress.pending_bytes(*p).is_some())
            .count()
            <= 8
    );
    for n in 20..55 {
        let raw = a.send(
            n,
            Content::Reaction {
                target: (n + 100).to_be_bytes(),
                remove: false,
                code: 1,
            },
            n * 1000,
            WALL,
        );
        b.receive(&raw, n * 1000);
        assert!(b.dm.orphan_count() <= 32);
    }
    assert_eq!(b.dm.orphan_count(), 32);
    assert!(b.dm.reserved_bytes() < 128 * 1024);
}
#[test]
fn reaction_storage_failure_is_atomic_and_ambiguous_target_never_applies() {
    let mut a = Node::new(2, 3);
    let mut b = Node::new(3, 2);
    let raw = a.send(1, Content::Chat("target".into()), 1, WALL);
    b.receive(&raw, 1);
    let reaction = a.send(
        2,
        Content::Reaction {
            target: 1u64.to_be_bytes(),
            remove: false,
            code: 4,
        },
        1000,
        WALL,
    );
    b.db.fail(Some("INSERT INTO records(kind,key,value) VALUES(6"));
    let job = b.admit(&reaction, 1000).unwrap();
    assert!(b.finish(job, 1000, Some(WALL)).is_err());
    b.db.fail(None);
    assert!(b.reactions(1, 1001).is_empty());
    let job = b.retry(&reaction, 2000).unwrap();
    assert_eq!(
        b.finish(job, 2000, Some(WALL)).unwrap().result,
        AcceptResult::Accepted
    );
    assert_eq!(b.reactions(1, 2001), vec![(0, 4)]);
    // Both directions reused the target ID: no selection by direction/arrival.
    b.send(1, Content::Chat("other meaning".into()), 3000, WALL);
    assert!(b.reactions(1, 3001).is_empty());
    b.store.delete_history(b.peer.to_vec(), true).unwrap();
    assert!(b.reactions(1, 3002).is_empty());
}

#[allow(dead_code)]
#[path = "../sync/admission.rs"]
mod sync_admission;
#[test]
fn deterministic_three_node_opaque_relay_and_ordered_sync_deliver_real_dm() {
    use meshchat_core::{
        framing::Encoder,
        power::Mode,
        sync::{
            Cache, CacheState,
            session::{Identity, Request, Sessions, Sink},
        },
    };
    let mut a = Node::new(2, 3);
    let mut relay = Node::new(4, 3);
    let mut b = Node::new(3, 2);
    let raw = a.send(1, Content::Chat("a".repeat(280)), 1, WALL);
    let mut forwarded = vec![0; raw.len()];
    codec::parse(&raw, codec::Context::Live)
        .unwrap()
        .forward_to(&mut forwarded)
        .unwrap()
        .unwrap();
    let encoder = Encoder::logical(&forwarded, 146, 1).unwrap();
    assert_eq!(encoder.frame_count(), 4);
    for index in 0..encoder.frame_count() {
        let mut frame = [0; 146];
        let n = encoder.frame(index, &mut frame).unwrap();
        relay
            .friends
            .receive(
                &mut relay.ingress,
                &relay.link,
                n as u64,
                &frame[..n],
                1000 + index as u64,
            )
            .unwrap();
    }
    assert!(relay.retry(&forwarded, 1004).is_none());
    assert!(
        relay
            .store
            .history(relay.peer.to_vec(), true, 100)
            .unwrap()
            .is_empty()
    );
    let mut cache = Cache::new(Mode::Normal, 1004).unwrap();
    cache
        .insert_admitted(&forwarded, CacheState::Pending, 1004)
        .unwrap();
    let mut requestor = Sessions::new(b.link.instance_nonce, 0).unwrap();
    requestor.register(&b.link, 0).unwrap();
    let mut responder = Sessions::new(relay.link.instance_nonce, 0).unwrap();
    responder.register(&relay.link, 0).unwrap();
    let mut request = [0; 546];
    let requested = requestor
        .request(
            &b.link,
            Request {
                identity: Identity {
                    message_id: [9; 8],
                    sender_id: [3; 8],
                },
                held_count: 0,
                filter: [0; 512],
            },
            &mut b.ingress,
            10_000,
            &mut request,
        )
        .unwrap();
    requestor.request_started(requested.token, 10_000).unwrap();
    let mut out = [0; 1035];
    let token = sync_admission::admission(
        &mut relay.ingress,
        &relay.link,
        &request[..requested.len],
        false,
        10_000,
        &mut out,
    );
    responder
        .accept_admitted_request(&relay.link, token, &mut cache, 10_000, &mut |_| {})
        .unwrap();
    let mut served = [0; 1035];
    let item = responder
        .next_served(&relay.link, &mut cache, 10_001, &mut served, &mut |_| {})
        .unwrap()
        .unwrap();
    assert_eq!(item.flags, 0);
    let mut collected = Vec::new();
    let token = sync_admission::admission(
        &mut b.ingress,
        &b.link,
        &served[..item.len],
        true,
        10_002,
        &mut out,
    );
    requestor
        .receive_admitted(
            &b.link,
            token,
            &mut b.ingress,
            10_002,
            &mut Sink {
                events: &mut |_| {},
                messages: &mut |raw, _| collected.push(raw.to_vec()),
            },
        )
        .unwrap();
    assert_eq!(collected, vec![forwarded.clone()]);
    let job = b.retry(&forwarded, 10_003).unwrap();
    assert_eq!(
        b.finish(job, 10_003, Some(WALL)).unwrap().content,
        Some(Content::Chat("a".repeat(280)))
    );
    responder
        .served_complete(item.token, true, 10_003, &mut |_| {})
        .unwrap();
    let marker = responder
        .next_served(&relay.link, &mut cache, 10_004, &mut served, &mut |_| {})
        .unwrap()
        .unwrap();
    assert_eq!(marker.flags, 5);
    let token = sync_admission::admission(
        &mut b.ingress,
        &b.link,
        &served[..marker.len],
        true,
        10_005,
        &mut out,
    );
    assert!(matches!(
        requestor
            .receive_admitted(
                &b.link,
                token,
                &mut b.ingress,
                10_005,
                &mut Sink {
                    events: &mut |_| {},
                    messages: &mut |_, _| panic!("marker is not a message")
                }
            )
            .unwrap(),
        meshchat_core::sync::session::Received::Complete { .. }
    ));
    // REACTIONs use the same opaque multi-hop logical path, never SYNC history.
    let reaction = a.send(
        2,
        Content::Reaction {
            target: 1u64.to_be_bytes(),
            remove: false,
            code: 2,
        },
        20_000,
        WALL,
    );
    assert!(relay.admit(&reaction, 20_000).is_none());
    b.receive(&reaction, 20_000);
    assert_eq!(b.reactions(1, 20_001), vec![(0, 2)]);
}

#[test]
fn active_dependency_graph_stays_within_exact_approved_pairs() {
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../src/core/check_dependency_pairs.py");
    assert!(
        std::process::Command::new(if cfg!(windows) { "python" } else { "python3" })
            .arg("-B")
            .arg(script)
            .status()
            .unwrap()
            .success()
    );
}

#[test]
fn evicted_valid_pending_bytes_and_expired_jobs_can_recover_without_effect() {
    let mut a = Node::new(2, 3);
    let mut b = Node::new(3, 2);
    let raw = a.send(1, Content::Chat("eviction".into()), 1, WALL);
    let held = b.ingress.begin_work(&b.link, 20, 1).unwrap().unwrap();
    b.ingress.finish_work(held).unwrap();
    assert!(b.admit(&raw, 1).is_none());
    for n in 2..12 {
        let mut unknown = raw.clone();
        unknown[11] = n;
        unknown[12] ^= n;
        b.admit(&unknown, 2);
    }
    assert!((0..32).all(|p| b.ingress.pending_bytes(p) != Some(raw.as_slice())));
    assert!(
        b.store
            .history(b.peer.to_vec(), true, 100)
            .unwrap()
            .is_empty()
    );
    assert_eq!(b.receive(&raw, 1000).result, AcceptResult::Accepted);
    let next = a.send(2, Content::Chat("expiry".into()), 2000, WALL);
    let job = b.admit(&next, 2000).unwrap();
    assert!(b.finish(job, 32_000, Some(WALL)).is_err());
    assert_eq!(b.receive(&next, 32_001).result, AcceptResult::Accepted);
}
#[test]
fn held_job_refuses_provider_invalidation_and_own_generation_reset() {
    let mut a = Node::new(2, 3);
    let mut b = Node::new(3, 2);
    let raw = a.send(1, Content::Chat("locked".into()), 1, WALL);
    let job = b.admit(&raw, 1).unwrap();
    b.key.invalidate().unwrap();
    assert!(b.finish(job, 1, Some(WALL)).is_err());
    assert!(
        b.store
            .history(b.peer.to_vec(), true, 100)
            .unwrap()
            .is_empty()
    );
    let mut b = Node::new(3, 2);
    let job = b.admit(&raw, 1).unwrap();
    b.key = identity(5);
    assert!(b.finish(job, 1, Some(WALL)).is_err());
    assert!(
        b.store
            .history(b.peer.to_vec(), true, 100)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn orphan_clock_refusal_does_not_commit_or_consume_recovery() {
    for wall in [None, Some(WALL - 301)] {
        let mut a = Node::new(2, 3);
        let mut b = Node::new(3, 2);
        let reaction = a.send(
            2,
            Content::Reaction {
                target: 1u64.to_be_bytes(),
                remove: false,
                code: 7,
            },
            1,
            WALL,
        );
        b.receive(&reaction, 1);
        let target = a.send(1, Content::Chat("late target".into()), 1000, WALL);
        b.receive(&target, 1000);
        assert_eq!(b.dm.orphan_count(), 1);
        assert!(
            b.dm.reactions(
                &b.store,
                &mut b.friends,
                &b.pin,
                1u64.to_be_bytes(),
                1001,
                wall
            )
            .is_err()
        );
        // Inspect raw committed state: refusal must precede any reaction effect.
        let rows =
            b.db.query(
                "SELECT count(*) FROM records WHERE kind=6".into(),
                vec![],
                1,
            )
            .unwrap();
        assert!(matches!(rows[0].cells[0], SqlValue::Integer { value: 0 }));
        assert_eq!(b.dm.orphan_count(), 1);
        assert_eq!(b.reactions(1, 1002), vec![(0, 7)]);
        assert_eq!(b.dm.orphan_count(), 0);
    }
}

#[derive(Clone)]
struct FailAfterAcceptance {
    db: Database,
    inserted: Arc<std::sync::atomic::AtomicBool>,
    failed: Arc<std::sync::atomic::AtomicBool>,
}
impl SqlDatabase for FailAfterAcceptance {
    fn execute(
        &self,
        sql: String,
        values: Vec<SqlValue>,
    ) -> Result<(), meshchat_core::storage::StorageError> {
        use std::sync::atomic::Ordering::SeqCst;
        if self.failed.load(SeqCst) {
            return Err(meshchat_core::storage::StorageError::Database);
        }
        self.db.execute(sql.clone(), values)?;
        if sql.starts_with("INSERT INTO ledger") {
            self.inserted.store(true, SeqCst);
        }
        if sql == "COMMIT" && self.inserted.swap(false, SeqCst) {
            self.failed.store(true, SeqCst);
        }
        Ok(())
    }
    fn query(
        &self,
        sql: String,
        values: Vec<SqlValue>,
        limit: u32,
    ) -> Result<Vec<meshchat_core::storage::SqlRow>, meshchat_core::storage::StorageError> {
        if self.failed.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(meshchat_core::storage::StorageError::Database);
        }
        self.db.query(sql, values, limit)
    }
}
fn fail_after_acceptance(node: &mut Node) {
    let db = FailAfterAcceptance {
        db: node.db.clone(),
        inserted: Default::default(),
        failed: Default::default(),
    };
    node.store = EncryptedStore::open(
        Box::new(db),
        node.key.public_identity().unwrap().generation,
        false,
        Some(WALL),
    )
    .unwrap();
}
#[test]
fn committed_reactions_keep_ciphertext_and_orphans_when_storage_then_fails() {
    for orphan in [false, true] {
        let mut a = Node::new(2, 3);
        let mut b = Node::new(3, 2);
        if !orphan {
            let target = a.send(1, Content::Chat("target".into()), 1, WALL);
            b.receive(&target, 1);
        }
        // Both ends lose database access immediately after ledger/history COMMIT.
        // Outgoing bytes and incoming acceptance/orphan registration still survive.
        fail_after_acceptance(&mut a);
        fail_after_acceptance(&mut b);
        let reaction = a.send(
            2,
            Content::Reaction {
                target: 1u64.to_be_bytes(),
                remove: false,
                code: 7,
            },
            1000,
            WALL,
        );
        let received = b.receive(&reaction, 1000);
        assert_eq!(received.result, AcceptResult::Accepted);
        assert_eq!(received.content.is_none(), orphan);
        assert_eq!(a.dm.orphan_count(), usize::from(orphan));
        assert_eq!(b.dm.orphan_count(), usize::from(orphan));
        // The sender still owns the exact ciphertext for transport retry.
        assert_eq!(
            codec::parse(&reaction, codec::Context::Live)
                .unwrap()
                .header()
                .kind,
            6
        );
    }
}
