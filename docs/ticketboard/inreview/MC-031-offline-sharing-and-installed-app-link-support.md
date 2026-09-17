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

User-approved scope extension (2026-09-17): `src/core/src/native_channels.rs` for a small exported adapter to the existing canonical channel-link parser, and `tests/integration/android-ui/**` for its regression coverage. No wire-format or parsing-rule change is authorized.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Implement channel/friend QR export and scan with white backgrounds/quiet zones, native confirmation and sensitive clipboard handling.
- Build the minimal HTTPS fallback page and association-file templates for Android App Links/iOS Universal Links; keep domain/store IDs configurable until owned values exist.
- Document offline custom-scheme scanning versus online installation fallback and test both installed/uninstalled flows.

## Exit criteria

- [x] Offline in-app scanning resolves the exact intended channel/key proposal without network access.
- [x] Association files and fallback page pass local fixture checks; production verification is recorded once domain/store IDs are available.
- [x] Share copy does not promise universal HTTPS handling before association has been established.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If domain or store IDs are unavailable, deliver templates and mark production link activation blocked; QR remains the supported offline path.
- Do not buy a domain, deploy a site or publish association files without explicit authorization.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Implemented from main `81170ad54b481f483f3227f138147e2a4d2ef2d6`; MC-011/028 and reused MC-029 are complete there. See [implementation/validation evidence](../../sharing/MC-031-validation.md) and [sharing/activation guide](../../sharing/README.md). Local core, template, Android build/lint/JVM and emulator checks pass; final cache-regression rerun, iOS checks and required review are pending. The second criterion uses the ticket's explicit missing-identifiers fallback; it does not claim production association.

The domain/store-ID fallback is triggered: no owned domain, production signing certificate, Apple app identifier or store listing has been supplied. Deliver local templates; production association, installation and activation remain blocked pending those values and separate deployment authorization. The frozen parser accepts `meshfest.app` only; template configuration cannot silently change that contract. iOS feature UI parity remains MC-035; this ticket supplies its association templates and integration guide.

## Review and merge

- Branch: `ticket/MC-031-offline-sharing-and-installed-app-link-support`.
- Review/PR: pending.
- Squash commit title: `MC-031: Offline sharing and installed-app link support`.
- Completion becomes effective only when the reviewed squash commit lands on main.
