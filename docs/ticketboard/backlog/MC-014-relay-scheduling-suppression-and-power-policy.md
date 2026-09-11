---
id: "MC-014"
title: "Relay scheduling, suppression and power policy"
depends_on: ["MC-013","MC-012"]
kind: "core"
branch: "ticket/MC-014-relay-scheduling-suppression-and-power-policy"
---

# MC-014 — Relay scheduling, suppression and power policy

## Objective

Implement TTL semantics, per-egress knowledge and default-to-relay when no coverage evidence exists.

## Dependencies

`MC-013`, `MC-012` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/core/**`, `tests/simulator/**`, `tests/integration/relay/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Implement TTL semantics, per-egress knowledge and default-to-relay when no coverage evidence exists.
- Schedule priorities, bounded own-message admission, hold-offs, fairness, disconnect cleanup and frame-preserving flushes.
- Implement Normal/Saver/Auto and beacon policy parameters without turning advisory peer or infra hints into trust.

## Exit criteria

- [ ] TTL-reachable chain and cut-vertex scenarios meet MC-007 delivery thresholds.
- [ ] Dense results report reduction relative to the baseline with actual GATT sends and measured power-tier work shares.
- [ ] Own-message overflow gives a visible admission failure; no queue exceeds its packet or byte cap and control traffic cannot starve indefinitely.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If probabilistic thinning adds no benefit under coverage-only suppression, omit it and retain deterministic per-egress suppression.
- Under power pressure shed relay work according to budget and expose degraded operation; do not refresh TTL or claim guaranteed delivery.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Not implemented. Record commands, versions, reproducible inputs and results here. For manual/hardware checks include device/OS, duration and report paths. No test or review is claimed yet.

## Review and merge

- Branch: `ticket/MC-014-relay-scheduling-suppression-and-power-policy`.
- Review/PR: pending.
- Squash commit title: `MC-014: Relay scheduling, suppression and power policy`.
- Completion becomes effective only when the reviewed squash commit lands on main.
