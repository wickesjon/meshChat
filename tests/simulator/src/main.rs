//! JSON-line adapter for synthetic simulator fixtures. No alternate wire codec.
use meshchat_core::{
    Capacities, Core, DriverEvent, Limits, LinkHandle, PowerState, SendPath, UiEvent,
    codec::{self, Context},
    framing::{Encoder, MAX_CAPACITY, MIN_CAPACITY, ObjectKind, Reassembler},
    ingress::{Ingress, Outcome},
    power::{Mode, Platform},
    relay::{Attempt, Kind, Relay, Request, ResultEvent, Traffic},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::{self, BufRead, Write};

struct Node {
    core: Core,
    receiver: Receiver,
    handles: BTreeMap<u64, LinkHandle>,
    relay: Option<Relay>,
    attempts: BTreeMap<u64, (Attempt, u64, u64)>,
    next_attempt: u64,
    events: Vec<Value>,
}
enum Receiver {
    Framing(Box<Reassembler>),
    Ingress(Box<Ingress>),
}
impl Receiver {
    fn advance(&mut self, now: u64) {
        match self {
            Self::Framing(r) => r.advance(now).unwrap(),
            Self::Ingress(r) => r.advance(now).unwrap(),
        }
    }
    fn register(&mut self, link: &LinkHandle, capacity: usize, now: u64) {
        match self {
            Self::Framing(r) => r.register(link, capacity).unwrap(),
            Self::Ingress(r) => r.register(link, capacity, now).unwrap(),
        }
    }
    fn disconnect(&mut self, link: &LinkHandle, now: u64) {
        match self {
            Self::Framing(r) => r.disconnect(link).unwrap(),
            Self::Ingress(r) => r.disconnect(link, now).unwrap(),
        }
    }
}
fn number(v: &Value, key: &str) -> u64 {
    v[key].as_u64().expect("integer command field")
}
fn bytes(v: &Value) -> Vec<u8> {
    let s = v.as_str().expect("hex string");
    assert!(s.is_ascii() && s.len() % 2 == 0);
    s.as_bytes()
        .chunks_exact(2)
        .map(|b| u8::from_str_radix(std::str::from_utf8(b).unwrap(), 16).unwrap())
        .collect()
}
fn hex(b: &[u8]) -> String {
    b.iter()
        .fold(String::with_capacity(b.len() * 2), |mut s, x| {
            write!(s, "{x:02x}").unwrap();
            s
        })
}
fn result_event(e: ResultEvent) -> Value {
    json!({"handle":e.link.generation,"object":e.cookie,"category":category(e.kind),"status":format!("{:?}",e.status)})
}
fn category(k: Kind) -> &'static str {
    match k {
        Kind::Control | Kind::Announce => "control",
        Kind::Own => "origin",
        Kind::Chat => "relay",
        Kind::Reaction => "reaction",
        Kind::Sync => "sync",
    }
}
fn command(nodes: &mut Vec<Node>, v: Value) -> Value {
    let node = v["node"].as_u64();
    let mut result = inner_command(nodes, v);
    if let Some(node) = node {
        let n = &mut nodes[node as usize];
        for event in &n.events {
            let cookie = number(event, "object");
            let handle = number(event, "handle");
            n.attempts
                .retain(|_, (_, c, h)| *c != cookie || *h != handle);
        }
        if !n.events.is_empty() {
            result["events"] = json!(std::mem::take(&mut n.events));
        }
    }
    result
}
fn inner_command(nodes: &mut Vec<Node>, v: Value) -> Value {
    let op = v["op"].as_str().expect("operation");
    if op == "reset" {
        let count = number(&v, "nodes");
        assert!((1..=64).contains(&count));
        *nodes = (1..=count)
            .map(|nonce| Node {
                core: Core::new(
                    Limits {
                        max_links: 8,
                        max_value_bytes: 512,
                    },
                    nonce,
                    0,
                )
                .unwrap(),
                receiver: if v["ingress"].as_bool().unwrap_or(false) {
                    Receiver::Ingress(Box::new(Ingress::new(nonce, 8, 0).unwrap()))
                } else {
                    Receiver::Framing(Box::new(Reassembler::new(nonce, 8, 0).unwrap()))
                },
                handles: BTreeMap::new(),
                relay: if v["relay"].as_bool().unwrap_or(false) {
                    Some(
                        Relay::new(
                            nonce,
                            Platform::Android,
                            v["suppression"].as_bool().unwrap_or(true),
                            0,
                        )
                        .unwrap(),
                    )
                } else {
                    None
                },
                attempts: BTreeMap::new(),
                next_attempt: 0,
                events: Vec::new(),
            })
            .collect();
        return json!({"ok": true});
    }
    if op == "encode" {
        let body = bytes(&v["body"]);
        let capacity = number(&v, "capacity") as usize;
        let encoded = if v["transport"].is_null() {
            Encoder::logical(&body, capacity, 1)
        } else {
            Encoder::transport(number(&v, "transport") as u8, &body, capacity, 1)
        };
        return match encoded {
            Err(e) => json!({"error": e.to_string()}),
            Ok(encoder) => {
                let frames: Vec<_> = (0..encoder.frame_count())
                    .map(|index| {
                        let mut output = [0; 512];
                        let len = encoder.frame(index, &mut output).unwrap();
                        hex(&output[..len])
                    })
                    .collect();
                json!({"frames": frames})
            }
        };
    }
    let n = &mut nodes[number(&v, "node") as usize];
    let now = number(&v, "now");
    n.core
        .handle_event(DriverEvent::TimeAdvanced { monotonic_ms: now })
        .unwrap();
    // Receiving uses the admission fast path before table maintenance/reassembly.
    if op != "receive" {
        n.receiver.advance(now);
    }
    match op {
        "connect" => {
            let send = number(&v, "send") as u16;
            let receive = number(&v, "receive") as u16;
            // Protocol capacity refusal is exercised by the actual framing encoder/register.
            if !(MIN_CAPACITY..=MAX_CAPACITY).contains(&usize::from(send))
                || !(MIN_CAPACITY..=MAX_CAPACITY).contains(&usize::from(receive))
            {
                return json!({"error": "unsupported capacity"});
            }
            match n.core.connected(Capacities {
                write_bytes: send,
                notify_bytes: send,
                receive_bytes: receive,
            }) {
                Err(e) => json!({"error": e.to_string()}),
                Ok(effects) => {
                    let UiEvent::LinkConnected { link } = &effects.ui_events[0] else {
                        unreachable!()
                    };
                    if let Some(r) = &mut n.relay {
                        if let Err(e) = r.register(link, send as usize, now) {
                            n.core
                                .handle_event(DriverEvent::Disconnected { link: link.clone() })
                                .unwrap();
                            return json!({"error":e.to_string()});
                        }
                    }
                    n.receiver.register(link, receive as usize, now);
                    n.handles.insert(link.generation, link.clone());
                    json!({"handle": link.generation})
                }
            }
        }
        "power" => {
            let state = match v["state"].as_str().unwrap() {
                "saver" => PowerState::LowPower,
                "background" => PowerState::Background,
                "normal" | "beacon" => PowerState::Foreground,
                _ => panic!("unsupported fixture power state"),
            };
            if let Some(r) = &mut n.relay {
                let (mode, tier) = match v["state"].as_str().unwrap() {
                    "saver" => (Mode::Saver, 0),
                    "beacon" => (Mode::Beacon, 3),
                    "background" => (Mode::Normal, 3),
                    _ => (Mode::Normal, 1),
                };
                if let Err(e) = r.set_power(mode, tier, now) {
                    return json!({"error":e.to_string()});
                }
            }
            n.core
                .handle_event(DriverEvent::PowerChanged { state })
                .unwrap();
            json!({"ok": true})
        }
        "stats" => {
            let (r, counters, ingress) = match &n.receiver {
                Receiver::Framing(r) => (r.reservations(), r.counters(), Value::Null),
                Receiver::Ingress(i) => {
                    let r = i.reservations();
                    let c = i.counters();
                    (
                        r.reassembly,
                        i.reassembly_counters(),
                        json!({"offered_frames":c.offered_frames,"admitted_frames":c.admitted_frames,
                        "budget_drops":c.budget_drops,"pending":r.pending,"accepted":r.accepted,"senders":r.senders,
                        "sender_peak":c.peak_senders,"accepted_peak":c.peak_accepted,"pending_peak":c.peak_pending,
                        "reserved_work_units":c.reserved_work_units,"sender_reserved_bytes":r.sender_bytes,
                        "pending_reserved_bytes":r.pending_bytes,"accepted_reserved_bytes":r.accepted_bytes,
                        "staging_reserved_bytes":r.staging_bytes,"rejected_reserved_bytes":r.rejected_bytes,"address_reserved_bytes":r.address_bytes,"manager_bytes":r.manager_bytes}),
                    )
                }
            };
            let mut result = json!({"reserved_bytes": r.logical_bytes + r.transport_bytes + r.rejected_bytes + r.manager_bytes,
                "peak_logical_groups": r.peak_logical_groups, "peak_transport_groups": r.peak_transport_groups,
                "peak_rejected_groups": r.peak_rejected_groups, "malformed": counters.malformed});
            if !ingress.is_null() {
                result["ingress"] = ingress;
            }
            if let Some(relay) = &n.relay {
                let r = relay.reservations();
                let c = relay.counters();
                result["relay"] = json!({"objects":r.objects,"outbound_bytes":r.outbound_bytes,"outbound_bytes_per_link":r.outbound_bytes_per_link,"recent_bytes":r.recent_bytes,"target_bytes":r.target_bytes,"manager_bytes":r.manager_bytes,"links_bytes":r.links_bytes,"attempts":c.attempts,"bytes":c.bytes,"retries":c.retries,"forwarded_attempts":c.forwarded_attempts,"suppressed":c.suppressed,"failures":c.failures,"refused":c.refused,"peak_node_objects":c.peak_node_objects,"peak_link_objects":c.peak_link_objects,"peak_recent":c.peak_recent,"attempts_by_tier":c.attempts_by_tier,"forwarded_by_mode":c.forwarded_by_mode});
            }
            result
        }
        "relay_queue" => {
            let h = n.handles.get(&number(&v, "handle")).unwrap();
            let body = bytes(&v["body"]);
            let traffic = match v["category"].as_str().unwrap() {
                "origin" => Traffic::Own,
                "relay" | "reaction" => Traffic::Forwarded,
                _ => Traffic::Local,
            };
            let r = n.relay.as_mut().unwrap();
            if traffic == Traffic::Own {
                let _ = r.observe(None, &body, now);
            }
            match r.enqueue(
                h,
                &body,
                Request {
                    traffic,
                    cookie: number(&v, "object"),
                    random: number(&v, "random"),
                },
                now,
                &mut |e| n.events.push(result_event(e)),
            ) {
                Ok(serial) => json!({"queued":serial.is_some(),"wake":r.next_wake(now)}),
                Err(e) => json!({"error":e.to_string()}),
            }
        }
        "relay_ready" => {
            let h = n.handles.get(&number(&v, "handle")).unwrap();
            n.relay
                .as_mut()
                .unwrap()
                .ready(h, v["ready"].as_bool().unwrap(), now)
                .unwrap();
            json!({"ok":true})
        }
        "relay_digest" => json!({"digest":hex(&n.relay.as_mut().unwrap().digest(now).unwrap())}),
        "relay_poll" => {
            let mut out = [0; 512];
            let r = n.relay.as_mut().unwrap();
            let send = r
                .poll(now, &mut out, &mut |e| n.events.push(result_event(e)))
                .unwrap();
            let wake = r.next_wake(now);
            if let Some(s) = send {
                n.next_attempt += 1;
                n.attempts
                    .insert(n.next_attempt, (s.attempt, s.cookie, s.link.generation));
                let effects = n
                    .core
                    .prepare_send(s.link.clone(), SendPath::Write, out[..s.len].to_vec())
                    .unwrap();
                assert_eq!(effects.sends[0].bytes, &out[..s.len]);
                json!({"send":{"token":n.next_attempt,"handle":s.link.generation,"object":s.cookie,"category":category(s.kind),"frame":hex(&out[..s.len]),"fragment":s.fragment,"frames":s.frames,"retry":s.retry,"ttl":s.ttl},"wake":wake})
            } else {
                json!({"wake":wake})
            }
        }
        "relay_complete" => {
            let Some((attempt, _, _)) = n.attempts.remove(&number(&v, "token")) else {
                return json!({"error":"stale completion"});
            };
            let r = n.relay.as_mut().unwrap();
            match r.complete(attempt, v["success"].as_bool().unwrap(), now, &mut |e| {
                n.events.push(result_event(e))
            }) {
                Ok(()) => json!({"ok":true,"wake":r.next_wake(now)}),
                Err(e) => json!({"error":e.to_string(),"wake":r.next_wake(now)}),
            }
        }
        "disconnect" | "send" | "receive" => {
            let generation = number(&v, "handle");
            let Some(link) = n.handles.get(&generation).cloned() else {
                return json!({"error": "stale handle"});
            };
            match op {
                "disconnect" => {
                    n.core
                        .handle_event(DriverEvent::Disconnected { link: link.clone() })
                        .unwrap();
                    n.receiver.disconnect(&link, now);
                    if let Some(r) = &mut n.relay {
                        r.disconnect(&link, now, &mut |e| n.events.push(result_event(e)))
                            .unwrap();
                    }
                    n.attempts.retain(|_, (_, _, h)| *h != generation);
                    n.handles.remove(&generation);
                    json!({"ok": true})
                }
                "send" => match n
                    .core
                    .prepare_send(link, SendPath::Write, bytes(&v["frame"]))
                {
                    Ok(effects) => json!({"frame": hex(&effects.sends[0].bytes)}),
                    Err(e) => json!({"error": e.to_string()}),
                },
                "receive" => {
                    let frame = bytes(&v["frame"]);
                    let mut output = [0; 1035];
                    let (done, admission) = match &mut n.receiver {
                        Receiver::Framing(r) => (
                            r.ingest_admitted(&link, &frame, now, &mut output)
                                .map_err(|e| e.to_string()),
                            Value::Null,
                        ),
                        Receiver::Ingress(i) => {
                            match i.receive(&link, frame.len() as u64, &frame, now, &mut output) {
                                Err(e) => (Err(e.to_string()), Value::Null),
                                Ok(Outcome::Dropped(reason)) => {
                                    return json!({"error":format!("ingress {reason:?}"),"ingress_drop":true});
                                }
                                Ok(Outcome::Incomplete) => (Ok(None), Value::Null),
                                Ok(Outcome::Complete { kind, len, state }) => (
                                    Ok(Some(meshchat_core::framing::Completed { kind, len })),
                                    json!(format!("{state:?}")),
                                ),
                            }
                        }
                    };
                    if let Err(e) = n.core.handle_event(DriverEvent::InboundBytes {
                        link: link.clone(),
                        bytes: frame.clone(),
                    }) {
                        return json!({"error": e.to_string()});
                    }
                    match done {
                        Err(e) => json!({"error": e}),
                        Ok(None) => json!({"complete": false}),
                        Ok(Some(done)) => {
                            let body = &output[..done.len];
                            if done.kind != ObjectKind::Logical {
                                return json!({"complete": true, "transport": true, "body": hex(body)});
                            }
                            match codec::parse(body, Context::Live) {
                                Err(e) => json!({"error": e.to_string()}),
                                Ok(packet) => {
                                    let mut forwarded = [0; 1024];
                                    let next = packet
                                        .forward_to(&mut forwarded)
                                        .unwrap()
                                        .map(|len| hex(&forwarded[..len]));
                                    let mut result = json!({"complete": true, "body": hex(body), "forward": next,
                                        "id": hex(&packet.header().message_id), "ttl": packet.header().ttl});
                                    if let Some(r) = &mut n.relay {
                                        let fresh = r.observe(Some(&link), body, now).unwrap();
                                        result["fresh_relay"] = json!(
                                            fresh
                                                && admission != "Duplicate"
                                                && admission != "PendingDuplicate"
                                        );
                                    }
                                    if !admission.is_null() {
                                        result["admission_state"] = admission;
                                    }
                                    result
                                }
                            }
                        }
                    }
                }
                _ => unreachable!(),
            }
        }
        _ => panic!("unsupported fixture operation"),
    }
}
fn main() {
    let mut nodes = Vec::new();
    let mut out = io::BufWriter::new(io::stdout().lock());
    for line in io::stdin().lock().lines() {
        let request: Value = serde_json::from_str(&line.unwrap()).unwrap();
        let response = command(&mut nodes, request);
        serde_json::to_writer(&mut out, &response).unwrap();
        writeln!(out).unwrap();
        out.flush().unwrap();
    }
}
