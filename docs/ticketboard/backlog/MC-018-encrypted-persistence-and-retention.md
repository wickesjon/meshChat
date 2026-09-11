---
id: "MC-018"
title: "Encrypted persistence and retention"
depends_on: ["MC-017","MC-011"]
kind: "security"
branch: "ticket/MC-018-encrypted-persistence-and-retention"
---

# MC-018 — Encrypted persistence and retention

## Objective

Define and implement migrations for messages, subscriptions, friend pins, DM conversations, trust provenance, event roots/credentials, settings and reaction state where persistent.

## Dependencies

`MC-017`, `MC-011` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/core/**`, `src/android/security/**`, `src/ios/Security/**`, `tests/integration/storage/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Define and implement migrations for messages, subscriptions, friend pins, DM conversations, trust provenance, event roots/credentials, settings and reaction state where persistent.
- Use SQLCipher through the selected key provider, parameterized SQL and explicit platform backup exclusions. Under the user-approved MC-005 deferral, physical restore/exclusion verification belongs to MC-043/044.
- Implement continuous/launch pruning, deletion and identity-reset coordination; distinguish local history from bounded forward caches.

## Exit criteria

- [ ] Wrong-key reads fail; plaintext message/key markers are absent from the database and auxiliary files in controlled tests.
- [ ] Restart preserves appropriate history and trust state; rotating DM tags do not split a conversation.
- [ ] Synthetic migration rollback/failure, retention, deletion and backup-configuration tests pass for both platform integrations. Physical backup/transfer/restore tests remain mandatory at MC-043/044 before real sensitive-data use.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If migration fails, preserve the encrypted original and present a recoverable error; do not recreate silently.
- If encrypted storage is unavailable, disable persistence-dependent features instead of writing plaintext.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Scheduling decision: the user deferred MC-005 physical verification on 2026-09-11. This implementation ticket uses synthetic data and automated tests; MC-043/044 retest the production provider/storage on actual devices. Its completion does not certify hardware, backup or lock behavior.

Not implemented. Record commands, versions, reproducible inputs and results here. For manual/hardware checks include device/OS, duration and report paths. No test or review is claimed yet.

## Review and merge

- Branch: `ticket/MC-018-encrypted-persistence-and-retention`.
- Review/PR: pending.
- Squash commit title: `MC-018: Encrypted persistence and retention`.
- Completion becomes effective only when the reviewed squash commit lands on main.
