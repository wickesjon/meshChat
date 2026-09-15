---
id: "MC-033"
title: "Android phone Beacon Mode"
depends_on: ["MC-024","MC-018","MC-028"]
kind: "product"
branch: "ticket/MC-033-android-phone-beacon-mode"
---

# MC-033 — Android phone Beacon Mode

## Objective

Implement the beacon profile with bounded anti-abuse controls, extended cache, runtime-enforced connection limits and external-power infra hint.

## Dependencies

`MC-024`, `MC-018`, `MC-028` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/android/power/**`, `src/android/ui/**`, `tests/bench/beacon/**`, `docs/testing/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Implement the beacon profile with bounded anti-abuse controls, extended cache, runtime-enforced connection limits and external-power infra hint.
- Build dim burn-in-safe status, hold-to-exit, auto-beacon charging behavior and battery auto-downgrade.
- Target retired Android phones for v1; provide the bridge/load measurement procedure for MC-025 without assuming battery offload or measured device capacity during implementation.

## Exit criteria

- [ ] Automated native/core integration exercises six logical hours of beacon scheduling/cache operation with bounded state; applicable native checks pass. MC-025 retains the six-hour powered physical-phone endurance test.
- [ ] Disconnecting power and reaching the downgrade threshold produce the approved behavior without lost core state.
- [ ] A reproducible sparse-gap, beacon/no-beacon comparison procedure and aggregate instrumentation are prepared for MC-025; physical coverage and phone battery/load results remain pending there.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If a device cannot support eight links, use its measured limit and document deployment capacity.
- ESP32, LoRa and backbone work remain follow-on scope; do not fork the protocol for this ticket.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Scheduling approved 2026-09-14 in [the validation policy](../../decisions/local-validation-policy.md#physical-acceptance-scheduling--approved-2026-09-14). Six-hour endurance, actual power removal/downgrade and sparse-gap/battery evidence transfer to MC-025. Synthetic time progression is not a physical endurance measurement.

Not implemented. Record commands, versions, reproducible inputs and results here. For manual/hardware checks include device/OS, duration and report paths. No test or review is claimed yet.

## Review and merge

- Branch: `ticket/MC-033-android-phone-beacon-mode`.
- Review/PR: pending.
- Squash commit title: `MC-033: Android phone Beacon Mode`.
- Completion becomes effective only when the reviewed squash commit lands on main.
