---
id: "MC-021"
title: "Organizer trust, credentials and signed updates"
depends_on: ["MC-019","MC-006","MC-016"]
kind: "security"
branch: "ticket/MC-021-organizer-trust-credentials-and-signed-updates"
---

# MC-021 — Organizer trust, credentials and signed updates

## Objective

Implement canonical root/staff bundles, root adoption expiry, credential binding and authenticated pin metadata.

## Dependencies

`MC-019`, `MC-006`, `MC-016` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/core/**`, `tests/vectors/crypto/**`, `tests/integration/organizer/**`, `tests/simulator/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Implement canonical root/staff bundles, root adoption expiry, credential binding and authenticated pin metadata.
- Implement bounded opaque credential caching and CRED_OFFER/REQ recovery across non-adopting relays, with quotas and expiry.
- Separate display trust from structural relay allowance and verify staff/root/time bindings before assigning authority.

## Exit criteria

- [ ] Root-to-credential-to-message vectors pass including multiple staff keys, altered pin fields and mismatched root/key IDs.
- [ ] A multi-hop late joiner resolves credentials when its immediate peer initially lacks them; stale or unavailable data stays pending/unverified.
- [ ] Unadopted roots and forged chains never grant badges; credential floods stay within memory and work limits.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If credentials cannot be recovered within bounded retries, keep the update unverified and retry only under the documented policy.
- If offline expiry makes verification uncertain, expose the clock/trust issue; never extend signed validity silently.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Not implemented. Record commands, versions, reproducible inputs and results here. For manual/hardware checks include device/OS, duration and report paths. No test or review is claimed yet.

## Review and merge

- Branch: `ticket/MC-021-organizer-trust-credentials-and-signed-updates`.
- Review/PR: pending.
- Squash commit title: `MC-021: Organizer trust, credentials and signed updates`.
- Completion becomes effective only when the reviewed squash commit lands on main.
