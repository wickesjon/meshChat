---
id: "MC-031"
title: "Offline sharing and installed-app link support"
depends_on: ["MC-011","MC-028"]
kind: "product"
branch: "ticket/MC-031-offline-sharing-and-installed-app-link-support"
---

# MC-031 — Offline sharing and installed-app link support

## Objective

Implement channel/friend QR export and scan with white backgrounds/quiet zones, native confirmation and sensitive clipboard handling.

## Dependencies

`MC-011`, `MC-028` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/android/**`, `src/ios/**`, `src/share-site/**`, `tests/integration/sharing/**`, `docs/sharing/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Implement channel/friend QR export and scan with white backgrounds/quiet zones, native confirmation and sensitive clipboard handling.
- Build the minimal HTTPS fallback page and association-file templates for Android App Links/iOS Universal Links; keep domain/store IDs configurable until owned values exist.
- Document offline custom-scheme scanning versus online installation fallback and test both installed/uninstalled flows.

## Exit criteria

- [ ] Offline in-app scanning resolves the exact intended channel/key proposal without network access.
- [ ] Association files and fallback page pass local fixture checks; production verification is recorded once domain/store IDs are available.
- [ ] Share copy does not promise universal HTTPS handling before association has been established.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If domain or store IDs are unavailable, deliver templates and mark production link activation blocked; QR remains the supported offline path.
- Do not buy a domain, deploy a site or publish association files without explicit authorization.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Not implemented. Record commands, versions, reproducible inputs and results here. For manual/hardware checks include device/OS, duration and report paths. No test or review is claimed yet.

## Review and merge

- Branch: `ticket/MC-031-offline-sharing-and-installed-app-link-support`.
- Review/PR: pending.
- Squash commit title: `MC-031: Offline sharing and installed-app link support`.
- Completion becomes effective only when the reviewed squash commit lands on main.
