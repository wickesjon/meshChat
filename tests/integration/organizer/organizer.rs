#[path = "../friends/database.rs"]
mod database;
use database::Database;
use ed25519_dalek::{Signer, SigningKey};
use meshchat_core::{
    LinkHandle, codec, framing,
    friends::{Friends, Role},
    identity::IdentityKeySession,
    ingress::Ingress,
    links::{self, UnconfirmedProposal},
    organizer::*,
    storage::{AcceptResult, EncryptedStore, SqlDatabase, SqlValue},
};
use sha2::{Digest, Sha256};
use std::sync::Arc;
const WALL: i64 = 200_000;
fn key(n: u8) -> SigningKey {
    SigningKey::from_bytes(&[n; 32])
}
fn hint(k: &SigningKey) -> [u8; 8] {
    Sha256::digest(k.verifying_key().as_bytes())[..8]
        .try_into()
        .unwrap()
}
fn root(n: u8, expiry: u32) -> UnconfirmedProposal {
    let k = key(n);
    let mut b = [0; 101];
    b[0] = 1;
    b[1..33].copy_from_slice(k.verifying_key().as_bytes());
    b[33..37].copy_from_slice(&expiry.to_be_bytes());
    let mut signed = b"meshfest/event-root/v1\0".to_vec();
    signed.extend_from_slice(&b[..37]);
    b[37..].copy_from_slice(&k.sign(&signed).to_bytes());
    UnconfirmedProposal::Event {
        bundle: b,
        name: "Festival".into(),
        root_id: hint(&k),
    }
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
fn credential() -> Vec<u8> {
    cred(9, 7, WALL as u32 - 10, WALL as u32 + 1000)
}
#[test]
fn signed_forbidden_labels_never_gain_authority_or_poison_valid_recovery() {
    for label in ["\u{202e}Ops", "\u{200b}", "A\nB", "A\0B"] {
        let mut credential = credential();
        credential.truncate(50);
        credential[49] = label.len() as u8;
        credential.extend_from_slice(label.as_bytes());
        let mut signed = b"meshfest/credential/v1\0".to_vec();
        signed.extend_from_slice(&(credential.len() as u16).to_be_bytes());
        signed.extend_from_slice(&credential);
        credential.extend_from_slice(&key(9).sign(&signed).to_bytes());
        let uri = zeroize::Zeroizing::new(format!(
            "meshfest://staff/{}/{}",
            b64(&credential),
            b64(&[7; 32])
        ));
        assert!(links::parse(&uri).is_err());

        let mut n = Node::new(3, true);
        let offered = offer(&credential, 100u64.to_be_bytes(), hint(&key(2))).unwrap();
        let job = n.feed(&offered, 1000).unwrap().job.unwrap();
        assert!(matches!(n.finish(job, 1000), Err(Error::Authority)));
        // Even if retained opaquely for relaying, this credential cannot authorize
        // a later omitted-credential message.
        let omitted = chat(9, 7, 1, false, 0);
        let job = n.feed(&omitted, 2000).unwrap().job.unwrap();
        assert!(matches!(n.finish(job, 2000), Err(Error::Authority)));

        let valid = chat(9, 7, 2, true, 0);
        let mut included = valid.clone();
        let start = included.len() - 64 - self::credential().len();
        included.splice(start..included.len() - 64, credential.iter().copied());
        included[start - 2..start].copy_from_slice(&(credential.len() as u16).to_be_bytes());
        let payload_len = (included.len() - 26) as u16;
        included[24..26].copy_from_slice(&payload_len.to_be_bytes());
        resign(&mut included, 7);
        let job = n.feed(&included, 3000).unwrap().job.unwrap();
        assert!(matches!(n.finish(job, 3000), Err(Error::Authority)));
        let rows =
            n.db.query("SELECT count(*) FROM history".into(), vec![], 1)
                .unwrap();
        assert!(matches!(rows[0].cells[0], SqlValue::Integer { value: 0 }));
        assert_eq!(
            n.receive(&valid, 4000).unwrap().result,
            AcceptResult::Accepted
        );
    }
}
fn chat(r: u8, s: u8, id: u64, include: bool, pin: u32) -> Vec<u8> {
    let credential = cred(r, s, WALL as u32 - 10, WALL as u32 + 1000);
    let mut body = (WALL as u32).to_be_bytes().to_vec();
    body.extend_from_slice(&[0, 1, b'A', 0, 0, 0, 0, 0, 5]);
    body.extend_from_slice(b"hello");
    body.push(u8::from(pin > 0));
    body.extend_from_slice(&pin.to_be_bytes());
    body.extend_from_slice(&hint(&key(r)));
    body.extend_from_slice(&hint(&key(s)));
    body.push(u8::from(include));
    body.extend_from_slice(&(if include { credential.len() as u16 } else { 0 }).to_be_bytes());
    if include {
        body.extend_from_slice(&credential);
    }
    body.extend_from_slice(&[0; 64]);
    let mut raw = vec![1, 1, 6, 7];
    raw.extend_from_slice(&id.to_be_bytes());
    raw.extend_from_slice(&hint(&key(2)));
    raw.extend_from_slice(&codec::EVENT_CHANNEL);
    raw.extend_from_slice(&(body.len() as u16).to_be_bytes());
    raw.extend_from_slice(&body);
    resign(&mut raw, s);
    raw
}
fn resign(raw: &mut [u8], s: u8) {
    let end = raw.len() - 64;
    let mut signed = b"meshfest/staff-sign/v1\0".to_vec();
    signed.extend_from_slice(&raw[..3]);
    signed.extend_from_slice(&raw[4..26]);
    signed.extend_from_slice(&((end - 26) as u16).to_be_bytes());
    signed.extend_from_slice(&raw[26..end]);
    raw[end..].copy_from_slice(&key(s).sign(&signed).to_bytes());
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
fn staff(raw: &[u8], seed: u8) -> links::StaffProposal {
    // Transient synthetic provisioning only: no seed-bearing URI is persisted/logged.
    use zeroize::Zeroize;
    let mut uri = format!("meshfest://staff/{}/{}", b64(raw), b64(&[seed; 32]));
    let p = links::parse(&uri).unwrap();
    uri.zeroize();
    let UnconfirmedProposal::Staff(p) = p else {
        panic!()
    };
    p
}
struct Node {
    db: Database,
    store: Arc<EncryptedStore>,
    identity: Arc<IdentityKeySession>,
    org: Organizer,
    friends: Friends,
    ingress: Ingress,
    link: LinkHandle,
}
impl Node {
    fn new(n: u8, adopt: bool) -> Self {
        let identity = IdentityKeySession::import_unlocked(vec![n; 64], vec![n; 16]).unwrap();
        let own = identity.public_identity().unwrap();
        let peer = IdentityKeySession::import_unlocked(vec![n + 1; 64], vec![n + 1; 16])
            .unwrap()
            .public_identity()
            .unwrap();
        let db = Database::new();
        let store = EncryptedStore::open(
            Box::new(db.clone()),
            own.generation.clone(),
            true,
            Some(WALL),
        )
        .unwrap();
        let mut friends = Friends::open(&store, &own, 0).unwrap();
        let mut ingress = Ingress::new(u64::from(n), 8, 0).unwrap();
        let link = LinkHandle {
            instance_nonce: u64::from(n),
            generation: 1,
        };
        let mut hello = friends
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
        hello[1] = 1;
        hello[6..22].fill(n + 1);
        hello[22..].copy_from_slice(&peer.signing_key);
        let mut frame = vec![2, 0, 0, 55, 2];
        frame.extend_from_slice(&hello);
        friends
            .receive(&mut ingress, &link, frame.len() as u64, &frame, 0)
            .unwrap();
        let org = Organizer::new(&own).unwrap();
        let mut node = Self {
            db,
            store,
            identity,
            org,
            friends,
            ingress,
            link,
        };
        if adopt {
            node.adopt(9, 0);
        }
        node
    }
    fn adopt(&mut self, n: u8, now: u64) {
        assert!(
            self.org
                .adopt(
                    &mut self.ingress,
                    &self.store,
                    root(n, WALL as u32 + 2000),
                    now,
                    Some(WALL)
                )
                .unwrap()
        );
    }
    fn feed(&mut self, raw: &[u8], now: u64) -> Result<Intake, Error> {
        let encoder =
            framing::Encoder::logical(raw, 512, now as u16).map_err(|_| Error::Invalid)?;
        let mut result = None;
        for i in 0..encoder.frame_count() {
            let mut frame = [0; 512];
            let n = encoder.frame(i, &mut frame).unwrap();
            result = Some(self.org.receive(
                &mut self.ingress,
                &self.friends,
                &self.store,
                &self.link,
                n as u64,
                &frame[..n],
                now,
                Some(WALL),
            )?);
        }
        Ok(result.unwrap())
    }
    fn finish(&mut self, job: Job, now: u64) -> Result<Option<Verified>, Error> {
        self.org
            .complete(&mut self.ingress, &self.store, job, now, Some(WALL))
    }
    fn receive(&mut self, raw: &[u8], now: u64) -> Option<Verified> {
        let job = self.feed(raw, now).unwrap().job?;
        self.finish(job, now).unwrap()
    }
    fn retry(&mut self, raw: &[u8], now: u64) -> Option<Job> {
        let p = (0..32).find(|p| self.ingress.pending_bytes(*p) == Some(raw))?;
        self.org
            .retry(
                &mut self.ingress,
                &self.friends,
                &self.store,
                &self.link,
                p,
                now,
                Some(WALL),
            )
            .unwrap()
    }
}
#[test]
fn reference_vectors_adoption_multiple_staff_and_pin_expiry() {
    for (i, s) in [7, 8].into_iter().enumerate() {
        let mut n = Node::new(3, true);
        let raw = chat(9, s, i as u64 + 1, true, WALL as u32 + 100);
        let v = n.receive(&raw, i as u64 * 1000 + 1).unwrap();
        assert_eq!(v.result, AcceptResult::Accepted);
        assert_eq!(v.authority.staff(), key(s).verifying_key().as_bytes());
        assert_eq!(
            n.org.current(&n.store, &v.authority, Some(WALL)).unwrap(),
            (true, true)
        );
        assert_eq!(
            n.org
                .current(&n.store, &v.authority, Some(WALL + 101))
                .unwrap(),
            (true, false)
        );
    }
}
#[test]
fn unadopted_and_forged_chains_never_grant_authority() {
    let raw = chat(9, 7, 1, true, 0);
    let mut n = Node::new(3, false);
    assert!(n.receive(&raw, 1).is_none());
    assert_eq!(n.org.cache_count(), 0);
    n.adopt(9, 2);
    let job = n.retry(&raw, 3).unwrap();
    assert_eq!(
        n.finish(job, 3).unwrap().unwrap().result,
        AcceptResult::Accepted
    );
    let mut forged = chat(9, 7, 2, true, 0);
    let signature_at = forged.len() - 64;
    forged[signature_at - 1] ^= 1;
    resign(&mut forged, 7);
    assert!(n.receive(&forged, 1000).is_none());
    let other = chat(10, 7, 3, true, 0);
    assert!(n.receive(&other, 2000).is_none());
}
#[test]
fn invalid_first_valid_second_replay_conflict_and_persistent_restart() {
    let mut n = Node::new(3, true);
    let raw = chat(9, 7, 1, true, 0);
    let mut bad = raw.clone();
    *bad.last_mut().unwrap() ^= 1;
    assert!(n.receive(&bad, 1).is_none());
    let v = n.receive(&raw, 1000).unwrap();
    assert_eq!(v.result, AcceptResult::Accepted);
    assert!(n.feed(&raw, 1001).unwrap().job.is_none());
    let mut conflict = raw.clone();
    conflict[33] = 1;
    resign(&mut conflict, 7);
    assert_eq!(
        n.receive(&conflict, 2000).unwrap().result,
        AcceptResult::Conflict
    );
    n.store
        .delete_history(codec::EVENT_CHANNEL.to_vec(), false)
        .unwrap();
    let path = n.db.path.clone();
    let own = n.identity.public_identity().unwrap();
    drop(n.store);
    n.db = Database::reopen(path);
    n.store = EncryptedStore::open(
        Box::new(n.db.clone()),
        own.generation.clone(),
        false,
        Some(WALL),
    )
    .unwrap();
    n.org = Organizer::new(&own).unwrap();
    assert_eq!(
        n.receive(&raw, 901_001).unwrap().result,
        AcceptResult::Replay
    );
}
#[test]
fn every_immutable_byte_is_bound_and_ttl_is_mutable() {
    let raw = chat(9, 7, 1, true, WALL as u32 + 100);
    let mut n = Node::new(3, true);
    for i in 0..raw.len() {
        if i == 3 {
            continue;
        }
        let mut bad = raw.clone();
        bad[i] ^= 1;
        let now = (i as u64 + 1) * 1000;
        if let Ok(intake) = n.feed(&bad, now) {
            if let Some(job) = intake.job {
                assert!(
                    n.finish(job, now).ok().flatten().is_none(),
                    "accepted altered byte {i}"
                );
            }
        }
    }
    let mut ttl = raw;
    ttl[3] = 1;
    assert_eq!(
        n.receive(&ttl, 400_000).unwrap().result,
        AcceptResult::Accepted
    );
}
#[test]
fn changed_adoption_held_jobs_clock_and_authority_lifetime() {
    let mut n = Node::new(3, true);
    let raw = chat(9, 7, 1, true, 0);
    let job = n.feed(&raw, 1).unwrap().job.unwrap();
    n.org
        .remove(&n.store, key(9).verifying_key().to_bytes(), Some(WALL))
        .unwrap();
    n.adopt(9, 2);
    assert!(n.finish(job, 3).is_err());
    let job = n.retry(&raw, 1000).unwrap();
    let verified = n.finish(job, 1000).unwrap().unwrap();
    assert!(n.org.current(&n.store, &verified.authority, None).is_err());
    assert_eq!(
        n.org
            .current(&n.store, &verified.authority, Some(WALL))
            .unwrap(),
        (true, false)
    );
    assert_eq!(
        n.org
            .current(&n.store, &verified.authority, Some(WALL + 1001))
            .unwrap(),
        (false, false)
    );
    assert_eq!(
        n.org
            .current(&n.store, &verified.authority, Some(WALL + 2001))
            .unwrap(),
        (false, false)
    );
}
#[test]
fn transaction_failure_and_concurrent_jobs_recover_with_real_signatures() {
    let mut n = Node::new(3, true);
    let a = chat(9, 7, 1, true, 0);
    let b = chat(9, 8, 2, true, 0);
    let c = chat(9, 7, 3, true, 0);
    let one = n.feed(&a, 1).unwrap().job.unwrap();
    let two = n.feed(&b, 2).unwrap().job.unwrap();
    assert!(n.feed(&c, 3).unwrap().job.is_none());
    assert!(n.retry(&a, 3).is_none());
    n.db.fail(Some("INSERT INTO history"));
    assert!(n.finish(one, 4).is_err());
    n.db.fail(None);
    assert_eq!(
        n.finish(two, 5).unwrap().unwrap().result,
        AcceptResult::Accepted
    );
    let retry = n.retry(&a, 1000).unwrap();
    assert_eq!(
        n.finish(retry, 1000).unwrap().unwrap().result,
        AcceptResult::Accepted
    );
    let retry = n.retry(&c, 2000).unwrap();
    assert_eq!(
        n.finish(retry, 2000).unwrap().unwrap().result,
        AcceptResult::Accepted
    );
}
#[test]
fn credential_flood_cap_expiry_and_pending_eviction_remain_bounded() {
    let mut blind = Node::new(3, false);
    for i in 0..140 {
        let raw = cred(9, (i + 10) as u8, WALL as u32 - 10, WALL as u32 + 1000);
        let offer = offer(&raw, (i as u64).to_be_bytes(), [1; 8]).unwrap();
        assert!(blind.receive(&offer, i as u64 * 30_000).is_none());
        assert!(blind.org.cache_count() <= 128);
        assert!(blind.org.reserved_bytes() < 128 * 1024);
    }
    // The fifteen-minute unused lifetime also limits this slow flood.
    assert!(blind.org.cache_count() <= 31);
    let mut n = Node::new(4, true);
    let first = chat(9, 7, 1, true, 0);
    let held = n.feed(&first, 1).unwrap().job.unwrap();
    for i in 2..12 {
        let raw = chat(9, 7, i, false, 0);
        assert!(n.feed(&raw, i * 1000).unwrap().job.is_none());
    }
    assert!(n.finish(held, 31_001).is_err());
    assert!(n.retry(&first, 31_002).is_none());
    assert_eq!(
        n.receive(&first, 32_000).unwrap().result,
        AcceptResult::Accepted
    );
}
#[test]
fn real_budget_exhaustion_then_refill_allows_acceptance() {
    let mut n = Node::new(3, true);
    for _ in 0..2 {
        let p = n.ingress.begin_work(&n.link, 10, 1).unwrap().unwrap();
        n.ingress.finish_work(p).unwrap();
    }
    let raw = chat(9, 7, 1, true, 0);
    assert!(n.feed(&raw, 1).unwrap().job.is_none());
    let retry = n.retry(&raw, 2000).unwrap();
    assert_eq!(
        n.finish(retry, 2000).unwrap().unwrap().result,
        AcceptResult::Accepted
    );
}
#[test]
fn late_joiner_recovers_through_initially_empty_nonadopting_relay() {
    let mut late = Node::new(3, true);
    let mut relay = Node::new(4, false);
    let raw = chat(9, 7, 1, false, 0);
    assert!(late.receive(&raw, 1).is_none());
    let req = late.org.next_request(&late.friends, 1).unwrap().unwrap();
    let request = request(req.pair, 2u64.to_be_bytes(), [3; 8]);
    assert!(relay.feed(&request, 1).unwrap().offer_response.is_none());
    // A later flooded offer from upstream supplies the initially empty peer.
    let offered = offer(&credential(), 3u64.to_be_bytes(), [2; 8]).unwrap();
    assert!(relay.receive(&offered, 1000).is_none());
    assert_eq!(relay.org.cache_count(), 1);
    let mut forwarded = vec![0; offered.len()];
    codec::parse(&offered, codec::Context::Live)
        .unwrap()
        .forward_to(&mut forwarded)
        .unwrap()
        .unwrap();
    assert!(late.receive(&forwarded, 1000).is_none());
    let job = late.retry(&raw, 1001).unwrap();
    assert_eq!(
        late.finish(job, 1001).unwrap().unwrap().result,
        AcceptResult::Accepted
    );
    assert!(relay.feed(&request, 6000).unwrap().offer_response.is_none()); // exact control dedup
    let response = relay
        .feed(
            &meshchat_core::organizer::request(req.pair, 4u64.to_be_bytes(), [3; 8]),
            6001,
        )
        .unwrap();
    assert_eq!(response.offer_response.unwrap(), credential());
}
#[test]
fn recovery_retry_and_absolute_deadline_do_not_refresh() {
    let mut n = Node::new(3, true);
    let raw = chat(9, 7, 1, false, 0);
    assert!(n.receive(&raw, 1).is_none());
    assert!(n.org.next_request(&n.friends, 1).unwrap().is_some());
    assert!(n.org.next_request(&n.friends, 5000).unwrap().is_none());
    assert!(n.org.next_request(&n.friends, 5001).unwrap().is_some());
    assert!(n.org.next_request(&n.friends, 10001).unwrap().is_some());
    assert!(n.org.next_request(&n.friends, 15001).unwrap().is_none());
    assert!(n.retry(&raw, 29999).is_none());
    assert!(n.retry(&raw, 30001).is_none());
    assert!(n.org.next_request(&n.friends, 30001).unwrap().is_none());
    assert_eq!(n.org.recovery_count(), 0);
}
#[test]
fn protected_staff_import_sign_forget_and_periodic_offer_policy() {
    let mut a = Node::new(2, true);
    let mut b = Node::new(3, true);
    let session = a
        .org
        .import_staff(
            &mut a.ingress,
            &a.store,
            staff(&credential(), 7),
            1,
            Some(WALL),
        )
        .unwrap()
        .unwrap();
    assert!(
        a.org
            .import_staff(
                &mut a.ingress,
                &a.store,
                staff(&credential(), 8),
                2,
                Some(WALL)
            )
            .is_err()
    );
    assert!(
        a.org
            .sign(
                &mut a.ingress,
                &a.friends,
                &a.store,
                &session,
                &a.link,
                chat(9, 7, 1, false, 0),
                3,
                Some(WALL)
            )
            .is_err()
    );
    let raw = a
        .org
        .sign(
            &mut a.ingress,
            &a.friends,
            &a.store,
            &session,
            &a.link,
            chat(9, 7, 1, true, 0),
            4,
            Some(WALL),
        )
        .unwrap()
        .unwrap();
    assert_eq!(raw, chat(9, 7, 1, true, 0));
    assert_eq!(b.receive(&raw, 4).unwrap().result, AcceptResult::Accepted);
    let raw = a
        .org
        .sign(
            &mut a.ingress,
            &a.friends,
            &a.store,
            &session,
            &a.link,
            chat(9, 7, 2, false, 0),
            1000,
            Some(WALL),
        )
        .unwrap()
        .unwrap();
    assert_eq!(
        b.receive(&raw, 1000).unwrap().result,
        AcceptResult::Accepted
    );
    session.invalidate();
    assert!(
        a.org
            .sign(
                &mut a.ingress,
                &a.friends,
                &a.store,
                &session,
                &a.link,
                chat(9, 7, 3, true, 0),
                2000,
                Some(WALL)
            )
            .is_err()
    );
    let mut schedule = OfferSchedule::default();
    assert!(schedule.due(0, false).unwrap());
    assert!(schedule.due(1, true).unwrap());
    assert!(!schedule.due(2, true).unwrap());
    assert!(schedule.due(300001, false).unwrap());
    assert!(schedule.due(1, false).is_err());
}

#[test]
fn openssl_reference_root_credentials_and_exact_staff_packets() {
    let vectors: std::collections::HashMap<_, _> =
        include_str!("../../vectors/crypto/organizer-v1.tsv")
            .lines()
            .map(|l| {
                let (n, h) = l.split_once('\t').unwrap();
                (n.to_string(), database::unhex(h))
            })
            .collect();
    check_binding_vectors(vectors);
}
pub fn check_binding_vectors(vectors: std::collections::HashMap<String, Vec<u8>>) {
    let UnconfirmedProposal::Event { bundle, .. } = root(9, 202000) else {
        panic!()
    };
    assert_eq!(bundle.as_slice(), vectors["root_bundle"]);
    assert_eq!(credential(), vectors["credential_a"]);
    assert_eq!(cred(9, 8, 199990, 201000), vectors["credential_b"]);
    let mut n = Node::new(3, true);
    for (i, name) in ["chat_a", "chat_b", "omitted_a", "pinned_a"]
        .into_iter()
        .enumerate()
    {
        assert_eq!(
            n.receive(&vectors[name], i as u64 * 1000 + 1)
                .unwrap()
                .result,
            AcceptResult::Accepted
        );
    }
    assert!(n.receive(&vectors["wrong_domain"], 5000).is_none());
    for (name, s, id, include, pin) in [
        ("chat_a", 7, 1, true, 0),
        ("chat_b", 8, 2, true, 0),
        ("omitted_a", 7, 3, false, 0),
        ("pinned_a", 7, 4, true, 200100),
    ] {
        assert_eq!(chat(9, s, id, include, pin), vectors[name]);
    }
}
#[test]
fn ambiguous_credentials_require_included_packet_and_expiry_is_strict() {
    let mut n = Node::new(3, true);
    let first = credential();
    let second = cred(9, 7, 199980, 201000);
    n.receive(&offer(&first, 1u64.to_be_bytes(), [2; 8]).unwrap(), 1);
    n.receive(&offer(&second, 2u64.to_be_bytes(), [2; 8]).unwrap(), 1000);
    let omitted = chat(9, 7, 3, false, 0);
    assert!(n.receive(&omitted, 2000).is_none());
    assert_eq!(
        n.receive(&chat(9, 7, 4, true, 0), 3000).unwrap().result,
        AcceptResult::Accepted
    );
    for (i, before, after) in [(5, 200001, 201000), (6, 199990, 202001)] {
        let replacement = cred(9, 7, before, after);
        let mut raw = chat(9, 7, i, true, 0);
        let offset = raw.len() - 64 - replacement.len();
        raw[offset..offset + replacement.len()].copy_from_slice(&replacement);
        resign(&mut raw, 7);
        let job = n.feed(&raw, i * 1000).unwrap().job.unwrap();
        assert!(n.finish(job, i * 1000).is_err());
    }
    let raw = chat(9, 7, 7, true, 201001);
    let job = n.feed(&raw, 7000).unwrap().job.unwrap();
    assert!(n.finish(job, 7000).is_err());
}
#[test]
fn full_ledger_refuses_organizer_effect_and_local_work_shares_slots() {
    let mut n = Node::new(3, true);
    let one = n.ingress.begin_local_work(1, 1).unwrap().unwrap();
    let two = n.ingress.begin_work(&n.link, 1, 1).unwrap().unwrap();
    assert!(n.ingress.begin_local_work(1, 1).unwrap().is_none());
    n.ingress.finish_work(one).unwrap();
    n.ingress.finish_work(two).unwrap();
    n.db.execute("WITH RECURSIVE seq(n) AS (SELECT 1 UNION ALL SELECT n+1 FROM seq WHERE n<100000) INSERT INTO ledger(subject,direction,logical_type,message_id,digest,timestamp) SELECT x'01',0,1,cast(printf('%08d',n) AS BLOB),zeroblob(32),200000 FROM seq".into(),vec![]).unwrap();
    let raw = chat(9, 7, 1, true, 0);
    let job = n.feed(&raw, 1000).unwrap().job.unwrap();
    assert!(n.finish(job, 1000).is_err());
    let rows =
        n.db.query("SELECT count(*) FROM history".into(), vec![], 1)
            .unwrap();
    assert!(matches!(rows[0].cells[0], SqlValue::Integer { value: 0 }));
}
#[test]
fn explicit_adoption_capacity_and_invalid_roots_fail_closed() {
    let mut n = Node::new(3, false);
    for i in 10..26 {
        n.adopt(i, u64::from(i) * 1000);
    }
    assert!(
        n.org
            .adopt(
                &mut n.ingress,
                &n.store,
                root(26, 202000),
                27000,
                Some(WALL)
            )
            .is_err()
    );
    let UnconfirmedProposal::Event {
        mut bundle,
        name,
        root_id,
    } = root(27, 202000)
    else {
        panic!()
    };
    bundle[100] ^= 1;
    assert!(
        n.org
            .adopt(
                &mut n.ingress,
                &n.store,
                UnconfirmedProposal::Event {
                    bundle,
                    name,
                    root_id
                },
                28000,
                Some(WALL)
            )
            .is_err()
    );
    let roots =
        n.db.query(
            "SELECT count(*) FROM records WHERE kind=3".into(),
            vec![],
            1,
        )
        .unwrap();
    assert!(matches!(roots[0].cells[0], SqlValue::Integer { value: 16 }));
}

#[test]
fn eight_links_fill_and_bound_opaque_credentials_at_128() {
    let mut n = Node::new(3, false);
    let mut links = vec![n.link.clone()];
    let own = n.identity.public_identity().unwrap();
    for generation in 2..=8 {
        let link = LinkHandle {
            instance_nonce: 3,
            generation,
        };
        let mut hello = n
            .friends
            .start_link(
                &mut n.ingress,
                link.clone(),
                Role::Central,
                (512, 512),
                0,
                || Ok([100 + generation as u8; 16]),
            )
            .unwrap();
        n.friends.hello_transmitted(&link, 0).unwrap();
        hello[1] = 1;
        hello[6..22].fill(150 + generation as u8);
        hello[22..].copy_from_slice(key(30 + generation as u8).verifying_key().as_bytes());
        let mut frame = vec![2, 0, 0, 55, 2];
        frame.extend_from_slice(&hello);
        n.friends
            .receive(&mut n.ingress, &link, frame.len() as u64, &frame, 0)
            .unwrap();
        links.push(link);
    }
    for i in 0..136 {
        n.link = links[i % 8].clone();
        let c = cred(9, (10 + i) as u8, 199990, 201000);
        assert!(
            n.receive(
                &offer(&c, (i as u64 + 1).to_be_bytes(), [1; 8]).unwrap(),
                (i / 8) as u64 * 30_000 + 1
            )
            .is_none()
        );
        assert!(n.org.cache_count() <= 128);
    }
    assert_eq!(n.org.cache_count(), 128);
    assert!(n.org.reserved_bytes() < 64 * 1024);
    n.org = Organizer::new(&own).unwrap();
    assert_eq!(n.org.cache_count(), 0); // opaque cache does not resurrect
}
#[test]
fn validated_credential_reuse_charges_one_message_verification() {
    let mut n = Node::new(3, true);
    let before = n.ingress.counters().reserved_work_units;
    let job = n.feed(&chat(9, 7, 1, true, 0), 1).unwrap().job.unwrap();
    assert_eq!(n.ingress.counters().reserved_work_units - before, 2);
    n.finish(job, 1).unwrap().unwrap();
    let before = n.ingress.counters().reserved_work_units;
    let job = n.feed(&chat(9, 7, 2, false, 0), 1000).unwrap().job.unwrap();
    assert_eq!(n.ingress.counters().reserved_work_units - before, 1);
    n.finish(job, 1000).unwrap().unwrap();
}

#[test]
fn synthetic_filtered_root_collision_bucket_never_selects_authority() {
    // Organizer root resolution uses this same pure helper after hint filtering.
    // Synthetic collisions avoid claiming to have found a real SHA-256 collision.
    use meshchat_core::friends::unique_signing_candidate;
    let a = key(9).verifying_key().to_bytes();
    let b = key(10).verifying_key().to_bytes();
    assert!(unique_signing_candidate([a, b].into_iter()).is_none());
    assert_eq!(unique_signing_candidate([a, a].into_iter()), Some(a));
    assert!(unique_signing_candidate(std::iter::empty()).is_none());
}
