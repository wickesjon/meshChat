"""Summarize passive samples. Charge loss is whole-device, not app-only energy.

Usage: python resource_analysis.py run.log .work/<run>/resources.json
"""
import json
import math
import re
from pathlib import Path
import sys


def analyze(text):
    rows=[];malformed=False
    for line in text.splitlines():
        if "MC025R " in line:
            try:rows.append(json.loads(line.split("MC025R ",1)[1]))
            except json.JSONDecodeError:malformed=True
    plans=[r for r in rows if r.get("event")=="plan"]
    summaries=[r for r in rows if r.get("event")=="summary"]
    samples=[r for r in rows if r.get("event")=="sample"]
    if len(plans)!=1 or len(summaries)>1:
        raise ValueError("Missing/duplicate plan or summary")
    plan=plans[0];seconds=plan["duration_seconds"]
    if type(seconds) is not int or not 60<=seconds<=21600 or plan.get("sample_seconds")!=60:
        raise ValueError("Invalid resource plan")
    if any(r.get("run")!=plan["run"] or r.get("workload")!=plan["workload"] for r in rows):
        raise ValueError("Mixed run/workload")
    if [r["index"] for r in samples]!=list(range(len(samples))):
        raise ValueError("Missing/repeated/out-of-order samples")
    issues=[]
    if any(not re.fullmatch(r"[0-9a-f]{64}",str(plan.get(k,""))) for k in ("apk_sha256","test_apk_sha256")):
        issues.append("missing/invalid app or instrumentation APK identity")
    if malformed:issues.append("truncated/malformed log record")
    if "OK (1 test)" not in text or "FAILURES!!!" in text or "INSTRUMENTATION_FAILED" in text:
        issues.append("runner did not pass")
    if len(samples)!=math.ceil(seconds/60)+1 or not summaries or summaries[0].get("samples")!=len(samples):
        issues.append("incomplete sample coverage")
    result={"run":plan["run"],"workload":plan["workload"],"apk_sha256":plan.get("apk_sha256"),"test_apk_sha256":plan.get("test_apk_sha256"),
            "samples":len(samples),"issues":issues,"comparable":False,"battery_drain_available":False}
    if len(samples)<2:
        issues.append("fewer than two samples")
        return result
    first,last=samples[0],samples[-1]
    elapsed=last["elapsed_ms"]-first["elapsed_ms"]
    if elapsed<=0 or any(b["elapsed_ms"]<=a["elapsed_ms"] for a,b in zip(samples,samples[1:])):
        raise ValueError("Invalid monotonic sample interval")
    if first["elapsed_ms"]>5000 or last["elapsed_ms"]<seconds*1000 or elapsed>seconds*1000+60000 or any(b["elapsed_ms"]-a["elapsed_ms"]>90000 for a,b in zip(samples,samples[1:])):
        issues.append("late/suspended samples or duration mismatch")
    for key in ("cpu_ms","completed_tasks"):
        if any(b[key]<a[key] for a,b in zip(samples,samples[1:])):
            issues.append(f"{key} counter reset")
    if any(not r["workload_valid"] for r in samples):issues.append("workload changed/unavailable")
    if len({r["screen_interactive"] for r in samples})!=1:issues.append("screen state changed")
    if len({r["plugged"] for r in samples})!=1 or any(r["plugged"]<0 for r in samples):issues.append("power source changed/unknown")
    if plan["workload"]=="beacon" and any(r["plugged"]==0 for r in samples):issues.append("powered Beacon workload lost external power")
    result.update(elapsed_seconds=elapsed/1000,process_cpu_ms=last["cpu_ms"]-first["cpu_ms"],
                  cpu_percent_one_core=100*(last["cpu_ms"]-first["cpu_ms"])/elapsed,
                  model_tasks=last["completed_tasks"]-first["completed_tasks"],
                  max_sampled_queue=max(r["queued_tasks"] for r in samples),
                  max_sampled_pss_kib=max(r["pss_kib"] for r in samples),
                  screen_interactive=first["screen_interactive"],plugged=first["plugged"])
    result["transport_deltas"]={}
    for key in ("scheduled_frames","received_frames","stats_elapsed_ms"):
        values=[int(r[key]) if r.get(key) is not None else None for r in samples]
        if any(v is None for v in values):
            result["transport_deltas"][key]=None
        elif any(b<a for a,b in zip(values,values[1:])):
            issues.append(f"{key} counter reset")
            result["transport_deltas"][key]=None
        else:result["transport_deltas"][key]=values[-1]-values[0]
    if plan["workload"]=="messaging":
        expected=plan.get("expected_test_messages")
        counts=[r.get("observed_test_messages") for r in samples]
        result["expected_test_messages"]=expected
        result["observed_test_messages"]=counts[-1]
        if (type(expected) is not int or not 1<=expected<=1000 or
            any(type(n) is not int or not 0<=n<=expected for n in counts) or
            counts[0]!=0 or counts[-1]!=expected or any(b<a for a,b in zip(counts,counts[1:]))):
            issues.append("declared synthetic messaging workload incomplete or stale")
    result["comparable"]=not issues
    charge=[r.get("charge_uah") for r in samples]
    percent=[r.get("battery_percent") for r in samples]
    valid_charge=all(type(v) is int and v>0 for v in charge) and all(b<=a for a,b in zip(charge,charge[1:]))
    valid_percent=all(type(v) is int and 0<=v<=100 for v in percent) and all(b<=a for a,b in zip(percent,percent[1:]))
    result["battery_drain_available"]=not issues and seconds>=1800 and all(r["plugged"]==0 for r in samples) and valid_charge and valid_percent
    if result["battery_drain_available"]:
        result["whole_device_charge_loss_uah"]=charge[0]-charge[-1]
        result["whole_device_charge_loss_uah_per_hour"]=(charge[0]-charge[-1])*3600000/elapsed
        result["battery_percentage_points_lost"]=percent[0]-percent[-1]
    result["note"]="CPU includes instrumentation; memory is sampled. Battery gauge is coarse whole-device evidence, not app-only energy or a passed power gate. Use matched repeated runs; powered Beacon has no drain result."
    return result


if __name__=="__main__":
    result=analyze(Path(sys.argv[1]).read_text(encoding="utf-8-sig"))
    output=Path(sys.argv[2]).resolve();root=Path(__file__).resolve().parents[3]
    if not output.is_relative_to(root/".work") or output.exists():raise ValueError("Use a new file inside repository .work")
    output.parent.mkdir(parents=True,exist_ok=True)
    output.write_text(json.dumps(result,indent=2)+"\n",encoding="utf8")
    print(f"comparable={result['comparable']}; battery_drain_available={result['battery_drain_available']}")
