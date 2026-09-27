import unittest
import json
from pathlib import Path
import tempfile
from unittest.mock import patch
from acceptance_analysis import aggregate, distribution, read


def fixture(initiator="A"):
    runs = []
    for role in ("A", "B"):
        rows = [dict(event="plan", mode="acceptance", initiator=initiator, scheduled_pairs=60, traced=False, poll_ms=100),
                dict(event="summary", completed_pairs=60, scheduled_pairs=60)]
        for i in range(60):
            if role == initiator:
                rows += [dict(event="request", index=i, ready=True),
                         dict(event="roundtrip", index=i, size="long" if i % 2 else "short", elapsed_ms=900,
                              received=True, own_accepted=True)]
            else:
                rows += [dict(event="request_received", index=i), dict(event="echo", index=i, native_complete=True)]
        runs.append(([dict(r, role=role, run="fresh") for r in rows], [], True))
    return runs


class AcceptanceTests(unittest.TestCase):
    def analyze(self, data):
        with patch("acceptance_analysis.read", side_effect=data):
            return aggregate("A", "B")

    def test_both_initiators_and_nearest_rank_p95(self):
        for role in ("A", "B"):
            result = self.analyze(fixture(role))
            self.assertTrue(result["typical_subsecond_target_met"])
            self.assertEqual(result["successful_observations"]["long"]["successful_samples"], 30)
        self.assertEqual(distribution(list(range(1,31)))["p95_ms"], 29)

    def test_partial_and_failed_runs_keep_sixty_slots(self):
        data=fixture();data[0]=(data[0][0][:12], [], False)
        result=self.analyze(data)
        self.assertEqual(len(result["samples"]),60)
        self.assertFalse(result["typical_subsecond_target_met"])
        self.assertIn("not_attempted",{s["status"] for s in result["samples"]})

    def test_refusal_and_missing_peer_are_not_success(self):
        data=fixture();data[0][0][3]["received"]=False
        data[1][0].pop()
        result=self.analyze(data)
        self.assertEqual(result["observed_pairs"],58)
        self.assertFalse(result["complete"])

    def test_mixed_duplicate_out_of_plan_and_mismatched_plan_refused(self):
        for mutate in (lambda d:d[0][0][2].update(run="other"),
                       lambda d:d[0][0].append(d[0][0][2]),
                       lambda d:d[0][0][2].update(index=60),
                       lambda d:d[1][0][0].update(initiator="B")):
            data=fixture();mutate(data)
            with self.assertRaises(ValueError):self.analyze(data)

    def test_entire_missing_endpoint_and_trace_overflow_fail_closed(self):
        data=fixture();data[1]=([],[],False)
        self.assertFalse(self.analyze(data)["complete"])
        data=fixture()
        for rows,_,_ in data:rows[0]["traced"]=True
        self.assertFalse(self.analyze(data)["complete"])

    def test_truncated_record_never_turns_a_failed_capture_into_a_pass(self):
        with tempfile.TemporaryDirectory(dir=Path(__file__).resolve().parents[3]/".work") as folder:
            path=Path(folder)/"log"
            path.write_text("\n".join("MC025 "+json.dumps(r) for r in fixture()[0][0])+'\nMC025 {"event":\nOK (1 test)',encoding="utf8")
            rows,_,passed=read(path)
            self.assertEqual(len(rows),122)
            self.assertFalse(passed)


if __name__ == "__main__":unittest.main()
