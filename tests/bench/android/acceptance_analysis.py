"""Account for every scheduled acceptance attempt, including failed/partial runs.

Usage: python acceptance_analysis.py A.log B.log .work/<run>/summary.json
Run again with initiator B; never subtract timestamps from different devices.
"""
import json
import math
from pathlib import Path
from statistics import median
import sys

from latency_analysis import frames


def read(path):
    text = Path(path).read_text(encoding="utf-8-sig")
    ordinary, traces = [], []
    malformed = False
    for line in text.splitlines():
        for prefix, target in [("MC025 ", ordinary), ("MC025L ", traces)]:
            if prefix in line:
                try:
                    target.append(json.loads(line.split(prefix, 1)[1]))
                except json.JSONDecodeError:
                    malformed = True
    passed = not malformed and "OK (1 test)" in text and "FAILURES!!!" not in text and "INSTRUMENTATION_FAILED" not in text
    return ordinary, traces, passed


def unique(rows, event, index=None):
    found = [r for r in rows if r.get("event") == event and (index is None or r.get("index") == index)]
    if len(found) > 1:
        raise ValueError(f"Duplicate {event}/{index}; ambiguous evidence")
    return found[0] if found else None


def distribution(values):
    ordered = sorted(values)
    return {"successful_samples": len(values), "median_ms": median(values) if values else None,
            "p95_ms": ordered[math.ceil(.95*len(ordered))-1] if values else None,
            "max_ms": max(values) if values else None}


def aggregate(a, b):
    inputs = {"A": read(a), "B": read(b)}
    plans = [unique(rows, "plan") for rows, _, _ in inputs.values()]
    plan = next((p for p in plans if p), None)
    if not plan or plan.get("mode") != "acceptance" or plan.get("scheduled_pairs") != 60 or plan.get("initiator") not in inputs:
        raise ValueError("Missing or unsupported acceptance plan")
    issues = []
    for role, (rows, traces, passed) in inputs.items():
        if any(r.get("role") != role or r.get("run") != plan["run"] for r in rows+traces):
            raise ValueError("Mixed run/role evidence")
        local = unique(rows, "plan")
        if local is None:
            issues.append(f"{role}: missing plan")
        elif any(local.get(k) != plan.get(k) for k in ("mode", "initiator", "scheduled_pairs", "traced", "poll_ms")):
            raise ValueError("Endpoint plans disagree")
        summary = unique(rows, "summary")
        if not passed or not summary or summary.get("completed_pairs") != 60 or summary.get("scheduled_pairs") != 60:
            issues.append(f"{role}: runner/summary incomplete or failed")
        if any(r.get("event") in ("request", "roundtrip", "request_received", "echo") and
               (type(r.get("index")) is not int or not 0 <= r["index"] < 60) for r in rows):
            raise ValueError("Out-of-plan sample")
        if plan.get("traced"):
            trace_summary = unique(traces, "trace_summary")
            if not trace_summary or trace_summary.get("dropped") != 0 or trace_summary.get("entries") != len(traces)-1:
                issues.append(f"{role}: trace incomplete/overflowed")
        elif traces:
            raise ValueError("Unexpected traces in untraced plan")
    initiator = plan["initiator"]
    responder = "B" if initiator == "A" else "A"
    own, own_trace, _ = inputs[initiator]
    remote, remote_trace, _ = inputs[responder]
    samples = []
    for i in range(60):
        size = "long" if i % 2 else "short"
        request, result = unique(own, "request", i), unique(own, "roundtrip", i)
        echo, received = unique(remote, "echo", i), unique(remote, "request_received", i)
        status = "not_attempted"
        elapsed = None
        if request:
            status = "interrupted"
            if not request.get("ready"):
                status = "not_ready"
            elif result:
                elapsed = result.get("elapsed_ms")
                if type(elapsed) is not int or elapsed < 0 or result.get("size") != size:
                    raise ValueError("Invalid RTT or sample size")
                status = "observed" if result.get("received") and result.get("own_accepted") else "timeout_or_refused"
                if status == "observed" and not (received and echo and echo.get("native_complete")):
                    status = "unconfirmed_peer_evidence"
        sample = dict(index=i, size=size, status=status, observed_rtt_ms=elapsed,
                      request_frames=None, echo_frames=None)
        if plan.get("traced") and status == "observed":
            try:
                sample["request_frames"] = len(frames(own_trace, i, False))
                sample["echo_frames"] = len(frames(remote_trace, i, True))
            except (ValueError, KeyError):
                issues.append(f"sample {i}: frame attribution unavailable")
        samples.append(sample)
    stats = {size: distribution([s["observed_rtt_ms"] for s in samples if s["size"] == size and s["status"] == "observed"])
             for size in ("short", "long")}
    complete = not issues and all(s["status"] == "observed" for s in samples)
    return {"run": plan["run"], "initiator": initiator, "traced": bool(plan.get("traced")),
            "scheduled_pairs": 60, "observed_pairs": sum(s["status"] == "observed" for s in samples),
            "complete": complete, "issues": issues, "successful_observations": stats,
            "typical_subsecond_target_met": complete and all(v["median_ms"] < 1000 for v in stats.values()),
            "samples": samples, "note": "100 ms polling; successful-only statistics never hide missing scheduled attempts. Run both initiator orientations separately."}


if __name__ == "__main__":
    result = aggregate(sys.argv[1], sys.argv[2])
    output = Path(sys.argv[3]).resolve()
    root = Path(__file__).resolve().parents[3]
    if not output.is_relative_to(root / ".work") or output.exists():
        raise ValueError("Use a new output file inside repository .work")
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(result, indent=2)+"\n", encoding="utf-8")
    print(f"{result['observed_pairs']}/60 observed; complete={result['complete']}; target={result['typical_subsecond_target_met']}")
