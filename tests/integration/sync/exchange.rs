use meshchat_core::{
    LinkHandle,
    framing::ObjectKind,
    ingress::{Ingress, Outcome, State},
    power::{Mode, Platform},
    relay::{Relay, Request as SendRequest, ResultEvent, Send, Status, Traffic},
    sync::{Cache, CacheState, session::*},
};
fn fixture(name: &str, id: u64) -> Vec<u8> {
    let line = include_str!("../../vectors/base/logical.txt")
        .lines()
        .find(|l| l.split_whitespace().next() == Some(name))
        .unwrap();
    let mut b: Vec<u8> = line
        .split_whitespace()
        .nth(3)
        .unwrap()
        .as_bytes()
        .chunks_exact(2)
        .map(|x| u8::from_str_radix(std::str::from_utf8(x).unwrap(), 16).unwrap())
        .collect();
    b[4..12].copy_from_slice(&id.to_be_bytes());
    if matches!(name, "max-chat" | "encrypted-chat-361") {
        b[12..20].copy_from_slice(&id.to_be_bytes());
    }
    b
}
struct Endpoint {
    link: LinkHandle,
    cache: Cache,
    ingress: Ingress,
    relay: Relay,
    sessions: Sessions,
    events: Vec<Event>,
    messages: Vec<(Vec<u8>, State)>,
    request: Option<(u64, RequestToken)>,
    served: Option<(u64, ServeToken)>,
    cookie: u64,
    first: Option<u64>,
    finished: Option<u64>,
    last_send: Option<u64>,
    frames: u64,
    bytes: u64,
}
impl Endpoint {
    fn new(instance: u64, capacity: usize, name: &str) -> Self {
        let link = LinkHandle {
            instance_nonce: instance,
            generation: 1,
        };
        let mut ingress = Ingress::new(instance, 1, 0).unwrap();
        ingress.register(&link, capacity, 0).unwrap();
        let mut relay = Relay::new(instance, Platform::Android, false, 0).unwrap();
        relay.register(&link, capacity, 0).unwrap();
        let mut sessions = Sessions::new(instance, 0).unwrap();
        sessions.register(&link, 0).unwrap();
        let mut cache = Cache::new(Mode::Normal, 0).unwrap();
        for n in 1..=8 {
            let mut raw = fixture(name, instance * 100 + n);
            raw[3] = (n % 8) as u8;
            let state = if raw[2] & 7 == 0 {
                CacheState::Unverified
            } else {
                CacheState::Pending
            };
            // Pre-admitted source cache is the scenario's explicit initial state.
            cache.insert_admitted(&raw, state, 0).unwrap();
        }
        Self {
            link,
            cache,
            ingress,
            relay,
            sessions,
            events: vec![],
            messages: vec![],
            request: None,
            served: None,
            cookie: 1000,
            first: None,
            finished: None,
            last_send: None,
            frames: 0,
            bytes: 0,
        }
    }
    fn identity(&mut self) -> Identity {
        self.cookie += 1;
        Identity {
            message_id: (self.link.instance_nonce * 10000 + self.cookie).to_be_bytes(),
            sender_id: self.link.instance_nonce.to_be_bytes(),
        }
    }
    fn results(&mut self, results: Vec<ResultEvent>, now: u64) {
        for e in results {
            if self.served.is_some_and(|(cookie, _)| cookie == e.cookie) {
                let (_, token) = self.served.take().unwrap();
                self.sessions
                    .served_complete(token, e.status == Status::NativeComplete, now, &mut |e| {
                        self.events.push(e)
                    })
                    .unwrap();
            }
            assert_eq!(
                e.status,
                Status::NativeComplete,
                "native object failure at {now}"
            );
        }
    }
    fn queue(&mut self, raw: &[u8], traffic: Traffic, now: u64) -> u64 {
        self.cookie += 1;
        let mut results = vec![];
        assert!(
            self.relay
                .enqueue(
                    &self.link,
                    raw,
                    SendRequest {
                        traffic,
                        cookie: self.cookie,
                        random: 0
                    },
                    now,
                    &mut |e| results.push(e)
                )
                .unwrap()
                .is_some()
        );
        self.results(results, now);
        self.cookie
    }
    fn begin(&mut self, now: u64) {
        let identity = self.identity();
        let mut raw = [0; 546];
        let r = self
            .sessions
            .request(
                &self.link,
                Request {
                    identity,
                    held_count: 0,
                    filter: [0; 512],
                },
                &mut self.ingress,
                now,
                &mut raw,
            )
            .unwrap();
        let cookie = self.queue(&raw[..r.len], Traffic::Local, now);
        self.request = Some((cookie, r.token));
    }
    fn receive(&mut self, frame: &[u8], now: u64) {
        let mut raw = [0; 1035];
        let admitted = self
            .ingress
            .receive_deferred_sync(&self.link, frame.len() as u64, frame, now, &mut raw)
            .unwrap();
        match admitted.outcome {
            Outcome::Incomplete => (),
            Outcome::Dropped(d) => panic!("ingress drop {d:?} at {now}"),
            Outcome::Complete { kind, state, .. } => {
                if state == State::DeferredSync {
                    let r = self
                        .sessions
                        .receive_admitted(
                            &self.link,
                            admitted.sync.unwrap(),
                            &mut self.ingress,
                            now,
                            &mut Sink {
                                events: &mut |e| self.events.push(e),
                                messages: &mut |b, s| self.messages.push((b.to_vec(), s)),
                            },
                        )
                        .unwrap_or_else(|e| {
                            panic!(
                                "response {e:?} at {now} events={:?} counters={:?}",
                                self.events,
                                self.ingress.counters()
                            )
                        });
                    if matches!(r, Received::Page { .. }) {
                        let identity = self.identity();
                        let mut out = [0; 546];
                        let r = self
                            .sessions
                            .continuation(&self.link, identity, now, &mut out)
                            .unwrap();
                        let cookie = self.queue(&out[..r.len], Traffic::Local, now);
                        self.request = Some((cookie, r.token));
                    } else if matches!(r, Received::Complete { .. }) {
                        self.finished = Some(now);
                    }
                } else if kind == ObjectKind::Logical && admitted.sync.is_some() {
                    self.sessions
                        .accept_admitted_request(
                            &self.link,
                            admitted.sync.unwrap(),
                            &mut self.cache,
                            now,
                            &mut |e| self.events.push(e),
                        )
                        .unwrap();
                }
            }
        }
    }
    fn tick(&mut self, now: u64) -> Option<(Send, Vec<u8>)> {
        self.sessions
            .advance(now, &mut |e| self.events.push(e))
            .unwrap();
        if self.sessions.reservations().serving > 0 && self.served.is_none() {
            let mut raw = [0; 1035];
            if let Some(s) = self
                .sessions
                .next_served(&self.link, &mut self.cache, now, &mut raw, &mut |e| {
                    self.events.push(e)
                })
                .unwrap()
            {
                let cookie = self.queue(&raw[..s.len], Traffic::Transport(1), now);
                self.served = Some((cookie, s.token));
            }
        }
        let mut raw = [0; 512];
        let mut results = vec![];
        let sent = self
            .relay
            .poll(now, &mut raw, &mut |e| results.push(e))
            .unwrap();
        self.results(results, now);
        sent.map(|s| {
            if let Some((cookie, token)) = self.request {
                if cookie == s.cookie {
                    self.sessions.request_started(token, now).unwrap();
                    self.first.get_or_insert(now);
                }
            }
            if let Some(last) = self.last_send {
                assert!(now - last >= 1000);
            }
            self.last_send = Some(now);
            self.frames += 1;
            self.bytes += s.len as u64;
            let bytes = raw[..s.len].to_vec();
            (s, bytes)
        })
    }
    fn complete(&mut self, send: Send, now: u64) {
        let mut results = vec![];
        self.relay
            .complete(send.attempt, true, now, &mut |e| results.push(e))
            .unwrap();
        self.results(results, now);
    }
}
#[test]
fn simultaneous_paced_recent_context_at_all_capacities() {
    for capacity in [146, 182, 512] {
        for name in ["max-chat", "max-organizer", "encrypted-chat-361"] {
            let mut nodes = [
                Endpoint::new(1, capacity, name),
                Endpoint::new(2, capacity, name),
            ];
            let mut pending: [Option<(Send, Vec<u8>)>; 2] = [None, None];
            // MC-007 assumes links admitted before traffic. Reserve 9 seconds/1227
            // bytes per direction for HELLO + eight proof frames; not proof evidence.
            let start = 9000;
            for now in (start..=start + 120000).step_by(20) {
                for side in 0..2 {
                    if let Some((send, raw)) = pending[side].take() {
                        nodes[side].complete(send, now);
                        nodes[1 - side].receive(&raw, now);
                    }
                }
                for node in &mut nodes {
                    if now == start {
                        node.begin(now);
                    }
                    let elapsed = now - start;
                    if elapsed < 120000 && elapsed % 30000 == 0 {
                        let id = u64::from_be_bytes(node.identity().message_id);
                        node.queue(&fixture("max-announce", id), Traffic::Local, now);
                    }
                    if elapsed < 120000 && elapsed % 10000 == 0 {
                        let id = u64::from_be_bytes(node.identity().message_id);
                        node.queue(&fixture("reaction", id), Traffic::Own, now);
                    }
                }
                for side in 0..2 {
                    pending[side] = nodes[side].tick(now);
                }
            }
            for (side, node) in nodes.iter().enumerate() {
                let elapsed = node.finished.expect("terminal marker missing") - node.first.unwrap();
                assert!(elapsed <= 120000);
                assert_eq!(node.messages.len(), 8);
                for (index, (raw, state)) in node.messages.iter().enumerate() {
                    let mut expected = fixture(name, (2 - side) as u64 * 100 + 8 - index as u64);
                    expected[3] = ((8 - index) % 8) as u8;
                    assert_eq!(raw, &expected);
                    assert_eq!(
                        *state,
                        if name == "max-chat" {
                            State::Unverified
                        } else {
                            State::Pending
                        }
                    );
                }
                assert_eq!(node.events.len(), 2);
                assert!(
                    node.events
                        .iter()
                        .all(|e| e.end == End::Complete && e.items == 8)
                );
                assert_eq!(node.ingress.counters().budget_drops, 0);
                assert_eq!(
                    node.ingress.counters().admitted_frames,
                    nodes[1 - side].frames
                );
                assert!(node.sessions.reservations().allocated_bytes <= 256 * 1024);
                println!(
                    "SYNC capacity={capacity} fixture={name} side={side} elapsed_ms={elapsed} frames={} bytes={} setup_reserved_frames=9 setup_reserved_bytes=1227",
                    node.frames, node.bytes
                );
            }
        }
    }
}
