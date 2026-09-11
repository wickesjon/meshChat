---
id: "MC-007"
title: "Budgets and measurable mesh acceptance"
depends_on: ["MC-004","MC-006"]
kind: "decision"
branch: "ticket/MC-007-budgets-and-measurable-mesh-acceptance"
---

# MC-007 — Budgets and measurable mesh acceptance

## Objective

Specify frame/byte/token accounting units, burst windows, per-link and global crypto budgets, queue byte limits and priority fairness.

## Dependencies

`MC-004`, `MC-006` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `docs/mesh-chat-design.md`, `docs/decisions/**`, `tests/simulator/scenarios/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Specify frame/byte/token accounting units, burst windows, per-link and global crypto budgets, queue byte limits and priority fairness.
- Reconcile SYNC eligible-set selection, item/byte limits, time target, mixed-channel relevance and deterministic Bloom false positives; cover maximum-size signed and encrypted traffic.
- Define TTL-reachable chain tests, GATT-send metrics, unsuppressed baselines, fixed traffic/loss/topology seeds and measured battery/latency thresholds.

## Exit criteria

- [ ] A budget worksheet demonstrates that the chosen SYNC test workload fits all simultaneous budgets including overhead and control traffic.
- [ ] Each simulator gate names denominator, sample duration, seeds, loss/churn parameters and pass threshold.
- [ ] Bounded-memory tables cover credentials, senders, orphans, reassembly, caches and outbound queues.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If the recovery target cannot fit safe budgets, narrow the byte-bounded eligible workload or explicitly extend time; do not silently relax ingress protection.
- If suppression gains are smaller than hoped, preserve reachable delivery and report the measured cost rather than forcing an unattainable relay ratio.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Not implemented. Record commands, versions, reproducible inputs and results here. For manual/hardware checks include device/OS, duration and report paths. No test or review is claimed yet.

## Review and merge

- Branch: `ticket/MC-007-budgets-and-measurable-mesh-acceptance`.
- Review/PR: pending.
- Squash commit title: `MC-007: Budgets and measurable mesh acceptance`.
- Completion becomes effective only when the reviewed squash commit lands on main.
