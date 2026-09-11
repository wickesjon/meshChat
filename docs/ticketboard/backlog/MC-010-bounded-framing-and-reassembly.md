---
id: "MC-010"
title: "Bounded framing and reassembly"
depends_on: ["MC-009","MC-007"]
kind: "core"
branch: "ticket/MC-010-bounded-framing-and-reassembly"
---

# MC-010 — Bounded framing and reassembly

## Objective

Implement all four frame kinds, per-direction slicing, logical envelopes and transport-object envelopes.

## Dependencies

`MC-009`, `MC-007` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/core/**`, `tests/integration/transport/**`, `tests/fuzz/**`, `tests/vectors/base/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Implement all four frame kinds, per-direction slicing, logical envelopes and transport-object envelopes.
- Enforce group/count/byte/time limits before allocation; scope groups to connection generation and validate IDs and exact reconstructed lengths.
- Define conflicting duplicate fragments, timeout/eviction, disconnect cleanup and malformed-frame accounting.

## Exit criteria

- [ ] Whole and fragmented vectors pass at minimum and larger supported capacities without oversized GATT values.
- [ ] Cross-link/group collisions, out-of-order fragments, conflicting duplicates and reconnects cannot mix packets.
- [ ] Exhaustion tests demonstrate bounded peak buffers and successful cleanup after expiry/disconnect.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If a link cannot carry a legal maximum-size packet within fragment bounds, return an explicit unsupported-capacity result.
- On resource exhaustion drop/evict according to the contract; never grow limits or accept truncated packets.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Not implemented. Record commands, versions, reproducible inputs and results here. For manual/hardware checks include device/OS, duration and report paths. No test or review is claimed yet.

## Review and merge

- Branch: `ticket/MC-010-bounded-framing-and-reassembly`.
- Review/PR: pending.
- Squash commit title: `MC-010: Bounded framing and reassembly`.
- Completion becomes effective only when the reviewed squash commit lands on main.
