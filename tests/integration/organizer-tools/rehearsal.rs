use super::database::Database;
use meshchat_core::{
    LinkHandle, codec, framing,
    friends::{Friends, Role},
    identity::IdentityKeySession,
    ingress::Ingress,
    links::{self, UnconfirmedProposal},
    organizer::*,
    storage::{AcceptResult, EncryptedStore, RecordKind},
};
use meshchat_organizer::*;
use sha2::{Digest, Sha256};
use std::sync::Arc;
const WALL: i64 = 200000;

fn post(credential: &[u8], sender: &[u8], id: u64) -> Vec<u8> {
    let c = codec::credential(credential).unwrap();
    let mut body = (WALL as u32).to_be_bytes().to_vec();
    body.extend_from_slice(&[0, 1, b'A', 0, 0, 0, 0, 0, 5]);
    body.extend_from_slice(b"hello");
    body.extend_from_slice(&[0; 5]);
    body.extend_from_slice(c.root_id);
    body.extend_from_slice(&Sha256::digest(c.staff_public_key)[..8]);
    body.push(1);
    body.extend_from_slice(&(credential.len() as u16).to_be_bytes());
    body.extend_from_slice(credential);
    body.extend_from_slice(&[0; 64]);
    let mut raw = vec![1, 1, 6, 7];
    raw.extend_from_slice(&id.to_be_bytes());
    raw.extend_from_slice(sender);
    raw.extend_from_slice(&codec::EVENT_CHANNEL);
    raw.extend_from_slice(&(body.len() as u16).to_be_bytes());
    raw.extend_from_slice(&body);
    raw
}

#[test]
fn offline_two_staff_updates_cross_a_nonadopting_relay() {
    use meshchat_core::{
        power::Platform,
        relay::{Relay, Request, Traffic},
    };
    let root = create_root("Offline rehearsal", 202000, 201000, WALL as u32).unwrap();
    let opened = OpenRoot::open(&root.vault, &root.unlock, WALL as u32).unwrap();
    let mut receiver = Node::new(5, Some(&root.event));
    let mut bridge = Node::new(4, None);
    let mut relay = Relay::new(4, Platform::Android, false, 0).unwrap();
    let destination = LinkHandle {
        instance_nonce: 4,
        generation: 2,
    };
    relay.register(&bridge.link, 512, 0).unwrap();
    relay.register(&destination, 512, 0).unwrap();
    for (index, label) in ["Stage A", "Stage B"].into_iter().enumerate() {
        let mut staff = Node::new(index as u8 + 1, Some(&root.event));
        let uri = opened.issue(label, 199990, 201000, WALL as u32).unwrap();
        let UnconfirmedProposal::Staff(proposal) = links::parse(&uri).unwrap() else {
            panic!()
        };
        let session = staff
            .org
            .import_staff(&mut staff.ingress, &staff.store, proposal, 1000, Some(WALL))
            .unwrap()
            .unwrap();
        let raw = post(
            session.credential(),
            &staff.identity.public_identity().unwrap().sender_id,
            index as u64 + 1,
        );
        let raw = staff
            .org
            .sign(
                &mut staff.ingress,
                &staff.friends,
                &staff.store,
                &session,
                &staff.link,
                raw,
                2000,
                Some(WALL),
            )
            .unwrap()
            .unwrap();
        session.invalidate();
        let time = 10000 + index as u64 * 10000;
        let intake = bridge.feed(&raw, time).unwrap();
        assert!(intake.job.is_none());
        assert!(matches!(
            intake.outcome,
            meshchat_core::ingress::Outcome::Complete { .. }
        ));
        assert!(relay.observe(Some(&bridge.link), &raw, time).unwrap());
        assert!(
            relay
                .enqueue(
                    &destination,
                    &raw,
                    Request {
                        traffic: Traffic::Forwarded,
                        cookie: index as u64,
                        random: 0
                    },
                    time,
                    &mut |_| {}
                )
                .unwrap()
                .is_some()
        );
        let mut accepted = None;
        for tick in time + 500..time + 520 {
            let mut frame = [0; 512];
            if let Some(sent) = relay.poll(tick, &mut frame, &mut |_| {}).unwrap() {
                let incoming = receiver
                    .org
                    .receive(
                        &mut receiver.ingress,
                        &receiver.friends,
                        &receiver.store,
                        &receiver.link,
                        sent.len as u64,
                        &frame[..sent.len],
                        tick,
                        Some(WALL),
                    )
                    .unwrap();
                if let Some(job) = incoming.job {
                    accepted = receiver
                        .org
                        .complete(
                            &mut receiver.ingress,
                            &receiver.store,
                            job,
                            tick,
                            Some(WALL),
                        )
                        .unwrap();
                }
                relay
                    .complete(sent.attempt, true, tick, &mut |_| {})
                    .unwrap();
            }
        }
        let accepted =
            accepted.expect("adopted receiver verifies staff update after ordinary relay");
        assert_eq!(accepted.result, AcceptResult::Accepted);
        assert_eq!(accepted.authority.label(), label);
        assert_eq!(
            receiver
                .org
                .current(&receiver.store, &accepted.authority, Some(WALL))
                .unwrap(),
            (true, false)
        );
    }
    // Public APIs are authoritative; the relay has no adopted root or staff key.
    assert_eq!(bridge.org.cache_count(), 0);
    assert!(
        bridge
            .store
            .history(codec::EVENT_CHANNEL.to_vec(), false, 10)
            .unwrap()
            .is_empty()
    );
    let UnconfirmedProposal::Event { bundle, .. } = links::parse(&root.event).unwrap() else {
        panic!()
    };
    assert!(
        bridge
            .store
            .get_record(RecordKind::EventRoot, bundle[1..33].to_vec())
            .unwrap()
            .is_none()
    );
}
struct Node {
    store: Arc<EncryptedStore>,
    identity: Arc<IdentityKeySession>,
    org: Organizer,
    friends: Friends,
    ingress: Ingress,
    link: LinkHandle,
}
impl Node {
    fn new(n: u8, event: Option<&str>) -> Self {
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
            store,
            identity,
            org,
            friends,
            ingress,
            link,
        };
        if let Some(uri) = event {
            node.adopt(uri, 0);
        }
        node
    }
    fn adopt(&mut self, event: &str, now: u64) {
        assert!(
            self.org
                .adopt(
                    &mut self.ingress,
                    &self.store,
                    links::parse(event).unwrap(),
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
}
