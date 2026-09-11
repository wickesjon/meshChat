---
id: "MC-009"
title: "Logical packet codec and golden vectors"
depends_on: ["MC-003","MC-006"]
kind: "core"
branch: "ticket/MC-009-logical-packet-codec-and-golden-vectors"
---

# MC-009 — Logical packet codec and golden vectors

## Objective

Implement bounded header/type parsing and serialization for CHAT, ANNOUNCE, SYNC_REQ, REACTION, EVENT_INFO, CRED_REQ/OFFER and opaque unknown types.

## Dependencies

`MC-003`, `MC-006` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/core/**`, `tests/vectors/base/**`, `tests/integration/codec/**`, `tests/fuzz/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Implement bounded header/type parsing and serialization for CHAT, ANNOUNCE, SYNC_REQ, REACTION, EVENT_INFO, CRED_REQ/OFFER and opaque unknown types.
- Enforce the resolved length, UTF-8, flag and control-packet rules using checked arithmetic; preserve original wire bytes for authentication.
- Add committed byte-exact vectors and parser fuzz targets from the canonical contract.

## Exit criteria

- [ ] All supported types round-trip and match golden vectors; malformed lengths and flag combinations return errors.
- [ ] Unknown-type forwarding preserves opaque bytes within conservative budgets; unknown flags never bypass known-type validation.
- [ ] Fuzz smoke runs complete without panic or unbounded allocation and persist useful regression inputs.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If a vector disagrees with the design, stop and resolve the contract; do not make the parser accept both ambiguous layouts.
- Keep unimplemented future types opaque and undisplayed within the defined compatibility policy.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Not implemented. Record commands, versions, reproducible inputs and results here. For manual/hardware checks include device/OS, duration and report paths. No test or review is claimed yet.

## Review and merge

- Branch: `ticket/MC-009-logical-packet-codec-and-golden-vectors`.
- Review/PR: pending.
- Squash commit title: `MC-009: Logical packet codec and golden vectors`.
- Completion becomes effective only when the reviewed squash commit lands on main.
