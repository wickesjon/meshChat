mod database;
use database::Database;
use ed25519_dalek::{Signer, SigningKey};
use meshchat_core::{
    LinkHandle, codec, framing,
    friends::*,
    identity::IdentityKeySession,
    ingress::{Ingress, Outcome, State},
    links::UnconfirmedProposal,
    storage::{EncryptedStore, RecordKind, SqlDatabase, SqlValue},
};
use sha2::{Digest, Sha256};
use std::sync::Arc;
const WALL: i64 = 200_000;
const CHANNEL: [u8; 4] = [1, 2, 3, 4];
fn identity(n: u8) -> Arc<IdentityKeySession> {
    IdentityKeySession::import_unlocked(vec![n; 64], vec![n; 16]).unwrap()
}
fn signing(n: u16) -> SigningKey {
    let mut seed = [0; 32];
    seed[..2].copy_from_slice(&n.to_be_bytes());
    SigningKey::from_bytes(&seed)
}
fn key_id(key: &SigningKey) -> [u8; 8] {
    Sha256::digest(key.verifying_key().to_bytes())[..8]
        .try_into()
        .unwrap()
}
fn proposal(key: &SigningKey, x: u8) -> UnconfirmedProposal {
    let mut bundle = [1; 65];
    bundle[1..33].copy_from_slice(&key.verifying_key().to_bytes());
    let secret = x25519_dalek::StaticSecret::from([x; 32]);
    bundle[33..].copy_from_slice(x25519_dalek::PublicKey::from(&secret).as_bytes());
    UnconfirmedProposal::Friend {
        bundle,
        nickname: "claimed name".into(),
    }
}
fn unsigned(key: &SigningKey, id: u64, announce: bool) -> Vec<u8> {
    let mut payload = (WALL as u32).to_be_bytes().to_vec();
    payload.push(0);
    if announce {
        payload.extend_from_slice(&[0, 0]);
    }
    payload.extend_from_slice(&[1, b'A']);
    payload.extend_from_slice(&[0; 4]);
    if announce {
        payload.extend_from_slice(&[0, 0]);
    } else {
        payload.extend_from_slice(&[0, 5]);
        payload.extend_from_slice(b"hello");
    }
    let header = codec::Header {
        kind: if announce { 2 } else { 1 },
        flags: 0,
        ttl: if announce { 1 } else { 7 },
        message_id: id.to_be_bytes(),
        sender_id: key_id(key),
        channel_id: if announce { [0; 4] } else { CHANNEL },
    };
    let mut out = [0; 1024];
    let n = codec::serialize(header, &payload, codec::Context::Live, &mut out).unwrap();
    out[..n].to_vec()
}
fn transcript(raw: &[u8]) -> Vec<u8> {
    let mut out = b"meshfest/friend-sign/v1\0".to_vec();
    out.extend_from_slice(&raw[..3]);
    out.extend_from_slice(&raw[4..26]);
    out.extend_from_slice(&((raw.len() - 26 - 64) as u16).to_be_bytes());
    out.extend_from_slice(&raw[26..raw.len() - 64]);
    out
}
fn signed(key: &SigningKey, id: u64, include: bool, announce: bool) -> Vec<u8> {
    let mut raw = unsigned(key, id, announce);
    raw[2] = 2;
    raw.extend_from_slice(&key_id(key));
    raw.push(u8::from(include));
    if include {
        raw.extend_from_slice(&key.verifying_key().to_bytes());
    }
    raw.extend_from_slice(&[0; 64]);
    let len = (raw.len() - 26) as u16;
    raw[24..26].copy_from_slice(&len.to_be_bytes());
    resign(key, &mut raw);
    raw
}
fn resign(key: &SigningKey, raw: &mut [u8]) {
    let signature = key.sign(&transcript(raw));
    let at = raw.len() - 64;
    raw[at..].copy_from_slice(&signature.to_bytes());
}
fn whole(raw: &[u8]) -> Vec<u8> {
    let mut frame = vec![0, 0];
    frame.extend_from_slice(&(raw.len() as u16).to_be_bytes());
    frame.extend_from_slice(raw);
    frame
}
fn transport(kind: u8, raw: &[u8]) -> Vec<u8> {
    let mut frame = vec![2, 0];
    frame.extend_from_slice(&((raw.len() + 1) as u16).to_be_bytes());
    frame.push(kind);
    frame.extend_from_slice(raw);
    frame
}
struct Setup {
    db: Database,
    store: Arc<EncryptedStore>,
    own: Arc<IdentityKeySession>,
    friends: Friends,
    ingress: Ingress,
    link: LinkHandle,
    peer: SigningKey,
    pin: SendToken,
    local: [u8; 54],
    remote: [u8; 54],
}
impl Setup {
    fn new() -> Self {
        let db = Database::new();
        let own = identity(41);
        let public = own.public_identity().unwrap();
        let store = EncryptedStore::open(
            Box::new(db.clone()),
            public.generation.clone(),
            true,
            Some(WALL),
        )
        .unwrap();
        let mut friends = Friends::open(&store, &public, 0).unwrap();
        let peer = signing(2);
        let pin = friends
            .confirm(&store, &own, proposal(&peer, 2), "Sarah", None)
            .unwrap();
        let mut ingress = Ingress::new(7, 8, 0).unwrap();
        let link = LinkHandle {
            instance_nonce: 7,
            generation: 1,
        };
        let local = friends
            .start_link(
                &mut ingress,
                link.clone(),
                Role::Central,
                (512, 512),
                0,
                || Ok([1; 16]),
            )
            .unwrap();
        friends.hello_transmitted(&link, 0).unwrap();
        let mut remote = local;
        remote[1] = 1;
        remote[6..22].fill(2);
        remote[22..].copy_from_slice(&peer.verifying_key().to_bytes());
        let raw = transport(2, &remote);
        friends
            .receive(&mut ingress, &link, raw.len() as u64, &raw, 0)
            .unwrap();
        Self {
            db,
            store,
            own,
            friends,
            ingress,
            link,
            peer,
            pin,
            local,
            remote,
        }
    }
    fn begin(&mut self, raw: &[u8], now: u64) -> (Outcome, Option<SignatureJob>) {
        let frame = whole(raw);
        self.friends
            .receive(
                &mut self.ingress,
                &self.link,
                frame.len() as u64,
                &frame,
                now,
            )
            .unwrap()
    }
    fn finish(&mut self, job: SignatureJob, now: u64) -> Option<VerifiedContent> {
        self.friends
            .complete(&mut self.ingress, &self.store, job, now, Some(WALL))
            .unwrap()
    }
    fn pending(&self, raw: &[u8]) -> usize {
        (0..32)
            .find(|i| self.ingress.pending_bytes(*i) == Some(raw))
            .unwrap()
    }
    fn peer_proof(&self) -> [u8; 66] {
        let mut bytes = b"meshfest/link-proof/v1\0".to_vec();
        bytes.push(1);
        bytes.extend_from_slice(&self.local);
        bytes.extend_from_slice(&self.remote);
        let mut proof = [0; 66];
        proof[0] = 1;
        proof[1] = 1;
        proof[2..].copy_from_slice(&self.peer.sign(&bytes).to_bytes());
        proof
    }
    fn prove(&mut self) {
        let proof = self
            .friends
            .local_proof(&mut self.ingress, &self.own, &self.link, 1)
            .unwrap()
            .unwrap();
        self.friends
            .proof_transmitted(&self.link, &proof, 1)
            .unwrap();
        let frame = transport(3, &self.peer_proof());
        self.friends
            .receive(&mut self.ingress, &self.link, frame.len() as u64, &frame, 2)
            .unwrap();
    }
}

