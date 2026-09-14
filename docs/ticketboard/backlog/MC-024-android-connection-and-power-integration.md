---
id: "MC-024"
title: "Android connection and power integration"
depends_on: ["MC-023","MC-014"]
kind: "android"
branch: "ticket/MC-024-android-connection-and-power-integration"
---

# MC-024 — Android connection and power integration

## Objective

Implement post-handshake duplicate-link resolution, bounded connection slots, novelty/RSSI selection and idle-peer eviction.

## Dependencies

`MC-023`, `MC-014` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/android/ble/**`, `src/android/power/**`, `tests/bench/android/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Implement post-handshake duplicate-link resolution, bounded connection slots, novelty/RSSI selection and idle-peer eviction.
- Wire core Auto/Saver/Normal policy, isolated scanning backoff, charging and permission changes to Android lifecycle APIs.
- Exercise reconnect churn and injected service-termination callbacks with explicit degraded states; retain actual OEM termination scenarios for MC-025.

## Exit criteria

- [ ] Duplicate simultaneous connections settle deterministically without blocking asymmetric discovery.
- [ ] Slot-exhaustion and reconnection tests obey device limits and do not accumulate stale resources.
- [ ] Automated lifecycle/permission/service-stop and threshold tests pass with applicable native checks. Pixel/Samsung/Xiaomi permission, screen-off, OEM termination and measured connection-limit evidence is retained in MC-025.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If an OEM cannot sustain target links, use a measured lower connection target and record coverage consequences.
- If the OS kills service work, expose stopped/degraded operation and recovery instructions rather than implying continued relay.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Scheduling approved 2026-09-14 in [the validation policy](../../decisions/local-validation-policy.md#physical-acceptance-scheduling--approved-2026-09-14). Physical OEM acceptance transfers to MC-025; native build checks and bounded-state regression tests remain implementation gates.

Not implemented. Record commands, versions, reproducible inputs and results here. For manual/hardware checks include device/OS, duration and report paths. No test or review is claimed yet.

## Review and merge

- Branch: `ticket/MC-024-android-connection-and-power-integration`.
- Review/PR: pending.
- Squash commit title: `MC-024: Android connection and power integration`.
- Completion becomes effective only when the reviewed squash commit lands on main.
