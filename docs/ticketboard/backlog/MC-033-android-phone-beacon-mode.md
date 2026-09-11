---
id: "MC-033"
title: "Android phone Beacon Mode"
depends_on: ["MC-024","MC-018","MC-028"]
kind: "product"
branch: "ticket/MC-033-android-phone-beacon-mode"
---

# MC-033 — Android phone Beacon Mode

## Objective

Implement the beacon profile with bounded anti-abuse controls, extended cache, measured device connection limits and external-power infra hint.

## Dependencies

`MC-024`, `MC-018`, `MC-028` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/android/power/**`, `src/android/ui/**`, `tests/bench/beacon/**`, `docs/testing/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Implement the beacon profile with bounded anti-abuse controls, extended cache, measured device connection limits and external-power infra hint.
- Build dim burn-in-safe status, hold-to-exit, auto-beacon charging behavior and battery auto-downgrade.
- Use retired Android phones for v1; measure bridge coverage and per-phone relay work rather than assuming battery offload.

## Exit criteria

- [ ] A powered phone runs six hours with bounded memory and no manual recovery.
- [ ] Disconnecting power and reaching the downgrade threshold produce the approved behavior without lost core state.
- [ ] A sparse-gap test records the coverage benefit and any phone battery/load change.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If a device cannot support eight links, use its measured limit and document deployment capacity.
- ESP32, LoRa and backbone work remain follow-on scope; do not fork the protocol for this ticket.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Not implemented. Record commands, versions, reproducible inputs and results here. For manual/hardware checks include device/OS, duration and report paths. No test or review is claimed yet.

## Review and merge

- Branch: `ticket/MC-033-android-phone-beacon-mode`.
- Review/PR: pending.
- Squash commit title: `MC-033: Android phone Beacon Mode`.
- Completion becomes effective only when the reviewed squash commit lands on main.
