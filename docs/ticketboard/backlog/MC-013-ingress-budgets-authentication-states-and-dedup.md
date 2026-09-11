---
id: "MC-013"
title: "Ingress budgets, authentication states and dedup"
depends_on: ["MC-010","MC-007","MC-012"]
kind: "core"
branch: "ticket/MC-013-ingress-budgets-authentication-states-and-dedup"
---

# MC-013 — Ingress budgets, authentication states and dedup

## Objective

Charge bytes/frames before allocation and apply global/per-link expensive-work limits at each actual operation.

## Dependencies

`MC-010`, `MC-007`, `MC-012` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/core/**`, `tests/integration/ingress/**`, `tests/simulator/**`, `tests/fuzz/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Charge bytes/frames before allocation and apply global/per-link expensive-work limits at each actual operation.
- Separate bounded attempt tracking from accepted-message dedup so invalid copies cannot suppress later authenticated packets; preserve bounded rejection of repeated garbage.
- Implement per-sender/class and unknown-type limits, control-packet admission and reconnect-resistant abuse accounting within documented limits.

## Exit criteria

- [ ] Invalid-first/valid-second and identical-replay tests pass for signed and encrypted test fixtures.
- [ ] Flooding rotated identities cannot exceed the sum of the attacker's actual admitted link budgets; multi-link attacks are measured separately.
- [ ] Over-budget frames never enter display, reassembly or crypto paths; memory and work remain within specified bounds.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If authentication is pending, use the explicit unverified/pending state without adding a trusted dedup entry.
- If budget pressure prevents verification, drop or defer within a bounded queue; never label unverified traffic authenticated.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Not implemented. Record commands, versions, reproducible inputs and results here. For manual/hardware checks include device/OS, duration and report paths. No test or review is claimed yet.

## Review and merge

- Branch: `ticket/MC-013-ingress-budgets-authentication-states-and-dedup`.
- Review/PR: pending.
- Squash commit title: `MC-013: Ingress budgets, authentication states and dedup`.
- Completion becomes effective only when the reviewed squash commit lands on main.
