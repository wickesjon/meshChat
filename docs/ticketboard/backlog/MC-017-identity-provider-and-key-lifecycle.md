---
id: "MC-017"
title: "Identity provider and key lifecycle"
depends_on: ["MC-005","MC-008","MC-003"]
kind: "security"
branch: "ticket/MC-017-identity-provider-and-key-lifecycle"
---

# MC-017 — Identity provider and key lifecycle

## Objective

Implement the selected protected-key provider for Ed25519/X25519 and secure randomness.

## Dependencies

`MC-005`, `MC-008`, `MC-003` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/core/**`, `src/android/security/**`, `src/ios/Security/**`, `tests/integration/identity/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Implement the selected protected-key provider for Ed25519/X25519 and secure randomness.
- Implement first-run creation, stable identity derivation, explicit reset, key invalidation and friend-pin clearing.
- Keep ephemeral message material out of persistent storage and avoid key-bearing diagnostics.

## Exit criteria

- [ ] Both platforms provision and reopen identities through the selected protection model.
- [ ] Reset rotates both keypairs and invalidates associated pins/state atomically or with recoverable journaled behavior.
- [ ] Key-unavailable and invalidated states block authenticated sends and have tested recovery paths.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If protected keys become unavailable, show recovery/reset choices rather than generating a silent replacement identity.
- Use only the wrapped-key fallback approved in MC-005; never write unprotected private keys.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Not implemented. Record commands, versions, reproducible inputs and results here. For manual/hardware checks include device/OS, duration and report paths. No test or review is claimed yet.

## Review and merge

- Branch: `ticket/MC-017-identity-provider-and-key-lifecycle`.
- Review/PR: pending.
- Squash commit title: `MC-017: Identity provider and key lifecycle`.
- Completion becomes effective only when the reviewed squash commit lands on main.
