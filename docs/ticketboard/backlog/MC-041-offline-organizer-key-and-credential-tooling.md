---
id: "MC-041"
title: "Offline organizer key and credential tooling"
depends_on: ["MC-021"]
kind: "product"
branch: "ticket/MC-041-offline-organizer-key-and-credential-tooling"
---

# MC-041 — Offline organizer key and credential tooling

## Objective

Build offline root/key generation and staff credential issuance using the canonical core formats; require explicit input for validity and labels.

## Dependencies

`MC-021` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/organizer-tools/**`, `tests/integration/organizer-tools/**`, `docs/organizer/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Build offline root/key generation and staff credential issuance using the canonical core formats; require explicit input for validity and labels.
- Produce public adoption QR data and controlled one-time staff provisioning display without committing private keys or QR images.
- Implement validation/dry-run output and documented rotation/expiry procedures; keep signing keys off relay beacons.

## Exit criteria

- [ ] Tool-generated public/staff bundles round-trip through core verification and mobile import fixtures.
- [ ] Expired, mismatched and malformed credentials are rejected; private material is absent from logs and tracked outputs.
- [ ] An offline rehearsal provisions two staff identities and verifies their updates through a non-trusting relay.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If safe QR display/export is unavailable, provide a controlled local workflow and block staff provisioning UI integration until verified.
- Do not use a shared permanent root private key on staff phones or beacons as a shortcut.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Not implemented. Record commands, versions, reproducible inputs and results here. For manual/hardware checks include device/OS, duration and report paths. No test or review is claimed yet.

## Review and merge

- Branch: `ticket/MC-041-offline-organizer-key-and-credential-tooling`.
- Review/PR: pending.
- Squash commit title: `MC-041: Offline organizer key and credential tooling`.
- Completion becomes effective only when the reviewed squash commit lands on main.
