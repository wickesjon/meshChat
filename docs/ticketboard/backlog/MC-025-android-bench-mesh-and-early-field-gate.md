---
id: "MC-025"
title: "Android bench mesh and early field gate"
depends_on: ["MC-024","MC-015"]
kind: "gate"
branch: "ticket/MC-025-android-bench-mesh-and-early-field-gate"
---

# MC-025 — Android bench mesh and early field gate

## Objective

Run two-, five- and ten-device mixed-OEM trials with controlled topology, late joining, churn and an identity-rotating flooder.

## Dependencies

`MC-024`, `MC-015` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `tests/bench/android/**`, `docs/testing/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Run two-, five- and ten-device mixed-OEM trials with controlled topology, late joining, churn and an identity-rotating flooder.
- Measure actual GATT transmissions, reachable delivery, latency and battery under the MC-007 workload.
- Compare radio outcomes with simulator predictions and record an early small field trial before full app completion.

## Exit criteria

- [ ] Recorded bench results meet the approved targets or have resolved deviations with updated evidence.
- [ ] SYNC and bridge tests succeed within the supported hop/capacity model.
- [ ] Battery and flood results include device/OS, duration, baseline and instrumentation limitations.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If field behavior differs from the simulator, fix the model or implementation and rerun the affected scenarios.
- Do not replace missing hardware evidence with simulated numbers or widen acceptance silently.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Not implemented. Record commands, versions, reproducible inputs and results here. For manual/hardware checks include device/OS, duration and report paths. No test or review is claimed yet.

## Review and merge

- Branch: `ticket/MC-025-android-bench-mesh-and-early-field-gate`.
- Review/PR: pending.
- Squash commit title: `MC-025: Android bench mesh and early field gate`.
- Completion becomes effective only when the reviewed squash commit lands on main.
