---
id: "MC-023"
title: "Android GATT transport"
depends_on: ["MC-004","MC-016"]
kind: "android"
branch: "ticket/MC-023-android-gatt-transport"
---

# MC-023 — Android GATT transport

## Objective

Implement scanner/advertiser, GATT server/client, CCCD setup, concrete UUIDs and runtime write/notify flow control.

## Dependencies

`MC-004`, `MC-016` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/android/ble/**`, `tests/bench/android/**`, `docs/testing/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Implement scanner/advertiser, GATT server/client, CCCD setup, concrete UUIDs and runtime write/notify flow control.
- Feed per-direction capacity and link lifecycle events to the core; preserve contiguous fragment transfer and disconnect cleanup.
- Integrate the foreground service and approved permission manifest for supported Android versions.

## Exit criteria

- [ ] Automated native-adapter tests exchange whole/fragmented packets in both directions and exercise runtime capacity/backpressure through controlled callbacks; applicable Android build/lint/tests pass. Two-device physical exchange and measured backpressure remain mandatory in MC-025.
- [ ] Notify subscription, busy/error callbacks, stalled writes, MTU changes and disconnects recover as specified.
- [ ] The native driver forwards untrusted bytes without duplicating core parsing.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If a device exposes a smaller capacity, use the agreed supported-capacity behavior rather than oversized sends.
- On stalled links cancel bounded work and reconnect via policy; avoid unbounded retry loops.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Scheduling approved 2026-09-14 in [the validation policy](../../decisions/local-validation-policy.md#physical-acceptance-scheduling--approved-2026-09-14). Physical driver acceptance transfers to MC-025; implementation evidence must distinguish controlled callbacks/emulation from physical radio results.

Not implemented. Record commands, versions, reproducible inputs and results here. For manual/hardware checks include device/OS, duration and report paths. No test or review is claimed yet.

## Review and merge

- Branch: `ticket/MC-023-android-gatt-transport`.
- Review/PR: pending.
- Squash commit title: `MC-023: Android GATT transport`.
- Completion becomes effective only when the reviewed squash commit lands on main.
