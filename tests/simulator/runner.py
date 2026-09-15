"""Deterministic directed-GATT driver of the real core, with a synthetic flood policy.

Fixture policy results validate this harness, NOT MC-013/014/015 production gates.
Only exact, predeclared unsigned CHAT fixtures count toward synthetic delivery.
"""
from __future__ import annotations

import argparse
from collections import Counter, deque
from dataclasses import dataclass, field
from decimal import Decimal
import hashlib
import heapq
import json
from pathlib import Path
import struct
import subprocess

ROOT = Path(__file__).resolve().parents[2]
MANIFEST = Path(__file__).parent / "scenarios/MC-007-acceptance.json"
QUANTA = (8, 16, 8, 4, 8)
CLASSES = ("control", "origin", "relay", "reaction", "sync")


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True)


def random_word(seed, domain, source, destination, obj, fragment=0, attempt=0):
    domain = domain.encode("ascii")
    encoded = (struct.pack(">QH", seed, len(domain)) + domain
               + struct.pack(">IIQHI", source, destination, obj, fragment, attempt))
    return int.from_bytes(hashlib.sha256(encoded).digest()[:8], "big")


class CoreProcess:
    """One persistent native process; JSON is only the test-control protocol."""
    def __init__(self, executable):
        self.process = subprocess.Popen([str(executable)], stdin=subprocess.PIPE,
                                        stdout=subprocess.PIPE, text=True, encoding="utf-8")

    def call(self, op, **fields):
        self.process.stdin.write(canonical(dict(op=op, **fields)) + "\n")
        self.process.stdin.flush()
        line = self.process.stdout.readline()
        if not line:
            raise RuntimeError(f"core adapter exited during {op}")
        return json.loads(line)

    def close(self):
        self.process.stdin.close()
        status = self.process.wait(timeout=10)
        self.process.stdout.close()
        if status:
            raise RuntimeError(f"core adapter exit status {status}")


class Bucket:
    """Exact integer thirds-of-a-millisecond credit; reconnect never resets node buckets."""
    def __init__(self, capacity, refill_per_three_seconds):
        self.capacity = capacity * 3000
        self.credit = self.capacity
        self.refill = refill_per_three_seconds
        self.now = 0

    def advance(self, now):
        assert now >= self.now
        self.credit = min(self.capacity, self.credit + (now - self.now) * self.refill)
        self.now = now

    def configure(self, now, capacity, refill):
        self.advance(now)
        self.capacity, self.refill = capacity * 3000, refill
        self.credit = min(self.credit, self.capacity)


def charge(now, costs):
    for bucket, _ in costs:
        bucket.advance(now)
    if any(bucket.credit < amount * 3000 for bucket, amount in costs):
        return False
    for bucket, amount in costs:
        bucket.credit -= amount * 3000
    return True


