"""Aggregate opted-in MC-025 traces without subtracting device clocks.

Usage: python latency_analysis.py A.log B.log output.json
Raw logs/output must remain in ignored repository-local evidence directories.
"""
import json
from pathlib import Path
from statistics import median
import sys


def read(path):
    text = Path(path).read_text(encoding="utf-8-sig")
    if "OK (1 test)" not in text or "FAILURES!!!" in text:
        raise ValueError(f"Runner did not pass: {path}")
    ordinary, traces = [], []
    for line in text.splitlines():
        for prefix, target in [("MC025 ", ordinary), ("MC025L ", traces)]:
            if prefix in line:
                target.append(json.loads(line.split(prefix, 1)[1]))
    return ordinary, traces


def one(rows, event, index=None, echo=None):
    found = [r for r in rows if r["event"] == event
             and (index is None or r.get("index") == index)
             and (echo is None or r.get("echo") == echo)]
    if len(found) != 1:
        raise ValueError(f"Expected one {event}/{index}/{echo}, found {len(found)}")
    return found[0]


def frames(rows, index, echo):
    found = sorted((r for r in rows if r["event"] == "egress"
                    and r["index"] == index and r["echo"] == echo), key=lambda r: r["mono_ms"])
    if not found:
        raise ValueError(f"Missing outgoing frames: {index}/{echo}")
    count = found[0]["fragments"]
    if len(found) != count or [r["fragment"] for r in found] != list(range(count)):
        raise ValueError("Missing/repeated/reordered submissions need separate attribution")
    for r in found:
        if not (r["allowed"] and r["native_accepted"] and r["fragments"] == count
                and r["mono_ms"] <= r["submit_ms"] <= r["submit_end_ms"] <= r["end_ms"]):
            raise ValueError("Refused or invalid frame timing; cannot attribute a complete pair")
    return found


def interval(end, start):
    value = end - start
    if value < 0:
        raise ValueError("Negative same-clock interval")
    return value


def pair(a, b, index):
    request = one(a, "request_start", index, False)["mono_ms"]
    observed = one(a, "observed", index, True)["mono_ms"]
    a_send = one(a, "send_call", index, False)["mono_ms"]
    b_rx = one(b, "native_receive", index, False)["mono_ms"]
    b_observed = one(b, "observed", index, False)["mono_ms"]
    b_send = one(b, "send_call", index, True)["mono_ms"]
    a_rx = one(a, "native_receive", index, True)["mono_ms"]
    af, bf = frames(a, index, False), frames(b, index, True)
    parts = {
        "request_to_first_submit_ms": interval(af[0]["submit_ms"], request),
        "request_fragment_span_ms": interval(af[-1]["submit_ms"], af[0]["submit_ms"]),
        "responder_receive_to_observed_ms": interval(b_observed, b_rx),
        "responder_observed_to_send_ms": interval(b_send, b_observed),
        "echo_call_to_first_submit_ms": interval(bf[0]["submit_ms"], b_send),
        "echo_fragment_span_ms": interval(bf[-1]["submit_ms"], bf[0]["submit_ms"]),
        "initiator_receive_to_observed_ms": interval(observed, a_rx),
    }
    rtt = interval(observed, request)
    residual = rtt - sum(parts.values())
    if residual < 0:
        raise ValueError("Negative cross-leg residual; causal attribution invalid")
    parts["delivery_callback_residual_ms"] = residual
    diagnostics = {"initiator_dispatch_ms": interval(a_send, request)}
    for label, rows, echo, sent, rx in [("A", a, False, a_send, a_rx), ("B", b, True, b_send, b_rx)]:
        before = one(rows, "send_queue_before", index, echo)["mono_ms"]
        after = one(rows, "send_queue_after", index, echo)["mono_ms"]
        receive_before = one(rows, "receive_queue_before", index, not echo)["mono_ms"]
        receive_after = one(rows, "receive_queue_after", index, not echo)["mono_ms"]
        diagnostics[f"{label}_send_queue_wait_ms"] = interval(before, sent)
        diagnostics[f"{label}_send_work_bracket_ms"] = interval(after, before)
        diagnostics[f"{label}_receive_queue_wait_ms"] = interval(receive_before, rx)
        diagnostics[f"{label}_receive_work_bracket_ms"] = interval(receive_after, receive_before)
    return {"index": index, "rtt_ms": rtt, "parts": parts, "diagnostics": diagnostics,
            "request_frames": len(af), "echo_frames": len(bf),
            "protected_egress_wait_ms": [r["submit_ms"]-r["mono_ms"] for r in af+bf],
            "native_submit_call_ms": [r["submit_end_ms"]-r["submit_ms"] for r in af+bf]}


def aggregate(a_log, b_log):
    ao, a = read(a_log)
    bo, b = read(b_log)
    run = one(ao, "summary")["run"]
    for role, ordinary in [("A", ao), ("B", bo)]:
        summary = one(ordinary, "summary")
        if summary["role"] != role or summary["run"] != run or summary["completed_pairs"] != 7:
            raise ValueError("Run/role/outcome mismatch")
    rtts = [one(ao, "roundtrip", i) for i in range(7)]
    if not all(r["received"] and r["own_accepted"] for r in rtts):
        raise ValueError("Refusals/timeouts must not be dropped or counted as complete")
    if not one(bo, "background_receive")["background"]:
        raise ValueError("Background sample was not backgrounded")
    result = {"run": run, "directed_deliveries": 14,
              "rtt_ms": [r["elapsed_ms"] for r in rtts],
              "short_median_ms": median(r["elapsed_ms"] for r in rtts[:3]),
              "long_median_ms": median(r["elapsed_ms"] for r in rtts[3:6])}
    if not a and not b:
        return result
    for role, rows in [("A", a), ("B", b)]:
        summary = one(rows, "trace_summary")
        if summary["dropped"] or summary["entries"] != len(rows)-1:
            raise ValueError("Missing or overflowed trace")
        if any(r["role"] != role or r["run"] != run for r in rows):
            raise ValueError("Mixed trace run/role")
    pairs = [pair(a, b, i) for i in range(7)]
    if [p["rtt_ms"] for p in pairs] != result["rtt_ms"]:
        raise ValueError("Trace RTT differs from original observation window")
    result["pairs"] = pairs
    result["storage_baseline_ms"] = {}
    for role, rows in [("A", a), ("B", b)]:
        result["storage_baseline_ms"][role] = {
            event: [one(rows, event, i)["duration_ms"] for i in range(5)]
            for event in ["protected_read", "native_status"]}
    for label, selected in [("short", pairs[:3]), ("long", pairs[3:6]), ("background", pairs[6:])]:
        result[label+"_parts_mean_ms"] = {
            key: sum(p["parts"][key] for p in selected)/len(selected) for key in pairs[0]["parts"]}
    return result


if __name__ == "__main__":
    result = aggregate(sys.argv[1], sys.argv[2])
    Path(sys.argv[3]).write_text(json.dumps(result, indent=2)+"\n", encoding="utf-8")
    print(json.dumps(result, indent=2))
