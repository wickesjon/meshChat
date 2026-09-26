import json
import unittest
from resource_analysis import analyze


def fixture():
    rows=[dict(event="plan",duration_seconds=1800,sample_seconds=60,apk_sha256="synthetic")]
    rows += [dict(event="sample",index=i,elapsed_ms=i*60000,cpu_ms=i*1200,completed_tasks=i*60,
                  queued_tasks=0,pss_kib=10000,workload_valid=True,screen_interactive=True,
                  plugged=0,charge_uah=4000000-i*1000,battery_percent=80-i//10,
                  scheduled_frames=str(i),received_frames=str(i),stats_elapsed_ms=str(i*60000)) for i in range(31)]
    rows += [dict(event="summary",samples=31)]
    return [dict(r,run="new",workload="connected_idle") for r in rows]


def report(rows, passed=True):
    return analyze("\n".join("MC025R "+json.dumps(r) for r in rows)+( "\nOK (1 test)" if passed else ""))


class ResourceTests(unittest.TestCase):
    def test_unplugged_matched_samples(self):
        result=report(fixture())
        self.assertTrue(result["battery_drain_available"])
        self.assertEqual(result["whole_device_charge_loss_uah_per_hour"],60000)
        self.assertEqual(result["cpu_percent_one_core"],2)

    def test_powered_unknown_gauge_and_rising_charge_never_claim_drain(self):
        for change in (lambda r:r.update(plugged=1),lambda r:r.update(charge_uah=None),lambda r:r.update(charge_uah=5000000)):
            rows=fixture();change(rows[2])
            self.assertFalse(report(rows)["battery_drain_available"])
        rows=fixture()
        for row in rows:
            if row["event"]=="sample":row["plugged"]=1
        result=report(rows)
        self.assertTrue(result["comparable"])
        self.assertFalse(result["battery_drain_available"])

    def test_missing_failed_workload_and_screen_changes_are_visible(self):
        for change in (lambda r:r[-2].update(workload_valid=False),lambda r:r[2].update(screen_interactive=False),
                       lambda r:r[2].update(cpu_ms=-1),lambda r:r.pop()):
            rows=fixture();change(rows)
            self.assertFalse(report(rows)["comparable"])
        self.assertFalse(report(fixture(),False)["comparable"])

    def test_duplicate_or_mixed_logs_refused(self):
        rows=fixture();rows.insert(2,rows[1])
        with self.assertRaises(ValueError):report(rows)
        rows=fixture();rows[3]["run"]="old"
        with self.assertRaises(ValueError):report(rows)

    def test_truncated_log_preserves_partial_samples_without_passing(self):
        text="\n".join("MC025R "+json.dumps(r) for r in fixture()[:-1])+ '\nMC025R {"event":'
        result=analyze(text)
        self.assertEqual(result["samples"],31)
        self.assertFalse(result["comparable"])


if __name__=="__main__":unittest.main()
