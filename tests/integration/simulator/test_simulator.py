"""Synthetic simulator/real-core contract checks; no production relay gate claims."""
import hashlib
import io
import json
import os
from pathlib import Path
import sys
import unittest

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "tests/simulator"))
from runner import (Bucket, CLASSES, CoreProcess, MANIFEST, Simulation, canonical,
                    charge, random_word, scenarios, traffic_body)


def case(nodes=2, edges=None, times=None, duration=40000, **kwargs):
    return dict(id="test", nodes=nodes, edges=edges if edges is not None else [[0, 1]],
                duration_ms=duration, origin_times_ms=[0] if times is None else times,
                announce=False, **kwargs)


class SimulatorTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        executable = Path(os.environ.get("MESHCHAT_SIM_CORE", ROOT / "target/debug/meshchat-simulator-core.exe"))
        cls.core = CoreProcess(executable.resolve())

    @classmethod
    def tearDownClass(cls):
        cls.core.close()

    def run_case(self, spec, policy="coverage-fixture", seed=7):
        trace = io.StringIO()
        simulation = Simulation(self.core, spec, seed, policy, trace)
        result = simulation.run()
        return simulation, result, [json.loads(row) for row in trace.getvalue().splitlines()]

    def test_hash_encoding_known_external_vector(self):
        # Independently laid out literal fields, rather than calling the encoder twice.
        raw = bytes.fromhex("000000000000000700046c6f737300000001000000020000000000000003000400000005")
        expected = int.from_bytes(hashlib.sha256(raw).digest()[:8], "big")
        self.assertEqual(random_word(7, "loss", 1, 2, 3, 4, 5), expected)

    def test_atomic_buckets_reconnect_and_power_transition_credit(self):
        a, b = Bucket(1, 3), Bucket(2, 6)
        self.assertFalse(charge(0, [(a, 2), (b, 1)]))
        self.assertEqual(b.credit, 6000)
        self.assertTrue(charge(0, [(a, 1), (b, 2)]))
        self.assertFalse(charge(999, [(a, 1)]))
        self.assertTrue(charge(1000, [(a, 1)]))
        a.configure(1000, 40, 2)
        self.assertEqual(a.credit, 0)  # Upgrade does not grant credit.
        with self.assertRaises(AssertionError):
            charge(999, [(a, 1)])

    def test_one_way_frames_use_actual_encoder_and_reassembly(self):
        _, result, rows = self.run_case(case())
        origin = [row for row in rows if row["event"] == "attempt" and row["category"] == "origin"]
        self.assertEqual([row["bytes"] for row in origin], [146, 146, 100])
        self.assertEqual([row["fragment"] for row in origin], [0, 1, 2])
        self.assertEqual(result["delivered_reachable_pairs"], 1)
        self.assertEqual(result["p95_latency_ms"], 2020)
        self.assertEqual(result["counters"]["origin_bytes"], 392)
        self.assertEqual(result["counters"]["relay_frames"] if "relay_frames" in result["counters"] else 0, 0)

    def test_directional_capacity_and_no_broadcast_accounting(self):
        _, result, rows = self.run_case(case(nodes=3, edges=[[0, 1, 146, 512], [0, 2, 182, 146]]))
        attempts = [row for row in rows if row["event"] == "attempt" and row["category"] == "origin"]
        self.assertEqual(len(attempts), 6)  # 3 fragments to each individual peer.
        self.assertEqual(sum(row["bytes"] for row in attempts), 784)
        self.assertEqual(result["delivered_reachable_pairs"], 2)
        _, reverse, _ = self.run_case(case(edges=[[0, 1, 146, 512]], times=[0, 20000]))
        self.assertEqual(reverse["per_egress"]["1->0"]["origin_frames"], 1)
        self.assertEqual(reverse["per_egress"]["1->0"]["origin_bytes"], 342)

    def test_capacity_floor_actual_encoder_and_origin_denominator(self):
        body = traffic_body(7, 0, 0).hex()
        for capacity, count in ((146, 3), (182, 3), (512, 1)):
            self.assertEqual(len(self.core.call("encode", body=body, capacity=capacity)["frames"]), count)
        self.assertIn("error", self.core.call("encode", body=body, capacity=145))
        _, result, _ = self.run_case(case(edges=[[0, 1, 145, 146]]))
        self.assertEqual(result["reachable_pairs"], 1)
        self.assertEqual(result["delivered_reachable_pairs"], 0)
        self.assertEqual(result["counters"]["origins_without_admitted_egress"], 1)

    def test_trace_digest_replay_and_seed_changes_loss(self):
        spec = case(nodes=3, edges=[[0, 1], [1, 2]], times=[0, 10000, 20000], loss_ppm=200000)
        _, first, rows = self.run_case(spec)
        _, second, same = self.run_case(spec)
        self.assertEqual(first, second)
        self.assertEqual(rows, same)
        self.assertEqual(first["trace_sha256"], hashlib.sha256("".join(canonical(row) + "\n" for row in rows).encode()).hexdigest())
        _, different, _ = self.run_case(spec, seed=19)
        self.assertNotEqual(first["trace_sha256"], different["trace_sha256"])

    def test_loss_cannot_complete_missing_fragments(self):
        _, result, _ = self.run_case(case(loss_ppm=1000000))
        self.assertEqual(result["counters"]["lost_frames"], 3)
        self.assertEqual(result["counters"]["native_completed_frames"], 3)
        self.assertEqual(result["delivered_reachable_pairs"], 0)
        self.assertIsNone(result["sync_session_results"])

    def test_stale_completion_and_arrival_after_reconnect(self):
        spec = case(changes=[dict(time_ms=10, kind="down", edge=[0, 1]),
                             dict(time_ms=15, kind="up", edge=[0, 1])])
        _, result, _ = self.run_case(spec)
        self.assertEqual(result["counters"]["stale_arrivals"], 1)
        self.assertEqual(result["counters"]["stale_completions"], 1)
        self.assertEqual(result["delivered_reachable_pairs"], 0)

    def test_late_join_retains_failed_pairs_and_does_not_invent_sync(self):
        spec = case(nodes=3, edges=[[0, 1], [1, 2]], times=[0, 20000], initial_down=[[1, 2]],
                    changes=[dict(time_ms=10000, kind="up", edge=[1, 2])])
        _, result, _ = self.run_case(spec)
        self.assertEqual(result["reachable_pairs"], 4)
        self.assertEqual(result["delivered_reachable_pairs"], 3)

    def test_queue_overflow_and_absolute_deadlines(self):
        spec = case(times=list(range(100)), duration=65000,
                    changes=[dict(time_ms=0, kind="readiness", edge=[0, 1], ready=False),
                             dict(time_ms=35000, kind="readiness", edge=[0, 1], ready=True)])
        _, result, rows = self.run_case(spec)
        self.assertLessEqual(result["harness_queue_peak_objects_per_link"], 32)
        self.assertLessEqual(max(result["harness_queue_peak_objects_per_node"]), 128)
        self.assertGreater(result["counters"]["origin_queue_refusals"], 0)
        self.assertEqual(result["reachable_pairs"], 100)
        self.assertTrue(any(row["event"] == "object_failed" and row["reason"] == "queue_deadline" for row in rows))

    def test_versions_identity_skew_and_power_are_explicit(self):
        cases = json.loads((ROOT / "tests/simulator/scenarios/MC-012-driver-cases.json").read_text(encoding="utf-8"))
        spec = next(item for item in cases if item["id"] == "version-identity-clock-power")
        simulation, result, rows = self.run_case(spec)
        self.assertEqual(result["reachable_pairs"], 10)
        self.assertEqual(result["delivered_reachable_pairs"], 8)
        self.assertEqual(result["counters"]["origin_codec_refusals"], 2)
        origins = [row for row in rows if row["event"] == "origin" and row["source"] == 1]
        self.assertEqual([row["identity_epoch"] for row in origins], [1, 2])
        self.assertEqual(simulation.mode[1], "beacon")
        self.assertEqual(simulation.wall[2], -7200000)
        self.assertEqual(result["production_acceptance"], "not_run")

    def test_chain_ttl_dense_pairing_and_aggregate_accounting(self):
        cases = list(scenarios(json.loads(MANIFEST.read_text(encoding="utf-8"))))
        for name, denominator in (("chain10", 99), ("dense6", 60), ("bridge9-saver", 96)):
            spec = next(item for item in cases if item["id"] == name)
            _, result, rows = self.run_case(spec)
            _, baseline, _ = self.run_case(spec, "unsuppressed-fixture")
            self.assertEqual(result["reachable_pairs"], denominator)
            self.assertEqual(result["delivered_reachable_pairs"], denominator)
            self.assertEqual(result["outside_ttl_deliveries"], 0)
            self.assertLessEqual(result["counters"].get("relay_frames", 0), baseline["counters"].get("relay_frames", 0))
            for category in CLASSES:
                for unit in ("frames", "bytes"):
                    key = f"{category}_{unit}"
                    self.assertEqual(result["counters"].get(key, 0), sum(value.get(key, 0) for value in result["per_egress"].values()))
            last = {}
            for row in rows:
                if row["event"] == "attempt":
                    edge = row["source"], row["destination"]
                    if edge in last:
                        self.assertGreaterEqual(row["time_ms"] - last[edge], 1000)
                    last[edge] = row["time_ms"]
                    if row["category"] == "relay":
                        self.assertGreater(row["ttl"], 0)

    def test_core_rejects_invalid_wire_and_stale_handles_directly(self):
        self.core.call("reset", nodes=2)
        handle = self.core.call("connect", node=0, now=0, send=146, receive=146)["handle"]
        # Reserved outer header bit, passed through native boundary to real reassembly.
        result = self.core.call("receive", node=0, now=0, handle=handle, frame="00010000")
        self.assertIn("error", result)
        self.core.call("disconnect", node=0, now=1, handle=handle)
        self.assertIn("error", self.core.call("receive", node=0, now=2, handle=handle, frame="00010000"))

    def test_transport_marker_is_structural_completion_only(self):
        self.core.call("reset", nodes=1)
        handle = self.core.call("connect", node=0, now=0, send=146, receive=146)["handle"]
        marker = "0001000005000000000000"
        frames = self.core.call("encode", body=marker, capacity=146, transport=1)["frames"]
        self.assertEqual(len(frames), 1)
        self.assertEqual(len(frames[0]) // 2, 16)
        received = self.core.call("receive", node=0, now=20, handle=handle, frame=frames[0])
        self.assertEqual(received, dict(complete=True, transport=True, body=marker))

    def test_outage_cohort_keeps_static_reachable_denominator(self):
        spec = case(nodes=3, edges=[[0, 1], [1, 2]], initial_down=[[1, 2]],
                    outage=dict(start_ms=0, end_ms=30000, edges=[[1, 2]]))
        _, result, _ = self.run_case(spec)
        self.assertEqual(result["reachable_pairs"], 2)
        self.assertEqual(result["outage_pairs"], 1)
        self.assertEqual(result["delivered_outage_pairs"], 0)
        self.assertEqual(result["delivered_reachable_pairs"], 1)


if __name__ == "__main__":
    unittest.main()
