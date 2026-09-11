---
id: "MC-012"
title: "Deterministic GATT-overlay simulator"
depends_on: ["MC-003","MC-007"]
kind: "core"
branch: "ticket/MC-012-deterministic-gatt-overlay-simulator"
---

# MC-012 — Deterministic GATT-overlay simulator

## Objective

Build an event-driven simulator using the actual sans-IO core, directed link capacities, bounded queues and per-egress transmissions.

## Dependencies

`MC-003`, `MC-007` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `tests/simulator/**`, `tests/integration/simulator/**`, `docs/testing/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Build an event-driven simulator using the actual sans-IO core, directed link capacities, bounded queues and per-egress transmissions.
- Support loss, mobility, churn, partitions, mixed versions, identity rotation, clock skew and power tiers.
- Commit reproducible dense, sparse, cut-vertex and late-join scenario definitions and machine-readable metrics.

## Exit criteria

- [ ] A seed reproduces event traces and metrics; link budgets and fragmentation costs match the transport contract.
- [ ] Scenarios count each actual egress transmission and distinguish origin sends, relays, fragments and delivery.
- [ ] The same harness can run an unsuppressed reference policy without changing production protocol behavior.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If runtime grows too large, keep a small deterministic PR suite and schedule a larger fixed-seed sweep.
- Do not substitute a broadcast-radio model for point-to-point GATT to improve delivery numbers.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Not implemented. Record commands, versions, reproducible inputs and results here. For manual/hardware checks include device/OS, duration and report paths. No test or review is claimed yet.

## Review and merge

- Branch: `ticket/MC-012-deterministic-gatt-overlay-simulator`.
- Review/PR: pending.
- Squash commit title: `MC-012: Deterministic GATT-overlay simulator`.
- Completion becomes effective only when the reviewed squash commit lands on main.
