"""MC-014 production relay driver. Python supplies topology, time and native outcomes.

Origin fixtures and metric denominators stay shared with MC-012. All queueing,
TTL mutation, coverage, pacing and work admission are performed by the Rust core.
"""
from collections import Counter
from runner import Simulation, immutable, random_word


class RelaySimulation(Simulation):
    def call_relay(self, node, op, **fields):
        result = self.core.call(op, node=node, now=self.now, **fields)
        for event in result.get("events", []):
            self.record("relay_result", source=node, **event)
            status = event["status"]
            if status == "Suppressed":
                self.counters["suppressed_egress_objects"] += 1
            elif status == "NativeComplete":
                self.counters["native_completed_objects"] += 1
            elif status == "Coalesced":
                self.counters["coalesced_announcements"] += 1
            else:
                self.counters[f"{event['category']}_object_failures"] += 1
        if result.get("wake") is not None:
            self.wake(node, result["wake"])
        return result

    def queue(self, node, dest, obj, body, category):
        link = self.links[node, dest]
        if category == "control":
            digest = bytes.fromhex(self.call_relay(node, "relay_digest")["digest"])
            body = body[:-256] + digest
        response = self.call_relay(node, "relay_queue", handle=link.handle, body=body.hex(),
                                   category=category, object=obj,
                                   random=random_word(self.seed, "holdoff", node, dest, obj))
        if "error" in response:
            self.counters[f"{category}_queue_refusals"] += 1
            self.record("queue_refused", source=node, destination=dest, category=category,
                        object=obj, reason=response["error"])
            return False
        self.record("queued", source=node, destination=dest, object=obj, category=category,
                    admitted=response["queued"])
        self.wake(node, self.now)
        return response["queued"]

    def disconnect(self, a, b):
        for source, dest in ((a, b), (b, a)):
            link = self.links.pop((source, dest), None)
            if link is not None:
                self.call_relay(source, "disconnect", handle=link.handle)
        self.record("disconnect", source=a, destination=b)

    def change(self, node, change):
        super().change(node, change)
        if change["kind"] == "readiness":
            a, b = change["edge"]
            self.call_relay(a, "relay_ready", handle=self.links[a, b].handle, ready=change["ready"])
        self.wake(node, self.now)

    def schedule(self, node):
        # One call emits at most one native attempt. At most eight can start at
        # one timestamp; pacing, link rotation and every bucket live in Rust.
        for _ in range(9):
            response = self.call_relay(node, "relay_poll")
            if "send" not in response:
                return
            s = response["send"]
            link = next(l for (a, _), l in self.links.items() if a == node and l.handle == s["handle"])
            key = (node, link.destination)
            category, size = s["category"], len(s["frame"]) // 2
            metric = self.egresses.setdefault(f"{node}->{link.destination}", Counter())
            metric[f"{category}_frames"] += 1
            metric[f"{category}_bytes"] += size
            self.counters[f"{category}_frames"] += 1
            self.counters[f"{category}_bytes"] += size
            self.counters["fragment_frames" if s["frames"] > 1 else "whole_frames"] += 1
            self.counters["native_retry_frames"] += int(s["retry"])
            loss_ppm = self.scenario.get("loss_ppm", 0)
            lost = random_word(self.seed, "loss", node, link.destination, s["object"], s["fragment"], int(s["retry"])) < loss_ppm * (1 << 64) // 1_000_000
            metric["lost_frames"] += int(lost)
            self.counters["lost_frames"] += int(lost)
            link.in_flight = True
            self.record("attempt", source=node, destination=link.destination, epoch=link.epoch,
                        object=s["object"], category=category, fragment=s["fragment"], frames=s["frames"],
                        bytes=size, lost=lost, ttl=s["ttl"], retry=s["retry"])
            packet = (key, link.epoch, s["object"], s["fragment"], s["frame"], lost, s["token"])
            delay = self.scenario.get("transit_ms", 20)
            self.event(self.now + delay, 0, node, "completion", packet)
            if not lost:
                self.event(self.now + delay, 0, link.destination, "arrival", packet)
        raise AssertionError("more than eight paced native attempts at one timestamp")

    def completion(self, data):
        key, epoch, obj, index, _, _, token = data
        link = self.links.get(key)
        if link is None or link.epoch != epoch:
            self.counters["stale_completions"] += 1
            return
        link.in_flight = False
        result = self.call_relay(key[0], "relay_complete", token=token, success=True)
        if "error" in result:
            self.counters["stale_completions"] += 1
        self.counters["native_completed_frames"] += 1
        self.egresses[f"{key[0]}->{key[1]}"]["native_completed_frames"] += 1
        self.record("native_complete", source=key[0], destination=key[1], object=obj, fragment=index)
        self.wake(key[0], self.now)

    def arrival(self, node, data):
        (source, dest), epoch, obj, index, frame, _, _ = data
        link = self.links.get((dest, source))
        if link is None or link.epoch != epoch:
            self.counters["stale_arrivals"] += 1
            self.record("stale_arrival", source=source, destination=dest, object=obj, fragment=index)
            return
        received = self.call_relay(node, "receive", handle=link.handle, frame=frame)
        self.counters["arrived_frames"] += 1
        self.record("arrival", source=source, destination=dest, object=obj, fragment=index,
                    complete=received.get("complete", False), error=received.get("error"))
        if "error" in received:
            self.counters["core_rejected_frames"] += 1
            if received.get("ingress_drop"):
                self.counters["production_ingress_drops"] += 1
            return
        if not received.get("complete") or obj not in self.origins:
            return
        body = bytes.fromhex(received["body"])
        origin, when, original = self.origins[obj]
        assert immutable(body) == immutable(original), "unexpected fixture bytes"
        if body[:3] != bytes([1, 1, 0]):
            return
        if obj not in self.received[node]:
            self.received[node].add(obj)
            if node != origin:
                self.deliveries[obj, node] = self.now - when
                self.record("synthetic_delivery", object=obj, node=node, latency_ms=self.now - when)
        if received.get("fresh_relay"):
            # Queue RECEIVED bytes. Rust alone applies TTL and per-egress coverage.
            for src, peer in sorted(self.links):
                if src == node:
                    self.queue(node, peer, obj, body, "relay")
        else:
            self.counters["duplicate_objects"] += 1
        self.wake(node, self.now)

    def run(self):
        result = super().run()
        stats = [node["relay"] for node in result["core_reassembly"]]
        for stat in stats:
            assert stat["peak_node_objects"] <= 128 and stat["peak_link_objects"] <= 32
            assert stat["outbound_bytes"] <= 192 * 1024 and stat["outbound_bytes_per_link"] <= 48 * 1024
            assert stat["recent_bytes"] <= 16 * 1024 and stat["peak_recent"] <= 200
        assert sum(s["attempts"] for s in stats) == sum(v for k, v in self.counters.items() if k in ("origin_frames", "relay_frames", "control_frames", "reaction_frames", "sync_frames"))
        assert result["outside_ttl_deliveries"] == 0
        result.update(evidence="production ingress and relay with simulated native GATT outcomes",
                      production_acceptance="MC-014 live relay; SYNC/authentication/native radio pending",
                      relay_mode="production core", pending_owners=["MC-015", "MC-016"],
                      initial_state="empty core ingress/relay state, full buckets, named links assumed preadmitted; no HELLO/proof exchange",
                      power_tier_attempts=[s["attempts_by_tier"] for s in stats],
                      mode_forwarded_attempts=[s["forwarded_by_mode"] for s in stats],
                      native_retry_frames=sum(s["retries"] for s in stats),
                      native_retry_policy="core allows one native-failure retry; simulated air loss remains unacknowledged",
                      queued_at_end=sum(s["objects"] for s in stats))
        for key in [k for k in result if k.startswith("harness_queue_")]:
            del result[key]
        return result
