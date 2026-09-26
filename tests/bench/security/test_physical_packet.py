from pathlib import Path
import tempfile
import unittest
from physical_packet import ROOT, blank_packet, validate


class PacketTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        (ROOT/".work").mkdir(parents=True,exist_ok=True)

    def test_blank_matrix_is_complete_but_uncertified(self):
        result=validate(blank_packet())
        self.assertEqual(result["states"],{"not_run":34})
        self.assertFalse(result["certified"])
        self.assertFalse(result["ready_for_manual_evidence_review"])

    def test_missing_duplicate_and_unknown_states_refused(self):
        for change in (lambda p:p["scenarios"].pop(),lambda p:p["scenarios"].append(p["scenarios"][0]),
                       lambda p:p["scenarios"][0].update(state="skipped")):
            packet=blank_packet();change(packet)
            with self.assertRaises(ValueError):validate(packet)

    def test_unavailable_needs_reason_and_does_not_become_pass(self):
        packet=blank_packet();packet["scenarios"][0]["state"]="unavailable"
        with self.assertRaises(ValueError):validate(packet)
        packet["scenarios"][0]["observation"]="API 29 device unavailable"
        self.assertFalse(validate(packet)["ready_for_manual_evidence_review"])

    def test_physical_results_need_build_lock_and_real_evidence(self):
        with tempfile.TemporaryDirectory(dir=ROOT/".work") as folder:
            root=Path(folder);artifact=root/"sanitized.txt";artifact.write_text("Synthetic validator fixture",encoding="utf8")
            packet=blank_packet();row=packet["scenarios"][0]
            row.update(state="pass",date="2026-09-26",duration_seconds=5,observation="Synthetic fixture",evidence=["sanitized.txt"])
            with self.assertRaises(ValueError):validate(packet,root)
            device=packet["devices"]["minimum"]
            device.update(model="synthetic",os_build="synthetic",api=29,physical=True,secure_lock_configured=True,
                          source_revision="a"*40,app_apk_sha256="b"*64,test_apk_sha256="c"*64,signing_kind="test")
            self.assertEqual(validate(packet,root)["states"]["pass"],1)
            self.assertFalse(validate(packet,root)["certified"])
            for bad in ("missing.txt","../escaped.txt",str(artifact.resolve())):
                row["evidence"]=[bad]
                with self.assertRaises(ValueError):validate(packet,root)
            row["evidence"]=["sanitized.txt"];device["physical"]=False
            with self.assertRaises(ValueError):validate(packet,root)

    def test_result_fields_cannot_hide_under_not_run(self):
        packet=blank_packet();packet["scenarios"][0]["duration_seconds"]=10
        with self.assertRaises(ValueError):validate(packet)

    def test_current_support_needs_dated_existing_contained_reference(self):
        with tempfile.TemporaryDirectory(dir=ROOT/".work") as folder:
            root=Path(folder);(root/"support.md").write_text("Synthetic support decision",encoding="utf8")
            packet=blank_packet()
            packet.update(current_supported_api=36,current_support_reference="support.md",current_support_date="2026-09-26")
            for slot,device in packet["devices"].items():
                device.update(model="synthetic",os_build="synthetic",api=29 if slot=="minimum" else 36,
                              physical=True,secure_lock_configured=True,source_revision="a"*40,
                              app_apk_sha256="b"*64,test_apk_sha256="c"*64,signing_kind="test")
            for row in packet["scenarios"]:
                row.update(state="pass",date="2026-09-26",duration_seconds=5,
                           observation="Synthetic fixture only",evidence=["support.md"])
            result=validate(packet,root)
            self.assertTrue(result["ready_for_manual_evidence_review"])
            self.assertFalse(result["certified"])
            for bad in (None,"x","../support.md",str((root/"support.md").resolve()),"x"*513):
                packet["current_support_reference"]=bad
                with self.assertRaises(ValueError):validate(packet,root)
            packet["current_support_reference"]="support.md"
            for bad in (None,"yesterday","2026-02-30"):
                packet["current_support_date"]=bad
                with self.assertRaises(ValueError):validate(packet,root)
            packet["current_support_date"]="2026-09-26"
            packet["devices"]["current"]["api"]=35
            with self.assertRaises(ValueError):validate(packet,root)


if __name__=="__main__":unittest.main()