#[test]
fn confirmed_pins_restart_replacement_rollback_removal_and_stale_sends() {
    let mut s = Setup::new();
    assert_eq!(s.friends.pins()[0].petname(), "Sarah");
    s.friends.begin_replacement(&s.store, &s.pin).unwrap();
    assert!(s.friends.validate_send(&s.store, &s.pin).is_err());
    let own = s.own.public_identity().unwrap();
    s.friends = Friends::open(&s.store, &own, 0).unwrap();
    assert!(s.friends.pins()[0].replacing());
    let new = signing(3);
    s.db.fail(Some("INSERT INTO records(kind,key,value) VALUES(2"));
    assert!(
        s.friends
            .confirm(
                &s.store,
                &s.own,
                proposal(&new, 3),
                "Sarah new",
                Some(&s.pin)
            )
            .is_err()
    );
    s.db.fail(None);
    assert_eq!(s.friends.pins().len(), 1);
    assert_eq!(s.friends.pins()[0].petname(), "Sarah");
    let next = s
        .friends
        .confirm(
            &s.store,
            &s.own,
            proposal(&new, 3),
            "Sarah new",
            Some(&s.pin),
        )
        .unwrap();
    assert!(s.friends.validate_send(&s.store, &s.pin).is_err());
    assert!(s.friends.validate_send(&s.store, &next).is_ok());
    s.friends.remove(&s.store, &next).unwrap();
    assert!(s.friends.validate_send(&s.store, &next).is_err());
    s.store
        .delete_record(RecordKind::Setting, b"mc019.pin.counter".to_vec())
        .unwrap();
    let again = s
        .friends
        .confirm(&s.store, &s.own, proposal(&new, 3), "Sarah", None)
        .unwrap();
    assert_ne!(next, again);
    let path = s.db.path.clone();
    drop(s);
    let db = Database::reopen(path);
    let store =
        EncryptedStore::open(Box::new(db), own.generation.clone(), false, Some(WALL)).unwrap();
    let mut friends = Friends::open(&store, &own, 0).unwrap();
    assert_eq!(friends.pins()[0].token(), again);
    assert!(friends.validate_send(&store, &next).is_err());
}
#[test]
fn qr_requires_strict_keys_and_available_provider_before_commit() {
    let mut s = Setup::new();
    let before = s.friends.pins().to_vec();
    for field in [0, 1, 2] {
        let UnconfirmedProposal::Friend {
            mut bundle,
            nickname,
        } = proposal(&signing(3), 3)
        else {
            unreachable!()
        };
        match field {
            0 => bundle[1..33].fill(0),
            1 => bundle[33..].fill(255),
            _ => bundle[33..].fill(0),
        };
        assert!(
            s.friends
                .confirm(
                    &s.store,
                    &s.own,
                    UnconfirmedProposal::Friend { bundle, nickname },
                    "Bad",
                    None
                )
                .is_err()
        );
    }
    s.own.invalidate().unwrap();
    assert!(
        s.friends
            .confirm(&s.store, &s.own, proposal(&signing(3), 3), "Bad", None)
            .is_err()
    );
    assert_eq!(s.friends.pins(), before);
}
#[test]
fn real_invalid_first_valid_same_id_replay_conflict_and_history_deletion() {
    let mut s = Setup::new();
    let good = signed(&s.peer, 1, true, false);
    let mut bad = good.clone();
    *bad.last_mut().unwrap() ^= 1;
    let job = s.begin(&bad, 1).1.unwrap();
    assert!(s.finish(job, 1).is_none());
    let job = s.begin(&good, 2).1.unwrap();
    let result = s.finish(job, 2).unwrap();
    assert!(result.new_history);
    assert_eq!(result.friend.unwrap().petname(), "Sarah");
    assert!(s.begin(&good, 3).1.is_none());
    s.store.delete_history(CHANNEL.to_vec(), false).unwrap();
    let mut conflict = good.clone();
    conflict[40] ^= 1;
    resign(&s.peer, &mut conflict);
    let job = s.begin(&conflict, 4).1.unwrap();
    assert!(s.finish(job, 4).unwrap().conflict);
    assert!(
        s.store
            .history(CHANNEL.to_vec(), false, 100)
            .unwrap()
            .is_empty()
    );
    let job = s.begin(&good, 900_004).1.unwrap();
    let result = s.finish(job, 900_004).unwrap();
    assert!(result.replay && !result.new_history);
    assert!(
        s.store
            .history(CHANNEL.to_vec(), false, 100)
            .unwrap()
            .is_empty()
    );
}
#[test]
fn copied_claims_unknown_keys_and_bounded_pending_recovery() {
    let mut s = Setup::new();
    let unsigned = unsigned(&s.peer, 8, false);
    let (out, job) = s.begin(&unsigned, 1);
    assert!(matches!(
        out,
        Outcome::Complete {
            state: State::Unverified,
            ..
        }
    ));
    assert!(job.is_none());
    let stranger = signing(3);
    let missing = signed(&stranger, 2, false, false);
    assert!(s.begin(&missing, 2).1.is_none());
    let included = signed(&stranger, 3, true, false);
    let job = s.begin(&included, 3).1.unwrap();
    let result = s.finish(job, 3).unwrap();
    assert!(result.friend.is_none());
    assert_eq!(s.friends.pins().len(), 1);
    let pos = s.pending(&missing);
    let job = s
        .friends
        .retry_pending(&mut s.ingress, &s.link, pos, 4)
        .unwrap()
        .unwrap();
    assert!(s.finish(job, 4).unwrap().new_history);
}
#[test]
fn two_concurrent_real_jobs_and_budget_available_recovery() {
    let mut s = Setup::new();
    let a = signed(&s.peer, 1, true, false);
    let b = signed(&s.peer, 2, true, false);
    let c = signed(&s.peer, 3, true, false);
    let one = s.begin(&a, 1).1.unwrap();
    assert!(s.begin(&a, 1).1.is_none());
    let two = s.begin(&b, 1).1.unwrap();
    assert!(s.begin(&c, 1).1.is_none());
    assert!(s.finish(one, 1).unwrap().new_history);
    assert!(s.finish(two, 1).unwrap().new_history);
    let pos = s.pending(&c);
    let three = s
        .friends
        .retry_pending(&mut s.ingress, &s.link, pos, 2)
        .unwrap()
        .unwrap();
    assert!(s.finish(three, 2).unwrap().new_history);
    let held = s.ingress.begin_work(&s.link, 17, 2).unwrap().unwrap();
    s.ingress.finish_work(held).unwrap();
    let d = signed(&s.peer, 4, true, false);
    assert!(s.begin(&d, 2).1.is_none());
    let pos = s.pending(&d);
    let job = s
        .friends
        .retry_pending(&mut s.ingress, &s.link, pos, 1_002)
        .unwrap()
        .unwrap();
    assert!(s.finish(job, 1_002).unwrap().new_history);
}
#[test]
fn atomic_effect_failure_clock_refusal_and_retry_do_not_poison_valid_bytes() {
    let mut s = Setup::new();
    let raw = signed(&s.peer, 1, true, false);
    s.db.fail(Some("INSERT INTO history"));
    let job = s.begin(&raw, 1).1.unwrap();
    assert!(
        s.friends
            .complete(&mut s.ingress, &s.store, job, 1, Some(WALL))
            .is_err()
    );
    s.db.fail(None);
    let pos = s.pending(&raw);
    let job = s
        .friends
        .retry_pending(&mut s.ingress, &s.link, pos, 2)
        .unwrap()
        .unwrap();
    assert!(s.finish(job, 2).unwrap().new_history);
    let raw = signed(&s.peer, 2, true, false);
    let job = s.begin(&raw, 3).1.unwrap();
    assert!(
        s.friends
            .complete(&mut s.ingress, &s.store, job, 3, None)
            .is_err()
    );
    let pos = s.pending(&raw);
    let job = s
        .friends
        .retry_pending(&mut s.ingress, &s.link, pos, 4)
        .unwrap()
        .unwrap();
    assert!(s.finish(job, 4).unwrap().new_history);
}
#[test]
fn public_cache_lru_expiry_and_pending_eviction_have_no_trust_shortcuts() {
    let mut s = Setup::new();
    for n in 10..267 {
        let key = signing(n);
        let raw = signed(&key, u64::from(n), true, false);
        let now = u64::from(n) * 1000;
        let job = s.begin(&raw, now).1.unwrap();
        assert!(s.finish(job, now).unwrap().friend.is_none());
        assert!(s.friends.cached_keys() <= 256);
    }
    assert_eq!(s.friends.cached_keys(), 256);
    let raw = signed(&signing(10), 999, false, false);
    assert!(s.begin(&raw, 268_000).1.is_none());
    for n in 300..310 {
        let raw = signed(&signing(n), u64::from(n), false, false);
        assert!(s.begin(&raw, u64::from(n) * 1000).1.is_none());
    }
    assert!((0..32).all(|i| s.ingress.pending_bytes(i) != Some(raw.as_slice())));
    let recovered = signed(&signing(10), 999, true, false);
    let job = s.begin(&recovered, 310_001).1.unwrap();
    assert!(s.finish(job, 310_001).unwrap().new_history);
    let omitted = signed(&signing(266), 1000, false, false);
    assert!(s.begin(&omitted, 1_300_002).1.is_none());
    assert_eq!(s.friends.cached_keys(), 0);
    assert!(s.friends.reserved_bytes() < 256 * 1024);
}
#[test]
fn authenticated_proof_freshness_is_nonce_bound_not_announce_bound() {
    let mut s = Setup::new();
    s.prove();
    let first = s.friends.observation(&s.pin, 2).unwrap();
    assert!(first.fresh && first.session_authenticated);
    assert_eq!(first.response_age_ms, Some(0));
    let raw = signed(&s.peer, 1, true, true);
    let job = s.begin(&raw, 59_999).1.unwrap();
    let result = s.finish(job, 59_999).unwrap();
    assert!(!result.new_history && !result.replay);
    assert_eq!(
        s.friends
            .observation(&s.pin, 59_999)
            .unwrap()
            .response_age_ms,
        Some(59_997)
    );
    assert!(!s.friends.observation(&s.pin, 60_000).unwrap().fresh);
    assert!(s.begin(&raw, 60_001).1.is_none());
    s.friends
        .disconnect(&mut s.ingress, &s.link, 60_002)
        .unwrap();
    let old = s.friends.observation(&s.pin, 60_003).unwrap();
    assert!(!old.fresh && !old.session_authenticated);
    assert_eq!(old.response_age_ms, Some(60_001));
}
#[test]
fn reflected_invalid_first_proof_cannot_be_replaced_by_valid_proof() {
    let mut s = Setup::new();
    let local = s
        .friends
        .local_proof(&mut s.ingress, &s.own, &s.link, 1)
        .unwrap()
        .unwrap();
    s.friends.proof_transmitted(&s.link, &local, 1).unwrap();
    let frame = transport(3, &local);
    s.friends
        .receive(&mut s.ingress, &s.link, frame.len() as u64, &frame, 2)
        .unwrap();
    assert!(!s.friends.observation(&s.pin, 2).unwrap().fresh);
    let valid = transport(3, &s.peer_proof());
    assert!(
        s.friends
            .receive(&mut s.ingress, &s.link, valid.len() as u64, &valid, 3)
            .is_err()
    );
    assert!(
        !s.friends
            .observation(&s.pin, 3)
            .unwrap()
            .session_authenticated
    );
}
#[test]
fn proof_replays_deadlines_clock_discontinuity_and_stale_work() {
    let mut s = Setup::new();
    s.prove();
    let used = s.ingress.counters().reserved_work_units;
    let frame = transport(3, &s.peer_proof());
    s.friends
        .receive(&mut s.ingress, &s.link, frame.len() as u64, &frame, 3)
        .unwrap();
    assert_eq!(s.ingress.counters().reserved_work_units, used);
    assert_eq!(
        s.friends.observation(&s.pin, 3).unwrap().response_age_ms,
        Some(1)
    );
    assert!(s.friends.observation(&s.pin, 1).is_err());
    assert!(!s.friends.observation(&s.pin, 4).unwrap().fresh);
    let mut s = Setup::new();
    assert!(
        s.friends
            .local_proof(&mut s.ingress, &s.own, &s.link, 10_001)
            .is_err()
    );
    let raw = signed(&s.peer, 1, true, false);
    let job = s.begin(&raw, 10_002).1.unwrap();
    s.friends
        .disconnect(&mut s.ingress, &s.link, 10_003)
        .unwrap();
    assert!(
        s.friends
            .complete(&mut s.ingress, &s.store, job, 10_003, Some(WALL))
            .is_err()
    );
}
#[test]
fn outgoing_signature_distributes_first_link_key_and_every_announce() {
    let mut s = Setup::new();
    let own = SigningKey::from_bytes(&[41; 32]);
    let raw = unsigned(&own, 1, false);
    let first = s
        .friends
        .sign_content(&mut s.ingress, &s.own, &s.link, &raw, 1)
        .unwrap()
        .unwrap();
    let packet = codec::parse(first.bytes(), codec::Context::Live).unwrap();
    assert!(matches!(
        packet.payload(),
        codec::Payload::Chat {
            signature: codec::Signature::Friend(codec::FriendSignature {
                public_key: Some(_),
                ..
            }),
            ..
        }
    ));
    own.verifying_key()
        .verify_strict(
            &transcript(first.bytes()),
            &ed25519_dalek::Signature::from_slice(&first.bytes()[first.bytes().len() - 64..])
                .unwrap(),
        )
        .unwrap();
    s.friends.content_transmitted(&first).unwrap();
    let second = s
        .friends
        .sign_content(
            &mut s.ingress,
            &s.own,
            &s.link,
            &unsigned(&own, 2, false),
            2,
        )
        .unwrap()
        .unwrap();
    assert!(matches!(
        codec::parse(second.bytes(), codec::Context::Live)
            .unwrap()
            .payload(),
        codec::Payload::Chat {
            signature: codec::Signature::Friend(codec::FriendSignature {
                public_key: None,
                ..
            }),
            ..
        }
    ));
    let announce = s
        .friends
        .sign_content(&mut s.ingress, &s.own, &s.link, &unsigned(&own, 3, true), 3)
        .unwrap()
        .unwrap();
    assert!(matches!(
        codec::parse(announce.bytes(), codec::Context::Live)
            .unwrap()
            .payload(),
        codec::Payload::Announce {
            signature: Some(codec::FriendSignature {
                public_key: Some(_),
                ..
            }),
            ..
        }
    ));
}
#[test]
fn fragmented_real_signature_and_immutable_mutations() {
    let mut s = Setup::new();
    let raw = signed(&s.peer, 1, true, false);
    let encoder = framing::Encoder::logical(&raw, 146, 1).unwrap();
    assert!(encoder.frame_count() > 1);
    let mut job = None;
    for i in 0..encoder.frame_count() {
        let mut frame = [0; 146];
        let n = encoder.frame(i, &mut frame).unwrap();
        job = s
            .friends
            .receive(&mut s.ingress, &s.link, n as u64, &frame[..n], 1)
            .unwrap()
            .1;
    }
    assert!(s.finish(job.unwrap(), 1).unwrap().new_history);
    let mut ttl = raw.clone();
    ttl[3] = 1;
    assert!(s.begin(&ttl, 2).1.is_none());
    for (i, index) in [2, 4, 20, 30].into_iter().enumerate() {
        let mut bad = signed(&s.peer, 10 + i as u64, true, false);
        bad[index] ^= if index == 2 { 128 } else { 1 };
        let job = s.begin(&bad, 3 + i as u64).1.unwrap();
        assert!(s.finish(job, 3 + i as u64).is_none());
    }
}
#[test]
fn full_persistent_ledger_refuses_real_valid_signature_without_effect() {
    let mut s = Setup::new();
    s.db.execute("WITH RECURSIVE n(x) AS (VALUES(0) UNION ALL SELECT x+1 FROM n WHERE x<99999) INSERT INTO ledger(subject,direction,logical_type,message_id,digest,timestamp) SELECT ?,0,1,CAST(printf('%08d',x) AS BLOB),?,? FROM n".into(),vec![SqlValue::Bytes{value:vec![1;33]},SqlValue::Bytes{value:vec![2;32]},SqlValue::Integer{value:WALL}]).unwrap();
    let raw = signed(&s.peer, 1, true, false);
    let job = s.begin(&raw, 1).1.unwrap();
    assert!(matches!(
        s.friends
            .complete(&mut s.ingress, &s.store, job, 1, Some(WALL)),
        Err(Error::Storage(
            meshchat_core::storage::StorageError::Capacity
        ))
    ));
    assert!(
        s.store
            .history(CHANNEL.to_vec(), false, 100)
            .unwrap()
            .is_empty()
    );
}

