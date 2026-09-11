---
id: "MC-019"
title: "Verified friends and fresh presence"
depends_on: ["MC-017","MC-018","MC-011","MC-008"]
kind: "security"
branch: "ticket/MC-019-verified-friends-and-fresh-presence"
---

# MC-019 — Verified friends and fresh presence

## Objective

Implement two-key QR pinning, petnames, signed CHAT/ANNOUNCE verification and bounded public-key distribution/cache behavior.

## Dependencies

`MC-017`, `MC-018`, `MC-011`, `MC-008` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/core/**`, `tests/integration/friends/**`, `tests/vectors/crypto/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Implement two-key QR pinning, petnames, signed CHAT/ANNOUNCE verification and bounded public-key distribution/cache behavior.
- Implement the approved freshness mechanism or last-seen semantics; bind presence to the actual authenticated link/session as specified.
- Handle unsigned impersonation, unknown signatures, changed keys, removal and explicit re-pairing without nickname-based auto-trust.

## Exit criteria

- [ ] Pinned valid signatures show verified identity; copied IDs/nicknames without valid proof never do.
- [ ] Replayed ANNOUNCE cannot create unsupported current-nearby claims.
- [ ] Key-change and removal tests block stale-key sends and require explicit pin replacement.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If the signer key is unavailable, mark the message unverified/pending and use bounded recovery.
- If identity continuity is uncertain, preserve the old pin and ask for an explicit re-scan rather than matching by nickname.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Not implemented. Record commands, versions, reproducible inputs and results here. For manual/hardware checks include device/OS, duration and report paths. No test or review is claimed yet.

## Review and merge

- Branch: `ticket/MC-019-verified-friends-and-fresh-presence`.
- Review/PR: pending.
- Squash commit title: `MC-019: Verified friends and fresh presence`.
- Completion becomes effective only when the reviewed squash commit lands on main.
