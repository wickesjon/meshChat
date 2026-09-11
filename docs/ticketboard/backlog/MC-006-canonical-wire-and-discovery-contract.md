---
id: "MC-006"
title: "Canonical wire and discovery contract"
depends_on: ["MC-004"]
kind: "decision"
branch: "ticket/MC-006-canonical-wire-and-discovery-contract"
---

# MC-006 — Canonical wire and discovery contract

## Objective

Freeze one outer frame per GATT value with runtime capacity per direction; define every frame/type/flag combination, exact lengths and reserved-field handling.

## Dependencies

`MC-004` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `docs/mesh-chat-design.md`, `docs/decisions/**`, `tests/vectors/README.md`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Freeze one outer frame per GATT value with runtime capacity per direction; define every frame/type/flag combination, exact lengths and reserved-field handling.
- Specify logical and transport-object reassembly, low-capacity refusal, control-packet hop scope, duplicate-link resolution after identity exchange and concrete UUIDs.
- Resolve event QR framing, organizer pin fields/transcripts, cosmetic fields, friend key distribution, DM plaintext/padding framing in coordination with MC-008, and canonical vector conventions.

## Exit criteria

- [ ] Independent readers can determine exact bytes and parser behavior for every v1 packet, QR bundle and flag combination.
- [ ] Every supported packet fits the selected capacity/fragment limits or has a documented explicit refusal case.
- [ ] No pin/cosmetic field is described without a byte position and authentication rule; unresolved crypto choices remain blocked on MC-008 before MC-020.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If optional remote cosmetic hints need a new incompatible layout, keep them local for v1 and record the approved product consequence.
- Do not guess a wire layout in implementation; reopen this decision for a versioned correction before freeze.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Not implemented. Record commands, versions, reproducible inputs and results here. For manual/hardware checks include device/OS, duration and report paths. No test or review is claimed yet.

## Review and merge

- Branch: `ticket/MC-006-canonical-wire-and-discovery-contract`.
- Review/PR: pending.
- Squash commit title: `MC-006: Canonical wire and discovery contract`.
- Completion becomes effective only when the reviewed squash commit lands on main.