fn another_link(
    s: &mut Setup,
    generation: u64,
    nonce: u8,
    now: u64,
) -> (LinkHandle, [u8; 54], [u8; 54]) {
    let link = LinkHandle {
        instance_nonce: 7,
        generation,
    };
    let local = s
        .friends
        .start_link(
            &mut s.ingress,
            link.clone(),
            Role::Central,
            (300, 200),
            now,
            || Ok([nonce; 16]),
        )
        .unwrap();
    let mut remote = local;
    remote[1] = 1;
    remote[2..4].copy_from_slice(&180u16.to_be_bytes());
    remote[4..6].copy_from_slice(&250u16.to_be_bytes());
    remote[6..22].fill(nonce + 1);
    remote[22..].copy_from_slice(&s.peer.verifying_key().to_bytes());
    (link, local, remote)
}
fn receive_control(
    s: &mut Setup,
    link: &LinkHandle,
    kind: u8,
    bytes: &[u8],
    now: u64,
) -> Result<(Outcome, Option<SignatureJob>), Error> {
    let frame = transport(kind, bytes);
    s.friends
        .receive(&mut s.ingress, link, frame.len() as u64, &frame, now)
}
fn proof_for(key: &SigningKey, local: &[u8; 54], remote: &[u8; 54]) -> [u8; 66] {
    let mut transcript = b"meshfest/link-proof/v1\0".to_vec();
    transcript.push(1);
    transcript.extend_from_slice(local);
    transcript.extend_from_slice(remote);
    let mut proof = [0; 66];
    proof[..2].copy_from_slice(&[1, 1]);
    proof[2..].copy_from_slice(&key.sign(&transcript).to_bytes());
    proof
}
#[test]
fn handshake_gates_directional_limits_and_unsolicited_sync() {
    let mut s = Setup::new();
    let (link, _, remote) = another_link(&mut s, 2, 10, 1);
    let raw = signed(&s.peer, 50, true, false);
    let frame = whole(&raw);
    let result = s
        .friends
        .receive(&mut s.ingress, &link, frame.len() as u64, &frame, 1)
        .unwrap();
    assert!(result.1.is_none());
    assert!((0..32).all(|i| s.ingress.pending_bytes(i).is_none()));
    receive_control(&mut s, &link, 2, &remote, 2).unwrap();
    s.friends.hello_transmitted(&link, 2).unwrap();
    assert_eq!(s.friends.effective_capacities(&link).unwrap(), (250, 180));
    let job = s
        .friends
        .receive(&mut s.ingress, &link, frame.len() as u64, &frame, 3)
        .unwrap()
        .1
        .unwrap();
    assert!(s.finish(job, 3).unwrap().new_history);
    // Larger than negotiated RX, but smaller than the original local RX limit.
    let oversized = vec![0; 181];
    assert!(
        s.friends
            .receive(&mut s.ingress, &link, 181, &oversized, 4)
            .unwrap()
            .1
            .is_none()
    );
    let inner = signed(&s.peer, 51, true, false);
    let mut sync = vec![0; 11];
    sync[9..11].copy_from_slice(&(inner.len() as u16).to_be_bytes());
    sync.extend_from_slice(&inner);
    let result = receive_control(&mut s, &link, 1, &sync, 5).unwrap();
    assert!(matches!(
        result.0,
        Outcome::Complete {
            state: State::DeferredSync,
            ..
        }
    ));
    assert!((0..32).all(|i| s.ingress.pending_bytes(i) != Some(&inner)));
    let mut changed = remote;
    changed[6] ^= 1;
    assert!(receive_control(&mut s, &link, 2, &changed, 6).is_err());
    assert!(s.friends.effective_capacities(&link).is_err());
    assert!(s.ingress.begin_work(&link, 1, 6).is_err());
}
#[test]
fn hello_nonce_provider_and_timeout_fail_closed() {
    let mut s = Setup::new();
    for nonce in [[0; 16], [1; 16]] {
        assert!(
            s.friends
                .start_link(
                    &mut s.ingress,
                    LinkHandle {
                        instance_nonce: 7,
                        generation: 2
                    },
                    Role::Central,
                    (512, 512),
                    1,
                    || Ok(nonce)
                )
                .is_err()
        );
    }
    assert!(
        s.friends
            .start_link(
                &mut s.ingress,
                LinkHandle {
                    instance_nonce: 7,
                    generation: 2
                },
                Role::Central,
                (512, 512),
                1,
                || Err(Error::Provider)
            )
            .is_err()
    );
    let (link, _, mut remote) = another_link(&mut s, 2, 10, 2);
    remote[22..].copy_from_slice(&s.own.public_identity().unwrap().signing_key);
    assert!(receive_control(&mut s, &link, 2, &remote, 3).is_err());
    remote[22..].copy_from_slice(&s.peer.verifying_key().to_bytes());
    assert!(receive_control(&mut s, &link, 2, &remote, 10_003).is_err());
    assert!(s.friends.hello_transmitted(&link, 10_003).is_err());
    s.own.invalidate().unwrap();
    assert!(
        s.friends
            .local_proof(&mut s.ingress, &s.own, &s.link, 10_003)
            .is_err()
    );
}
#[test]
fn duplicate_arbitration_requires_both_proofs_and_local_retry_is_identical() {
    let mut s = Setup::new();
    s.prove();
    assert!(s.friends.duplicate_links_to_close().is_empty());
    let (link, local, remote) = another_link(&mut s, 2, 10, 3);
    receive_control(&mut s, &link, 2, &remote, 3).unwrap();
    s.friends.hello_transmitted(&link, 3).unwrap();
    let proof = s
        .friends
        .local_proof(&mut s.ingress, &s.own, &link, 4)
        .unwrap()
        .unwrap();
    s.friends.proof_transmitted(&link, &proof, 4).unwrap();
    assert!(s.friends.duplicate_links_to_close().is_empty());
    let retry = s
        .friends
        .local_proof(&mut s.ingress, &s.own, &link, 5)
        .unwrap()
        .unwrap();
    assert_eq!(proof, retry);
    s.friends.proof_transmitted(&link, &retry, 5).unwrap();
    assert!(
        s.friends
            .local_proof(&mut s.ingress, &s.own, &link, 6)
            .unwrap()
            .is_none()
    );
    assert!(s.friends.proof_transmitted(&link, &retry, 6).is_err());
    let proof = proof_for(&s.peer, &local, &remote);
    receive_control(&mut s, &link, 3, &proof, 6).unwrap();
    assert_eq!(s.friends.duplicate_links_to_close(), vec![link]);
}
#[test]
fn cross_link_proof_replay_fails_and_budget_recovery_verifies_one_candidate() {
    let mut s = Setup::new();
    let (link, _, remote) = another_link(&mut s, 2, 10, 1);
    receive_control(&mut s, &link, 2, &remote, 1).unwrap();
    s.friends.hello_transmitted(&link, 1).unwrap();
    let proof = s
        .friends
        .local_proof(&mut s.ingress, &s.own, &link, 2)
        .unwrap()
        .unwrap();
    s.friends.proof_transmitted(&link, &proof, 2).unwrap();
    let wrong = s.peer_proof();
    receive_control(&mut s, &link, 3, &wrong, 2).unwrap();
    assert!(
        !s.friends
            .observation(&s.pin, 2)
            .unwrap()
            .session_authenticated
    );
    let local = s
        .friends
        .local_proof(&mut s.ingress, &s.own, &s.link, 3)
        .unwrap()
        .unwrap();
    s.friends.proof_transmitted(&s.link, &local, 3).unwrap();
    let held = s.ingress.begin_work(&s.link, 19, 3).unwrap().unwrap();
    s.ingress.finish_work(held).unwrap();
    let link = s.link.clone();
    let proof = s.peer_proof();
    receive_control(&mut s, &link, 3, &proof, 3).unwrap();
    assert!(!s.friends.observation(&s.pin, 3).unwrap().fresh);
    receive_control(&mut s, &link, 3, &proof, 1003).unwrap();
    assert!(s.friends.observation(&s.pin, 1003).unwrap().fresh);
}
#[test]
fn expired_jobs_free_slots_and_cannot_authenticate() {
    let mut s = Setup::new();
    let raw = signed(&s.peer, 1, true, false);
    let job = s.begin(&raw, 1).1.unwrap();
    assert!(
        s.friends
            .complete(&mut s.ingress, &s.store, job, 30_001, Some(WALL))
            .is_err()
    );
    let job = s.begin(&raw, 30_002).1.unwrap();
    assert!(s.finish(job, 30_002).unwrap().new_history);
}
#[test]
fn strict_signature_encodings_hint_collisions_and_content_window() {
    assert_eq!(unique_signing_candidate([].into_iter()), None);
    assert_eq!(
        unique_signing_candidate([[1; 32], [1; 32]].into_iter()),
        Some([1; 32])
    );
    assert_eq!(
        unique_signing_candidate([[1; 32], [2; 32]].into_iter()),
        None
    );
    let mut s = Setup::new();
    for n in 0..3 {
        let mut raw = signed(&s.peer, n, true, false);
        let at = raw.len() - 64;
        match n {
            0 => raw[at..at + 32].fill(0),   // small-order R
            1 => raw[at..at + 32].fill(255), // noncanonical R
            _ => raw[at + 32..].copy_from_slice(&[
                0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9,
                0xde, 0x14, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x10,
            ]), // S = L
        }
        let job = s.begin(&raw, n + 1).1.unwrap();
        assert!(s.finish(job, n + 1).is_none());
    }
    let mut mismatched = signed(&s.peer, 10, true, false);
    mismatched[12] ^= 1;
    resign(&s.peer, &mut mismatched);
    assert!(s.begin(&mismatched, 4).1.is_none());
    for (n, timestamp) in [WALL - 172801, WALL + 301].into_iter().enumerate() {
        let mut raw = signed(&s.peer, 20 + n as u64, true, false);
        raw[26..30].copy_from_slice(&(timestamp as u32).to_be_bytes());
        resign(&s.peer, &mut raw);
        let job = s.begin(&raw, 5 + n as u64).1.unwrap();
        assert!(
            s.friends
                .complete(&mut s.ingress, &s.store, job, 5 + n as u64, Some(WALL))
                .is_err()
        );
    }
    assert!(
        s.store
            .history(CHANNEL.to_vec(), false, 100)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn independent_openssl_vectors_match_real_sender_verifier_and_proof() {
    let vectors: std::collections::HashMap<_, _> =
        include_str!("../../vectors/crypto/friend-v1.tsv")
            .lines()
            .map(|line| {
                let (name, bytes) = line.split_once('\t').unwrap();
                (name, database::unhex(bytes))
            })
            .collect();
    let mut s = Setup::new();
    assert_eq!(s.local.as_slice(), vectors["local_hello"]);
    assert_eq!(s.remote.as_slice(), vectors["remote_hello"]);
    assert_eq!(
        s.peer.verifying_key().as_bytes().as_slice(),
        vectors["peer_public"]
    );
    let job = s.begin(&vectors["peer_chat"], 1).1.unwrap();
    let result = s.finish(job, 1).unwrap();
    assert!(result.new_history && result.friend.is_some());
    let own = SigningKey::from_bytes(&[41; 32]);
    let packet = s
        .friends
        .sign_content(
            &mut s.ingress,
            &s.own,
            &s.link,
            &unsigned(&own, 1, false),
            2,
        )
        .unwrap()
        .unwrap();
    assert_eq!(packet.bytes(), vectors["own_chat"]);
    let proof = s
        .friends
        .local_proof(&mut s.ingress, &s.own, &s.link, 3)
        .unwrap()
        .unwrap();
    assert_eq!(proof.as_slice(), vectors["own_proof"]);
    s.friends.proof_transmitted(&s.link, &proof, 3).unwrap();
    let link = s.link.clone();
    receive_control(&mut s, &link, 3, &vectors["peer_proof"], 4).unwrap();
    assert!(s.friends.observation(&s.pin, 4).unwrap().fresh);
}

#[test]
fn removal_clears_live_trust_and_replayed_history_does_not_restore_friend() {
    let mut s = Setup::new();
    s.prove();
    let raw = signed(&s.peer, 1, true, false);
    let job = s.begin(&raw, 3).1.unwrap();
    assert!(s.finish(job, 3).unwrap().friend.is_some());
    s.friends.remove(&s.store, &s.pin).unwrap();
    assert!(s.friends.observation(&s.pin, 4).is_err());
    assert_eq!(s.friends.cached_keys(), 0);
    assert!(s.friends.validate_send(&s.store, &s.pin).is_err());
    let (link, _, remote) = another_link(&mut s, 2, 10, 900_004);
    receive_control(&mut s, &link, 2, &remote, 900_004).unwrap();
    s.friends.hello_transmitted(&link, 900_004).unwrap();
    let frame = whole(&raw);
    let job = s
        .friends
        .receive(&mut s.ingress, &link, frame.len() as u64, &frame, 900_005)
        .unwrap()
        .1
        .unwrap();
    let result = s.finish(job, 900_005).unwrap();
    assert!(result.friend.is_none() && result.replay && !result.new_history);
    assert!(s.friends.pins().is_empty());
}

#[test]
fn invalidated_provider_removes_previously_authenticated_session() {
    let mut s = Setup::new();
    s.prove();
    s.own.invalidate().unwrap();
    assert!(
        s.friends
            .local_proof(&mut s.ingress, &s.own, &s.link, 3)
            .is_err()
    );
    let observation = s.friends.observation(&s.pin, 3).unwrap();
    assert!(!observation.fresh && !observation.session_authenticated);
    assert_eq!(observation.response_age_ms, Some(1));
}
