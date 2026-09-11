---
id: "MC-004"
title: "Two-platform BLE and permission feasibility"
depends_on: ["MC-003"]
kind: "spike"
branch: "ticket/MC-004-two-platform-ble-and-permission-feasibility"
---

# MC-004 — Two-platform BLE and permission feasibility

## Objective

Build temporary direct-link probes in both GATT roles using proposed service/characteristic UUIDs.

## Dependencies

`MC-003` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/android/ble/**`, `src/ios/BLE/**`, `tests/bench/**`, `docs/decisions/**`, `docs/mesh-chat-design.md`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Build temporary direct-link probes in both GATT roles using proposed service/characteristic UUIDs.
- Measure write and notify capacities separately, backpressure, low-MTU behavior, duplicate connections, foreground/background discovery, held-link survival and reconnects.
- Record devices, OS versions, durations, traces and the Android permission/RSSI decision; include suspended and force-quit states as separate outcomes.

## Exit criteria

- [ ] Recorded Android/Android, Android/iOS and iOS/iOS results distinguish discovery from established connections.
- [ ] Both directions have measured capacity and flow-control evidence, including failure/recovery behavior.
- [ ] A permission decision and supported deployment matrix are committed; unsupported cases are explicit.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If no Mac/iPhone or physical Android devices are available, mark hardware evidence blocked; simulation cannot replace the spike.
- If low-capacity links cannot fit the agreed maximum packet, propose a supported-link floor or a versioned fragmentation change before MC-006 closes.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Not implemented. Record commands, versions, reproducible inputs and results here. For manual/hardware checks include device/OS, duration and report paths. No test or review is claimed yet.

## Review and merge

- Branch: `ticket/MC-004-two-platform-ble-and-permission-feasibility`.
- Review/PR: pending.
- Squash commit title: `MC-004: Two-platform BLE and permission feasibility`.
- Completion becomes effective only when the reviewed squash commit lands on main.
