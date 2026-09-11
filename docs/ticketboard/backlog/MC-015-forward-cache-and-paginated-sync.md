---
id: "MC-015"
title: "Forward cache and paginated SYNC"
depends_on: ["MC-014","MC-007"]
kind: "core"
branch: "ticket/MC-015-forward-cache-and-paginated-sync"
---

# MC-015 — Forward cache and paginated SYNC

## Objective

Implement bounded live caches with explicit storable types, local retention and stable walk cursors.

## Dependencies

`MC-014`, `MC-007` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/core/**`, `tests/simulator/**`, `tests/integration/sync/**`, `tests/vectors/base/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Implement bounded live caches with explicit storable types, local retention and stable walk cursors.
- Implement session admission/continuation, empty completion, cancellation and exact embedded-packet ingress accounting using the MC-007 budgets.
- Preserve stored TTL and message identity; define cache expiry during pagination, fixed-Bloom omissions and byte-bounded recent-context selection.

## Exit criteria

- [ ] Late-join tests meet the approved eligible-workload gate at each supported link capacity.
- [ ] Empty caches, full-byte-budget pages, concurrent mutation, disconnects and stale cursors terminate predictably.
- [ ] SYNC cannot reset TTL, bypass crypto/ingress caps or turn expired cache entries into indefinite replay loops.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If the eligible data exceeds budget, return explicit truncation/continuation metadata and deliver bounded recent context.
- If a Bloom positive hides an item, report probabilistic best effort; retries with the unchanged filter must not be claimed to repair it.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Not implemented. Record commands, versions, reproducible inputs and results here. For manual/hardware checks include device/OS, duration and report paths. No test or review is claimed yet.

## Review and merge

- Branch: `ticket/MC-015-forward-cache-and-paginated-sync`.
- Review/PR: pending.
- Squash commit title: `MC-015: Forward cache and paginated SYNC`.
- Completion becomes effective only when the reviewed squash commit lands on main.
