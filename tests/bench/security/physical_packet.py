"""Prepare/check MC-043 evidence records; never operates a device or certifies it."""
import argparse
from collections import Counter
from datetime import date
import json
from pathlib import Path
import re

ROOT=Path(__file__).resolve().parents[3]
SCENARIOS=("production_curves","wrapping_metadata","encrypted_create","restart_wrong_correct",
           "lock_closed","lock_inflight","held_handle_limit","reboot_before_unlock",
           "key_loss","os_invalidation","explicit_reset","cloud_backup","device_transfer",
           "same_device_restore","other_device_restore","uninstall_reinstall","sidecars_logging")
SLOTS=("minimum","current")
STATES=("not_run","unavailable","fail","pass")


def blank_packet():
    return {"schema_version":1,"current_supported_api":None,"current_support_reference":None,
            "devices":{slot:{"model":None,"os_build":None,"api":None,"physical":None,
                              "source_revision":None,"app_apk_sha256":None,"test_apk_sha256":None,
                              "signing_kind":None,"secure_lock_configured":None} for slot in SLOTS},
            "scenarios":[{"device":slot,"scenario":name,"state":"not_run","date":None,
                          "duration_seconds":None,"observation":"","evidence":[]} for slot in SLOTS for name in SCENARIOS]}


def validate(packet, root=ROOT):
    if packet.get("schema_version")!=1 or set(packet.get("devices",{}))!=set(SLOTS):
        raise ValueError("Unsupported schema or missing device slots")
    rows=packet.get("scenarios",[])
    keys=[(r.get("device"),r.get("scenario")) for r in rows]
    if len(keys)!=len(set(keys)) or set(keys)!={(slot,name) for slot in SLOTS for name in SCENARIOS}:
        raise ValueError("Missing, repeated or unknown physical scenario")
    for row in rows:
        state=row.get("state")
        if state not in STATES:raise ValueError("Unknown evidence state")
        if state=="not_run":
            if row.get("evidence") or row.get("date") is not None or row.get("duration_seconds") is not None:
                raise ValueError("Executed evidence cannot be labeled not_run")
            continue
        if not isinstance(row.get("observation"),str) or not row["observation"].strip():
            raise ValueError("Record an observation or an explicit unavailable reason")
        if state=="unavailable":continue
        device=packet["devices"][row["device"]]
        if device.get("physical") is not True or device.get("secure_lock_configured") is not True:
            raise ValueError("Executed physical results require a real, securely locked test device")
        for key in ("model","os_build","signing_kind"):
            if not isinstance(device.get(key),str) or not device[key].strip():raise ValueError("Missing device/build/signing metadata")
        if type(device.get("api")) is not int or device["api"]<29:raise ValueError("Invalid Android API")
        if row["device"]=="minimum" and device["api"]!=29:raise ValueError("Minimum matrix requires API 29")
        if row["device"]=="current":
            if type(packet.get("current_supported_api")) is not int or packet["current_supported_api"]<=29:
                raise ValueError("Declare current supported API for the test date")
            if device["api"]!=packet["current_supported_api"] or not packet.get("current_support_reference"):
                raise ValueError("Current device must match the dated support decision")
        for key,size in (("source_revision",40),("app_apk_sha256",64),("test_apk_sha256",64)):
            if not re.fullmatch(f"[0-9a-f]{{{size}}}",str(device.get(key,""))):raise ValueError("Missing exact source/APK identity")
        try:date.fromisoformat(row["date"])
        except (ValueError,TypeError,KeyError):raise ValueError("Executed result needs an ISO date") from None
        if type(row.get("duration_seconds")) not in (int,float) or not 0<row["duration_seconds"]<604800:
            raise ValueError("Executed result needs a positive, bounded duration")
        evidence=row.get("evidence")
        if not isinstance(evidence,list) or not evidence:raise ValueError("Executed result needs evidence")
        for name in evidence:
            if not isinstance(name,str) or Path(name).is_absolute():raise ValueError("Evidence must be repository-relative")
            path=(root/name).resolve()
            if not path.is_relative_to(root.resolve()) or not path.is_file():raise ValueError("Missing or escaped evidence path")
    counts=dict(Counter(r["state"] for r in rows))
    return {"schema_valid":True,"states":counts,"ready_for_manual_evidence_review":counts.get("pass")==len(rows),
            "certified":False,"note":"Valid structure is not verified evidence. MC-043 review, support decisions and merge gates remain authoritative."}


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command",choices=("init","check"))
    parser.add_argument("packet",type=Path)
    args=parser.parse_args();path=args.packet.resolve()
    if not path.is_relative_to(ROOT):raise ValueError("Packet must remain inside repository")
    if args.command=="init":
        if not path.is_relative_to(ROOT/".work") or path.exists():raise ValueError("Use a new packet under repository .work")
        path.parent.mkdir(parents=True,exist_ok=True)
        path.write_text(json.dumps(blank_packet(),indent=2)+"\n",encoding="utf8")
        print("Prepared 34 not-run records; no device action or certification performed.")
    else:
        if path.stat().st_size>1048576:raise ValueError("Packet exceeds bounded metadata size")
        print(json.dumps(validate(json.loads(path.read_text(encoding="utf-8-sig"))),indent=2))


if __name__=="__main__":main()
