---
id: "MC-005"
title: "Platform key and encrypted-store feasibility"
depends_on: ["MC-003"]
kind: "spike"
branch: "ticket/MC-005-platform-key-and-encrypted-store-feasibility"
---

# MC-005 — Platform key and encrypted-store feasibility

## Objective

Probe Ed25519/X25519 use from the shared core and each platform's key APIs on minimum supported OS versions.

## Dependencies

`MC-003` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/core/**`, `src/android/security/**`, `src/ios/Security/**`, `tests/bench/security/**`, `docs/decisions/**`, `Cargo.lock`, `.github/workflows/ci.yml`.

Scope approval: the user explicitly approved adding `Cargo.lock` and `.github/workflows/ci.yml` on 2026-09-11 for the prepared Rust crypto dependencies and standalone security-probe checks. No physical or security acceptance criterion is waived.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

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

Started from main `9e3c75a17b759391fa53217d6002ade6336ff55c`, where MC-003 and MC-004 are complete. The [probe plan](../../decisions/MC-005-probe-plan.md) records implemented candidate protection models, dependency/build implications, automated evidence and required device scenarios. The [bench runbook](../../../tests/bench/security/README.md) provides concrete build and acceptance procedures.

Implemented: optional Rust Ed25519/X25519 probe exports and RFC vectors; isolated generated bindings; standalone Android/iOS wrapped-key and SQLCipher fixtures; native/core interoperability; explicit key deletion/reset and lock-read controls; backup exclusion configuration; sanitized bounded reports. These are feasibility tools, not production identity/storage providers.

Automated evidence: Rust 1.85.1 all-feature Clippy and Debug/Release tests pass (four foundation and three probe tests); cargo-deny policy passes; probe bindings generate. CI runs `34634940919` (`0741b26`) and `34636287902` (`12c30af`) passed native builds/lint, instrumentation compilation, rebuilt SQLCipher/Rust/JNA APK alignment, all four unsigned iOS security builds, Swift/CryptoKit interoperability and Rust/ticketboard gates. The original AAR alignment failure was resolved by the pinned native rebuild.

The user requested emulator testing. Three process-separated Android API 29 phases implement create/reopen/wrong-key, key-loss refusal/ciphertext retention and explicit reset. Run `34638458504` (`331dec9`) passed all native checks and booted the software emulator, but package service was unavailable during APK installation after compilation; no lifecycle phase ran. Startup now checks package/activity readiness, stops the VM during builds and restarts it immediately before testing. [CI run 34641115491](https://github.com/wickesjon/meshChat/actions/runs/34641115491) passed all four jobs at source/workflow revision `62f81e68e29e4218362f5176e58eea1ffb707d32`. Android API 29 (4096-byte pages) passed create, reopen and key-loss phases with explicit instrumentation success codes. This establishes the synthetic emulator lifecycle, wrong-key refusal/correct-key recovery and explicit reset, with key-loss ciphertext retention. The final evidence-only update leaves tested sources unchanged. No physical gate was waived.

Device/Mac/signing inventory remains unknown. Hardware capability, restart/wrong-key behavior on devices, actual backup/restore exclusion, lock/reboot, OS invalidation and uninstall/restore results are still required. No final protection model is selected yet. The ticket is inreview for code review with all exit criteria unchecked and is **not merge-ready**; MC-004's replaced gate does not waive MC-005 security evidence.

## Review and merge

- Branch: `ticket/MC-005-platform-key-and-encrypted-store-feasibility`.
- Review/PR: [PR #7](https://github.com/wickesjon/meshChat/pull/7). Separate Terra medium review of `85047becbf428dcd4f860d6e89f12ea9019a841a` posted [review #5182256064](https://github.com/wickesjon/meshChat/pull/7#pullrequestreview-5182256064), no actionable code defects; merge withheld for missing physical evidence/final model. Completion was awaited without polling. Follow-up [review #5182328957](https://github.com/wickesjon/meshChat/pull/7#pullrequestreview-5182328957) flagged possible instrumentation APK inclusion in native checks. The actual `0741b26` CI check passed (the test APK is deeper than the original glob), and selectors are now explicit Debug/Release paths to remove ambiguity. Subsequent Terra reviews found no blocking code issue, including [review #5182864367](https://github.com/wickesjon/meshChat/pull/7#pullrequestreview-5182864367) of final source/workflow revision `62f81e68e29e4218362f5176e58eea1ffb707d32`. Review completion was awaited without polling; physical evidence and final model selection remain separate merge blockers.
- Squash commit title: `MC-005: Platform key and encrypted-store feasibility`.
- Completion becomes effective only when the reviewed squash commit lands on main.
