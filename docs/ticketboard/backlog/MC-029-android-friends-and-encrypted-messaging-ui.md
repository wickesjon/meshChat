---
id: "MC-029"
title: "Android friends and encrypted messaging UI"
depends_on: ["MC-028","MC-020"]
kind: "android"
branch: "ticket/MC-029-android-friends-and-encrypted-messaging-ui"
---

# MC-029 — Android friends and encrypted messaging UI

## Objective

Build friend QR/scanner, explicit fingerprint/petname confirmation, friend management and approved nearby/last-seen behavior.

## Dependencies

`MC-028`, `MC-020` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/android/ui/**`, `tests/integration/android-ui/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Build friend QR/scanner, explicit fingerprint/petname confirmation, friend management and approved nearby/last-seen behavior.
- Build DM list/threads and encrypted reactions using authenticated core events and stored conversation identity.
- Implement changed-key blocking, missing-key states and clear confidentiality/metadata information.

## Exit criteria

- [ ] Adding friends, encrypted send/receive, reactions, restart and friend removal work end to end.
- [ ] Deep-link and scanned-key flows require explicit trust confirmation; spoofed nicknames cannot gain a verified badge.
- [ ] Key changes block sending until re-pairing; no UI action causes plaintext DM fallback.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If camera access is denied, offer the validated explicit link/code flow with its source-trust warning.
- If live presence proof is unavailable, show last authenticated observation using MC-008 semantics.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Not implemented. Record commands, versions, reproducible inputs and results here. For manual/hardware checks include device/OS, duration and report paths. No test or review is claimed yet.

## Review and merge

- Branch: `ticket/MC-029-android-friends-and-encrypted-messaging-ui`.
- Review/PR: pending.
- Squash commit title: `MC-029: Android friends and encrypted messaging UI`.
- Completion becomes effective only when the reviewed squash commit lands on main.
