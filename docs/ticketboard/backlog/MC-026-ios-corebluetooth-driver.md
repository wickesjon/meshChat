---
id: "MC-026"
title: "iOS CoreBluetooth driver"
depends_on: ["MC-004","MC-016"]
kind: "ios"
branch: "ticket/MC-026-ios-corebluetooth-driver"
---

# MC-026 — iOS CoreBluetooth driver

## Objective

Implement central/peripheral roles, write-readiness and notify-readiness flow control, restoration and per-direction capacities.

## Dependencies

`MC-004`, `MC-016` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/ios/BLE/**`, `tests/bench/ios/**`, `docs/testing/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Implement central/peripheral roles, write-readiness and notify-readiness flow control, restoration and per-direction capacities.
- Support the asymmetric discovery strategy and established-link recovery with bounded reconnect state.
- Connect native lifecycle events to the shared core without assuming background execution is continuous.

## Exit criteria

- [ ] Real devices exchange all base frame kinds in both roles within runtime value limits.
- [ ] Background, suspension, disconnect, restoration and foreground catch-up produce recorded expected outcomes.
- [ ] Unsupported discovery/force-quit cases are distinguished from regressions.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If background participation is intermittent, retain foreground catch-up and honest degraded status.
- If an achievable discovery pairing fails, block interop and revise the measured transport contract rather than claiming parity.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Not implemented. Record commands, versions, reproducible inputs and results here. For manual/hardware checks include device/OS, duration and report paths. No test or review is claimed yet.

## Review and merge

- Branch: `ticket/MC-026-ios-corebluetooth-driver`.
- Review/PR: pending.
- Squash commit title: `MC-026: iOS CoreBluetooth driver`.
- Completion becomes effective only when the reviewed squash commit lands on main.
