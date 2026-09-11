---
id: "MC-008"
title: "Reviewed DM and trust contract"
depends_on: ["MC-005"]
kind: "decision"
branch: "ticket/MC-008-reviewed-dm-and-trust-contract"
---

# MC-008 — Reviewed DM and trust contract

## Objective

Evaluate a vetted RFC 9180 authenticated-mode implementation against the current custom analogue; record suite, dependency review and interoperability implications.

## Dependencies

`MC-005` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `docs/mesh-chat-design.md`, `docs/decisions/**`, `tests/vectors/crypto/README.md`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Evaluate a vetted RFC 9180 authenticated-mode implementation against the current custom analogue; record suite, dependency review and interoperability implications.
- Define authentication of immutable header fields, mutable TTL exclusion, replay policy, sender/pin binding, key-compromise limitations and domain-separated encodings.
- Define signed-presence freshness and key-change behavior; distinguish a verified historical signature from evidence of a live direct peer.

## Exit criteria

- [ ] An explicit crypto decision selects the construction and specifies exact inputs, encodings, rejection rules and evidence required for MC-022.
- [ ] Friend identity, X25519 binding, stale presence, unknown keys and key reset have unambiguous state transitions.
- [ ] The decision includes independent review requirements and reference-vector sources.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If no suitable vetted HPKE dependency meets platform constraints, keep DMs blocked pending review of the alternative construction.
- If live presence authentication cannot be implemented within scope, display authenticated last-seen evidence rather than asserting current proximity.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Not implemented. Record commands, versions, reproducible inputs and results here. For manual/hardware checks include device/OS, duration and report paths. No test or review is claimed yet.

## Review and merge

- Branch: `ticket/MC-008-reviewed-dm-and-trust-contract`.
- Review/PR: pending.
- Squash commit title: `MC-008: Reviewed DM and trust contract`.
- Completion becomes effective only when the reviewed squash commit lands on main.
