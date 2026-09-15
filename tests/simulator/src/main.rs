//! JSON-line adapter for synthetic simulator fixtures. No alternate wire codec.
use meshchat_core::{
    Capacities, Core, DriverEvent, Limits, LinkHandle, PowerState, SendPath, UiEvent,
    codec::{self, Context},
    framing::{Encoder, MAX_CAPACITY, MIN_CAPACITY, ObjectKind, Reassembler},
    ingress::{Ingress, Outcome},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::{self, BufRead, Write};

struct Node {
    core: Core,
    receiver: Receiver,
    handles: BTreeMap<u64, LinkHandle>,
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
fn command(nodes: &mut Vec<Node>, v: Value) -> Value {
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
            result
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
