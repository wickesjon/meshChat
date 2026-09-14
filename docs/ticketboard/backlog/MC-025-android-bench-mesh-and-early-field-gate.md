---
id: "MC-025"
title: "Android integrated bench mesh and field gate"
depends_on: ["MC-024","MC-015","MC-029","MC-030","MC-031","MC-032","MC-033","MC-042","MC-022"]
kind: "gate"
branch: "ticket/MC-025-android-bench-mesh-and-early-field-gate"
---

# MC-025 — Android integrated bench mesh and field gate

## Objective

Run two-, five- and ten-device mixed-OEM trials with controlled topology, late joining, churn and an identity-rotating flooder.

## Dependencies

`MC-024`, `MC-015`, `MC-029`, `MC-030`, `MC-031`, `MC-032`, `MC-033`, `MC-042`, `MC-022` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `tests/bench/android/**`, `tests/bench/beacon/**`, `docs/testing/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Run two-, five- and ten-device mixed-OEM trials with controlled topology, late joining, churn and an identity-rotating flooder.
- Measure actual GATT transmissions, reachable delivery, latency and battery under the MC-007 workload.
- Compare radio outcomes with simulator predictions and record a small field trial on the integrated Android candidate before MC-034 beta acceptance.

## Exit criteria

- [ ] Deferred MC-023 two-device whole/fragmented bidirectional exchange, runtime capacity and measured backpressure pass; notify subscription, busy/error/stalled callbacks, MTU changes and disconnect recovery are covered on actual devices.
- [ ] Deferred MC-024 Pixel/Samsung/Xiaomi permission, screen-off, OEM service termination, power-threshold, duplicate-link and measured slot/reconnection-limit results pass with explicit degraded states.
- [ ] Deferred MC-033 powered-phone endurance runs six wall-clock hours with bounded memory and no manual recovery; actual power removal/downgrade preserves core state. Sparse-gap beacon/no-beacon trials record coverage, relay work and phone battery/load changes without assuming offload.
- [ ] Recorded bench results meet the approved targets or have resolved deviations with updated evidence.
- [ ] SYNC and bridge tests succeed within the supported hop/capacity model.
- [ ] Battery and flood results include device/OS, duration, baseline and instrumentation limitations.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If field behavior differs from the simulator, fix the model or implementation and rerun the affected scenarios.
- Do not replace missing hardware evidence with simulated numbers or widen acceptance silently.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Scheduling approved 2026-09-14 in [the validation policy](../../decisions/local-validation-policy.md#physical-acceptance-scheduling--approved-2026-09-14). Near completion means the listed Android feature implementations and full-wire gate are complete, before MC-034 acceptance or distribution. This physical gate is deferred, not satisfied; use synthetic messages/identities until MC-043 permits sensitive-data use. Original MC-007 workload/thresholds and all original two/five/ten-device scenarios remain required.

Not implemented. Record commands, versions, reproducible inputs and results here. For manual/hardware checks include device/OS, duration and report paths. No test or review is claimed yet.

## Review and merge

- Branch: `ticket/MC-025-android-bench-mesh-and-early-field-gate`.
- Review/PR: pending.
- Squash commit title: `MC-025: Android integrated bench mesh and field gate`.
- Completion becomes effective only when the reviewed squash commit lands on main.