def traffic_body(seed, source, index, identity=0, version=1, wall_ms=0):
    msg_id = random_word(seed, "traffic", source, 0, index).to_bytes(8, "big")
    sender = random_word(seed, "traffic", source, identity, index + (1 << 32)).to_bytes(8, "big")
    payload = (struct.pack(">I", max(0, wall_ms // 1000)) + b"\0\x14" + b"n" * 20
               + bytes(4) + struct.pack(">H", 280) + b"x" * 280)
    return bytes([version, 1, 0, 7]) + msg_id + sender + bytes.fromhex("bcf0eae3") + struct.pack(">H", len(payload)) + payload


def announce_body(node, index):
    # Unsigned 316-byte ANNOUNCE: no signature/proof or authenticated digest claim.
    payload = bytes(7) + b"\x14" + b"n" * 20 + bytes(4) + struct.pack(">H", 256) + bytes(256)
    return bytes([1, 2, 0, 1]) + struct.pack(">Q", (1 << 63) + index) + struct.pack(">Q", node + 1) + bytes(4) + struct.pack(">H", len(payload)) + payload


def immutable(body):
    return body[:3] + body[4:]


@dataclass
class Job:
    obj: int
    body: bytes
    category: str
    frames: list[str]
    queued: int
    ready: int
    index: int = 0
    started: int | None = None


@dataclass
class Link:
    source: int
    destination: int
    capacity: int
    handle: int
    remote_handle: int
    epoch: int
    queues: list = field(default_factory=lambda: [deque() for _ in CLASSES])
    active: Job | None = None
    in_flight: bool = False
    next_send: int = 0
    ready: bool = True
    deficits: list = field(default_factory=lambda: [0] * len(CLASSES))
    cursor: int = 0
    ingress_frames: Bucket = field(default_factory=lambda: Bucket(15, 3))
    ingress_bytes: Bucket = field(default_factory=lambda: Bucket(24 * 1024, 6144))
    egress_frames: Bucket = field(default_factory=lambda: Bucket(15, 3))
    egress_bytes: Bucket = field(default_factory=lambda: Bucket(24 * 1024, 6144))

    def jobs(self):
        return ([self.active] if self.active else []) + [job for queue in self.queues for job in queue]


class Simulation:
    """Test-driver policy. Replace policy decisions with core effects in their owning tickets.

    State and work limits here constrain synthetic offered load. They are not evidence
    of production ingress/dedup/scheduler correctness or total Python process memory.
    """
    def __init__(self, core, scenario, seed, policy="coverage-fixture", trace=None):
        assert policy in ("coverage-fixture", "unsuppressed-fixture")
        assert scenario["id"] and all(c.isascii() and (c.isalnum() or c in "-_") for c in scenario["id"])
        self.core, self.scenario, self.seed, self.policy = core, scenario, seed, policy
        self.count = scenario["nodes"]
        assert 1 <= self.count <= 64
        self.duration = scenario["duration_ms"]
        assert 0 < self.duration <= 600_000
        assert len(scenario["origin_times_ms"]) <= 512
        self.now, self.epoch = 0, 0
        self.events, self.sequences, self.wakes = [], [0] * self.count, {}
        self.links = {}
        self.mode = ["normal"] * self.count
        self.identity, self.version, self.wall = [0] * self.count, [1] * self.count, [0] * self.count
        self.received = [set() for _ in range(self.count)]
        self.peer_has = [set() for _ in range(self.count)]
        self.origins, self.deliveries, self.expected = {}, {}, set()
        self.outside, self.outage_pairs = set(), set()
        self.counters, self.egresses = Counter(), {}
        self.peak_node_jobs, self.peak_link_jobs = [0] * self.count, 0
        self.node_frames_in = [Bucket(60, 24) for _ in range(self.count)]
        self.node_bytes_in = [Bucket(64 * 1024, 24576) for _ in range(self.count)]
        self.node_frames_out = [Bucket(60, 24) for _ in range(self.count)]
        self.node_bytes_out = [Bucket(64 * 1024, 24576) for _ in range(self.count)]
        self.forward = [Bucket(120, 6) for _ in range(self.count)]
        self.trace = trace
        self.trace_hash = hashlib.sha256()
        self.link_cursor = [0] * self.count
        core.call("reset", nodes=self.count)
        self.edges = {}
        for edge in scenario["edges"]:
            a, b, *capacities = edge
            assert 0 <= a < self.count and 0 <= b < self.count and a != b
            key = (min(a, b), max(a, b))
            assert key not in self.edges
            ca, cb = capacities or [146, 146]
            self.edges[key] = (ca, cb) if a < b else (cb, ca)
        for a, b in sorted(self.edges):
            if [a, b] not in scenario.get("initial_down", []):
                self.connect(a, b)
        for index, when in enumerate(scenario["origin_times_ms"]):
            assert 0 <= when < self.duration
            self.event(when, 1, index % self.count, "origin", index)
        for change in scenario.get("changes", []):
            assert 0 <= change["time_ms"] <= self.duration
            self.event(change["time_ms"], 1, change.get("node", min(change.get("edge", [0]))), "change", change)
        if scenario.get("announce", True):
            for node in range(self.count):
                self.event(0, 1, node, "announce", 0)

    def record(self, kind, **fields):
        row = canonical(dict(time_ms=self.now, event=kind, **fields)) + "\n"
        self.trace_hash.update(row.encode("utf-8"))
        if self.trace:
            self.trace.write(row)

    def event(self, when, priority, node, kind, data):
        assert when >= self.now and 0 <= node < self.count
        if when > self.duration:
            return
        seq = self.sequences[node]
        self.sequences[node] += 1
        heapq.heappush(self.events, (when, priority, node, seq, kind, data))
        assert len(self.events) <= 16384, "fixture event ceiling"

    def wake(self, node, when):
        when = max(when, self.now)
        if self.wakes.get(node, self.duration + 1) > when:
            self.wakes[node] = when
            self.event(when, 2, node, "schedule", None)

    def connect(self, a, b):
        a, b = sorted((a, b))
        if (a, b) in self.links:
            raise ValueError("link already connected")
        ca, cb = self.edges[a, b]
        ra = self.core.call("connect", node=a, now=self.now, send=ca, receive=cb)
        if "error" in ra:
            self.counters["link_refusals"] += 1
            self.record("link_refused", source=a, destination=b, reason=ra["error"])
            return
        rb = self.core.call("connect", node=b, now=self.now, send=cb, receive=ca)
        if "error" in rb:
            self.core.call("disconnect", node=a, now=self.now, handle=ra["handle"])
            self.counters["link_refusals"] += 1
            self.record("link_refused", source=b, destination=a, reason=rb["error"])
            return
        self.epoch += 1
        self.links[a, b] = Link(a, b, ca, ra["handle"], rb["handle"], self.epoch)
        self.links[b, a] = Link(b, a, cb, rb["handle"], ra["handle"], self.epoch)
        self.record("connect", source=a, destination=b, epoch=self.epoch, capacities=[ca, cb])

    def disconnect(self, a, b):
        for source, dest in ((a, b), (b, a)):
            link = self.links.pop((source, dest), None)
            if link is None:
                continue
            for job in link.jobs():
                self.failure(job, "disconnect", source, dest)
            self.core.call("disconnect", node=source, now=self.now, handle=link.handle)
            self.peer_has[source] = {key for key in self.peer_has[source] if key[0] != dest}
        self.record("disconnect", source=a, destination=b)

    def failure(self, job, reason, source, dest):
        self.counters[f"{job.category}_object_failures"] += 1
        self.record("object_failed", object=job.obj, category=job.category, reason=reason, source=source, destination=dest)

    def queue(self, node, dest, obj, body, category):
        link = self.links[node, dest]
        encoded = self.core.call("encode", body=body.hex(), capacity=link.capacity)
        if "error" in encoded:
            self.counters["origin_codec_refusals" if category == "origin" else "codec_refusals"] += 1
            self.record("encode_refused", source=node, destination=dest, object=obj, reason=encoded["error"])
            return False
        ci = CLASSES.index(category)
        if category == "control":
            # Only ANNOUNCE uses this coalescible fixture queue.
            self.counters["coalesced_announcements"] += len(link.queues[ci])
            link.queues[ci].clear()
        total = sum(len(item.jobs()) for key, item in self.links.items() if key[0] == node)
        if len(link.jobs()) >= 32 or total >= 128:
            self.counters[f"{category}_queue_refusals"] += 1
            self.record("queue_refused", source=node, destination=dest, category=category, object=obj)
            return False
        holdoff = 0
        if category == "relay" and self.policy == "coverage-fixture":
            low, high = {"normal": (80, 400), "background": (40, 150), "saver": (300, 700), "beacon": (10, 40)}[self.mode[node]]
            holdoff = low + random_word(self.seed, "holdoff", node, dest, obj) % (high - low + 1)
        job = Job(obj, body, category, encoded["frames"], self.now, self.now + holdoff)
        link.queues[ci].append(job)
        self.peak_node_jobs[node] = max(self.peak_node_jobs[node], total + 1)
        self.peak_link_jobs = max(self.peak_link_jobs, len(link.jobs()))
        self.record("queued", source=node, destination=dest, object=obj, category=category, frames=len(job.frames), ready_ms=job.ready)
        self.wake(node, job.ready)
        return True

    def distances(self, origin, removed=()):
        distance, todo = {origin: 0}, deque([origin])
        while todo:
            node = todo.popleft()
            for a, b in self.edges:
                if [a, b] in removed or [b, a] in removed:
                    continue
                dest = b if a == node else a if b == node else None
                if dest is not None and dest not in distance:
                    distance[dest] = distance[node] + 1
                    todo.append(dest)
        return distance

    def originate(self, node, index):
        # Denominators are scheduled, even when no link or queue will admit an origin.
        body = traffic_body(self.seed, node, index, self.identity[node], self.version[node], self.wall[node])
        obj = int.from_bytes(body[4:12], "big")
        assert obj not in self.origins, "regenerate fixture: message ID collision"
        self.origins[obj] = (node, self.now, body)
        distances = self.distances(node)
        outage = self.scenario.get("outage", {})
        affected = (outage and outage["start_ms"] <= self.now < outage["end_ms"])
        available = self.distances(node, outage["edges"]) if affected else distances
        for dest, distance in distances.items():
            pair = (obj, dest)
            if 1 <= distance <= 7:
                self.expected.add(pair)
                if available.get(dest, 8) > 7:
                    self.outage_pairs.add(pair)
            elif distance > 7:
                self.outside.add(pair)
        self.received[node].add(obj)
        self.record("origin", source=node, object=obj, version=self.version[node], identity_epoch=self.identity[node], wall_offset_ms=self.wall[node])
        accepted = [self.queue(node, dest, obj, body, "origin") for source, dest in sorted(self.links) if source == node]
        if not any(accepted):
            self.counters["origins_without_admitted_egress"] += 1

    def change(self, node, change):
        kind = change["kind"]
        if kind == "down":
            self.disconnect(*change["edge"])
        elif kind == "up":
            self.connect(*change["edge"])
        elif kind == "readiness":
            a, b = change["edge"]
            self.links[a, b].ready = change["ready"]
            self.wake(a, self.now)
        elif kind == "power":
            mode = change["mode"]
            self.mode[node] = mode
            self.core.call("power", node=node, now=self.now, state=mode)
            self.forward[node].configure(self.now, 40 if mode == "saver" else 120, 2 if mode == "saver" else 6)
        elif kind == "identity":
            self.identity[node] += 1
        elif kind == "version":
            assert 0 <= change["version"] <= 255
            self.version[node] = change["version"]
        elif kind == "clock":
            self.wall[node] = change["offset_ms"]
        else:
            raise ValueError(f"unknown scenario change {kind}")
        self.record("change", node=node, change=change)

    def pick(self, link):
        for queue in link.queues:
            retained = deque()
            for job in queue:
                if self.now >= job.queued + 30_000:
                    self.failure(job, "queue_deadline", link.source, link.destination)
                elif (job.category == "relay" and self.policy == "coverage-fixture"
                      and (link.destination, job.obj) in self.peer_has[link.source]):
                    self.counters["suppressed_egress_objects"] += 1
                    self.record("suppressed", source=link.source, destination=link.destination, object=job.obj)
                else:
                    retained.append(job)
            queue.clear()
            queue.extend(retained)
        # Full-frame-cost deficit round robin at object boundaries, no fragment interleave.
        for _ in range(15):
            ci = link.cursor
            link.cursor = (ci + 1) % len(CLASSES)
            queue = link.queues[ci]
            if not queue:
                link.deficits[ci] = 0
                continue
            link.deficits[ci] = min(32, link.deficits[ci] + QUANTA[ci])
            if queue[0].ready <= self.now and len(queue[0].frames) <= link.deficits[ci]:
                link.deficits[ci] -= len(queue[0].frames)
                return queue.popleft()
        return None

    def schedule(self, node):
        keys = [key for key in sorted(self.links) if key[0] == node]
        if not keys:
            return
        rotation = self.link_cursor[node] % len(keys)
        keys = keys[rotation:] + keys[:rotation]
        self.link_cursor[node] += 1
        for key in keys:
            link = self.links[key]
            if link.active and link.active.started is not None and self.now >= link.active.started + 30_000:
                self.failure(link.active, "inflight_deadline", *key)
                link.active = None
            if link.in_flight:
                continue
            if not link.active:
                link.active = self.pick(link)
            job = link.active
            if not job:
                continue
            if (job.started is None and job.category == "relay" and self.policy == "coverage-fixture"
                    and (link.destination, job.obj) in self.peer_has[node]):
                self.counters["suppressed_egress_objects"] += 1
                self.record("suppressed", source=node, destination=link.destination, object=job.obj)
                link.active = None
                continue
            # Not-yet-attempted active objects keep their original queue deadline.
            if job.started is None and self.now >= job.queued + 30_000:
                self.failure(job, "queue_deadline", *key)
                link.active = None
                continue
            if not link.ready or self.now < link.next_send:
                continue
            frame = job.frames[job.index]
            size = len(frame) // 2
            costs = [(link.egress_frames, 1), (link.egress_bytes, size),
                     (self.node_frames_out[node], 1), (self.node_bytes_out[node], size)]
            if job.category in ("relay", "reaction", "sync") and self.mode[node] != "beacon":
                costs.append((self.forward[node], 1))
            if not charge(self.now, costs):
                self.counters["egress_budget_waits"] += 1
                continue
            response = self.core.call("send", node=node, now=self.now, handle=link.handle, frame=frame)
            assert response.get("frame") == frame, response
            if job.started is None:
                job.started = self.now
            link.in_flight, link.next_send = True, self.now + 1000
            metric = self.egresses.setdefault(f"{node}->{link.destination}", Counter())
            metric[f"{job.category}_frames"] += 1
            metric[f"{job.category}_bytes"] += size
            self.counters[f"{job.category}_frames"] += 1
            self.counters[f"{job.category}_bytes"] += size
            self.counters["fragment_frames" if len(job.frames) > 1 else "whole_frames"] += 1
            loss_ppm = self.scenario.get("loss_ppm", 0)
            assert 0 <= loss_ppm <= 1_000_000
            lost = random_word(self.seed, "loss", node, link.destination, job.obj, job.index, 0) < (loss_ppm * (1 << 64) // 1_000_000)
            metric["lost_frames"] += int(lost)
            self.counters["lost_frames"] += int(lost)
            self.record("attempt", source=node, destination=link.destination, epoch=link.epoch,
                        object=job.obj, category=job.category, fragment=job.index, frames=len(job.frames),
                        bytes=size, lost=lost, ttl=job.body[3])
            packet = (key, link.epoch, job.obj, job.index, frame, lost)
            self.event(self.now + self.scenario.get("transit_ms", 20), 0, node, "completion", packet)
            if not lost:
                self.event(self.now + self.scenario.get("transit_ms", 20), 0, link.destination, "arrival", packet)
        if any(link.jobs() for key, link in self.links.items() if key[0] == node):
            candidates = [self.now + 1000]
            for key, link in self.links.items():
                if key[0] == node:
                    candidates.extend(job.ready for job in link.jobs() if job.ready > self.now)
                    candidates.extend((job.started if job.started is not None else job.queued) + 30_000
                                      for job in link.jobs()
                                      if (job.started if job.started is not None else job.queued) + 30_000 > self.now)
                    if link.next_send > self.now:
                        candidates.append(link.next_send)
            self.wake(node, min(candidates))

    def completion(self, data):
        key, epoch, obj, index, _, _ = data
        link = self.links.get(key)
        if link is None or link.epoch != epoch:
            self.counters["stale_completions"] += 1
            return
        link.in_flight = False
        self.counters["native_completed_frames"] += 1
        self.egresses[f"{key[0]}->{key[1]}"]["native_completed_frames"] += 1
        job = link.active
        if job is not None and job.obj == obj and job.index == index:
            job.index += 1
            if job.index == len(job.frames):
                link.active = None
                self.counters["native_completed_objects"] += 1
        self.record("native_complete", source=key[0], destination=key[1], object=obj, fragment=index)
        self.wake(key[0], max(self.now, link.next_send))

    def arrival(self, node, data):
        (source, dest), epoch, obj, index, frame, _ = data
        link = self.links.get((dest, source))
        if link is None or link.epoch != epoch:
            self.counters["stale_arrivals"] += 1
            self.record("stale_arrival", source=source, destination=dest, object=obj, fragment=index)
            return
        size = len(frame) // 2
        if not charge(self.now, [(link.ingress_frames, 1), (link.ingress_bytes, size),
                                 (self.node_frames_in[node], 1), (self.node_bytes_in[node], size)]):
            self.counters["fixture_ingress_drops"] += 1
            self.record("ingress_drop", source=source, destination=dest, object=obj, fragment=index)
            return
        received = self.core.call("receive", node=node, now=self.now, handle=link.handle, frame=frame)
        self.counters["arrived_frames"] += 1
        self.record("arrival", source=source, destination=dest, object=obj, fragment=index, complete=received.get("complete", False), error=received.get("error"))
        if "error" in received:
            self.counters["core_rejected_frames"] += 1
            return
        if not received.get("complete") or obj not in self.origins:
            return
        body = bytes.fromhex(received["body"])
        origin, when, original = self.origins[obj]
        assert immutable(body) == immutable(original), "unexpected fixture bytes"
        if body[0] != 1 or body[1] != 1 or body[2] != 0:
            return
        if len(self.peer_has[node]) < 4096:
            self.peer_has[node].add((source, obj))
        if obj in self.received[node]:
            self.counters["duplicate_objects"] += 1
            return
        if len(self.received[node]) >= 512:
            self.counters["fixture_dedup_refusals"] += 1
            return
        self.received[node].add(obj)
        if node != origin:
            self.deliveries[obj, node] = self.now - when
            self.record("synthetic_delivery", object=obj, node=node, latency_ms=self.now - when)
        if received["forward"]:
            forwarded = bytes.fromhex(received["forward"])
            assert 1 <= forwarded[3] < min(body[3], 8)
            for src, peer in sorted(self.links):
                if src == node:
                    self.queue(node, peer, obj, forwarded, "relay")

    def run(self):
        while self.events:
            when, _, node, seq, kind, data = heapq.heappop(self.events)
            self.now = when
            if kind == "schedule":
                if self.wakes.get(node) != when:
                    continue
                del self.wakes[node]
                self.schedule(node)
            elif kind == "origin":
                self.originate(node, data)
            elif kind == "announce":
                body = announce_body(node, data * self.count + node)
                obj = int.from_bytes(body[4:12], "big")
                for source, dest in sorted(self.links):
                    if source == node:
                        self.queue(node, dest, obj, body, "control")
                period = 60_000 if self.mode[node] == "saver" else 30_000
                if self.now + period < self.duration:
                    self.event(self.now + period, 1, node, "announce", data + 1)
            elif kind == "change":
                self.change(node, data)
            elif kind == "completion":
                self.completion(data)
            elif kind == "arrival":
                self.arrival(node, data)
            else:
                raise ValueError(kind)
        self.now = self.duration
        core_stats = [self.core.call("stats", node=node, now=self.now) for node in range(self.count)]
        latencies = sorted(value for pair, value in self.deliveries.items() if pair in self.expected)
        delivered = set(self.deliveries)
        return dict(schema_version=1, scenario=self.scenario["id"], seed=self.seed, policy=self.policy,
                    evidence="tested synthetic harness and real core primitives", production_acceptance="not_run",
                    initial_state="empty fixture dedup, full fixture buckets, links assumed preadmitted; no HELLO/proof exchange",
                    pending_owners=["MC-013", "MC-014", "MC-015", "MC-016"], duration_ms=self.duration,
                    scheduled_origins=len(self.origins), reachable_pairs=len(self.expected),
                    delivered_reachable_pairs=len(delivered & self.expected),
                    delivery_ratio=len(delivered & self.expected) / len(self.expected) if self.expected else None,
                    outside_ttl_pairs=len(self.outside), outside_ttl_deliveries=len(delivered & self.outside),
                    outage_pairs=len(self.outage_pairs), delivered_outage_pairs=len(delivered & self.outage_pairs),
                    p95_latency_ms=latencies[(95 * len(latencies) + 99) // 100 - 1] if latencies else None,
                    counters=dict(self.counters), per_egress=self.egresses,
                    harness_queue_peak_objects_per_node=self.peak_node_jobs, harness_queue_peak_objects_per_link=self.peak_link_jobs,
                    harness_queue_reserved_encoded_bytes_per_slot=1024,
                    core_reassembly=core_stats, trace_sha256=self.trace_hash.hexdigest(),
                    native_retry_frames=0,
                    relay_frames_per_node=[sum(value.get("relay_frames", 0) for key, value in self.egresses.items()
                                              if key.startswith(f"{node}->")) for node in range(self.count)],
                    native_retry_policy="none; independent frame loss is unacknowledged",
                    sync_session_results=None, authentication_results=None,
                    in_flight_at_end=sum(link.in_flight for link in self.links.values()),
                    queued_at_end=sum(len(link.jobs()) for link in self.links.values()))


def scenarios(manifest):
    """Normalize committed MC-007 live/stress inputs; no silent claim of later gates."""
    assert manifest["schema_version"] == 1
    assert manifest["common"]["scheduler_frame_spacing_ms"] == 1000
    assert manifest["live_workload"]["logical_size_bytes"] == 338
    assert manifest["live_workload"]["ttl"] == 7
    capacity = manifest["common"]["capacity_bytes_each_direction"]
    for topology in manifest["topologies"]:
        base = dict(id=topology["id"], nodes=topology["nodes"], edges=[edge + [capacity, capacity] for edge in topology["edges"]],
                    duration_ms=manifest["live_workload"]["duration_seconds"] * 1000,
                    origin_times_ms=[n * 1000 for n in manifest["live_workload"]["origin_times_seconds"]],
                    transit_ms=manifest["common"]["native_completion_and_transit_ms"])
        yield base
        if topology.get("saver_repeat_nodes"):
            yield dict(base, id=base["id"] + "-saver", changes=[dict(time_ms=0, kind="power", node=n, mode="saver") for n in topology["saver_repeat_nodes"]])
        edges = topology.get("bridge_edges", [])
        start, end = manifest["stress_repeats"]["bridge_edges_down_seconds"]
        loss_ppm = int(Decimal(str(manifest["stress_repeats"]["independent_frame_loss_probability"])) * 1_000_000)
        yield dict(base, id=base["id"] + "-stress", loss_ppm=loss_ppm,
                   outage=dict(start_ms=start * 1000, end_ms=end * 1000, edges=edges),
                   changes=[dict(time_ms=t * 1000, kind=kind, edge=edge)
                            for t, kind in ((start, "down"), (end, "up")) for edge in edges])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--core", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--scenario", help="one manifest scenario ID; default all")
    parser.add_argument("--seed", type=int, help="one seed; default all committed seeds")
    parser.add_argument("--extra", type=Path, help="additional simulator driver scenario JSON array")
    args = parser.parse_args()
    output = args.output.resolve()
    if not output.is_relative_to(ROOT / ".work"):
        parser.error("generated evidence must stay under the repository .work directory")
    output.mkdir(parents=True, exist_ok=True)
    manifest_bytes = MANIFEST.read_bytes()
    manifest = json.loads(manifest_bytes)
    cases = list(scenarios(manifest))
    if args.extra:
        cases.extend(json.loads(args.extra.read_text(encoding="utf-8")))
    if args.scenario:
        cases = [case for case in cases if case["id"] == args.scenario]
        if not cases:
            parser.error("unknown scenario")
    revision = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    dirty = bool(subprocess.check_output(["git", "status", "--porcelain", "--untracked-files=normal"], cwd=ROOT, text=True))
    source_digest = hashlib.sha256()
    for path in sorted(list((ROOT / "src/core").rglob("*.rs")) + list((ROOT / "tests/simulator").rglob("*.py")) + list((ROOT / "tests/simulator/src").rglob("*.rs"))):
        source_digest.update(path.relative_to(ROOT).as_posix().encode() + b"\0" + path.read_bytes())
    provenance = dict(source_revision=revision, source_dirty=dirty, source_tree_sha256=source_digest.hexdigest(),
                      core_binary_sha256=hashlib.sha256(args.core.read_bytes()).hexdigest(),
                      manifest_sha256=hashlib.sha256(manifest_bytes).hexdigest())
    core = CoreProcess(args.core.resolve())
    reports = []
    try:
        for case in cases:
            for seed in [args.seed] if args.seed is not None else manifest["seeds"]:
                pair = []
                for policy in ("coverage-fixture", "unsuppressed-fixture"):
                    name = f"{case['id']}-{seed}-{policy}"
                    with (output / f"{name}.jsonl").open("w", encoding="utf-8", newline="\n") as trace:
                        result = Simulation(core, case, seed, policy, trace).run()
                    repeated = Simulation(core, case, seed, policy).run()
                    assert result == repeated, f"nondeterministic metrics/trace: {name}"
                    result.update(provenance, scenario_sha256=hashlib.sha256(canonical(case).encode()).hexdigest(), identical_rerun=True)
                    pair.append(result)
                baseline = pair[1]["counters"].get("relay_frames", 0)
                ratio = pair[0]["counters"].get("relay_frames", 0) / baseline if baseline else None
                for result in pair:
                    result["paired_fixture_relay_frame_ratio"] = ratio
                    reports.append(result)
                print(f"{case['id']} seed={seed}: fixture delivery {pair[0]['delivered_reachable_pairs']}/{pair[0]['reachable_pairs']}; relay ratio={ratio}; identical reruns", flush=True)
    finally:
        core.close()
    (output / "metrics.json").write_text(json.dumps(reports, indent=2, sort_keys=True) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
