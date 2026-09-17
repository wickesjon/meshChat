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

### Preparation checkpoint — 2026-09-17

All hard dependencies are complete on main; this branch starts from integrated candidate `e20a05e4c55df857de2dc8168258138bf2092fbc` (MC-035 squash). Prepare `docs/testing/MC-025-android-physical-acceptance.md` within the existing scope: a requirements-to-scenario matrix, preflight/evidence template, unchanged MC-007 measurement rules and explicit blockers. No production changes or acceptance relaxation is planned. Validate board generation/default, board unit tests, whitespace and local documentation links. Procedure preparation is not executed physical acceptance.

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

Physical execution is blocked: the read-only `adb devices -l` inventory on 2026-09-17 returned no devices after the isolated development emulator was stopped. The physical inventory question is pending. At least two real Android phones are needed to start, with Pixel/Samsung/Xiaomi and five-/ten-device cohorts required for the full gate. The [acceptance packet](../../testing/MC-025-android-physical-acceptance.md) is prepared; every physical scenario remains NOT RUN. Candidate-specific workload/measurement instrumentation must also be verified before execution; the existing emulator runners deliberately reject physical serials and must not be reused by removing that guard. No physical test, completed ticket or final review is claimed.

## Review and merge

- Branch: `ticket/MC-025-android-bench-mesh-and-early-field-gate`.
- Review/PR: pending.
- Squash commit title: `MC-025: Android integrated bench mesh and field gate`.
- Completion becomes effective only when the reviewed squash commit lands on main.

Preparation validation on 2026-09-17: ticketboard regeneration/default validation, all 12 board unit tests, acceptance-packet local-link checks and `git diff --check` pass. These validate documentation only. No PR is published and no final Terra or physical acceptance review is claimed while the gate is incomplete.
