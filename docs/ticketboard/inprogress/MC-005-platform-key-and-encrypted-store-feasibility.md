---
id: "MC-005"
title: "Platform key and encrypted-store feasibility"
depends_on: ["MC-003"]
kind: "spike"
branch: "ticket/MC-005-platform-key-and-encrypted-store-feasibility"
---

# MC-005 â€” Platform key and encrypted-store feasibility

## Objective

Probe Ed25519/X25519 use from the shared core and each platform's key APIs on minimum supported OS versions.

## Dependencies

`MC-003` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/core/**`, `src/android/security/**`, `src/ios/Security/**`, `tests/bench/security/**`, `docs/decisions/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its Â§0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Probe Ed25519/X25519 use from the shared core and each platform's key APIs on minimum supported OS versions.
- Compare platform-operated non-exportable keys with encrypted software key material protected by a platform wrapping key; document the threat model honestly.
- Prove SQLCipher open/reopen, backup exclusion and locked-device behavior on both platforms; record dependency/build implications.

## Exit criteria

- [ ] An explicit key-provider interface and supported protection model are selected for both platforms.
- [ ] A protected test database survives restart, fails to open with the wrong key, and has a documented backup-exclusion test.
- [ ] Record reset, key invalidation, uninstall/restore and device-lock behavior without writing secrets to logs.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If a platform cannot perform the selected curves in its protected hardware, use reviewed wrapped software keys with the weaker protection clearly recorded.
- If secure persistence cannot be demonstrated, block persistent identities/DM release instead of falling back to plaintext storage.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Started from main `9e3c75a17b759391fa53217d6002ade6336ff55c`, where MC-003 and MC-004 are complete. The [probe plan](../../decisions/MC-005-probe-plan.md) records the provider comparison, native/storage scenarios and threat-model limits. No production probe or physical evidence is claimed.

Blocker requiring scope approval: add root `Cargo.lock` for reproducible Rust curve dependencies and `.github/workflows/ci.yml` for standalone security-probe checks. Concrete patches are prepared under ignored `.work/mc005-scope-proposal/`; the actual root files are unchanged. Isolated dependency resolution and Rust 1.85.1 checking passed, as did existing cargo-deny advisory/license/version/source gates. This is dependency preflight only, not successful key/storage acceptance.

Device/Mac/signing inventory remains unknown. MC-005 hardware/security evidence requirements have not been replaced by the MC-004 approval. Ticket remains inprogress with all exit criteria unchecked.

## Review and merge

- Branch: `ticket/MC-005-platform-key-and-encrypted-store-feasibility`.
- Review/PR: pending.
- Squash commit title: `MC-005: Platform key and encrypted-store feasibility`.
- Completion becomes effective only when the reviewed squash commit lands on main.
