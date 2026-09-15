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

- [ ] Both platform adapters implement the MC-005 development contract; synthetic automated provisioning/reopen/error tests pass, and iOS device/simulator builds pass. Production iOS has no software wrapping fallback. Physical certification is deferred to MC-043/044; test doubles are confined to tests.
- [ ] Reset rotates both keypairs and invalidates associated pins/state atomically or with recoverable journaled behavior.
- [ ] Key-unavailable and invalidated states block authenticated sends and have tested recovery paths.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If protected keys become unavailable, show recovery/reset choices rather than generating a silent replacement identity.
- Use only the wrapped-key fallback approved in MC-005; never write unprotected private keys.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Scheduling decision: the user deferred MC-005 physical verification on 2026-09-11. This implementation ticket uses synthetic data and automated tests; MC-043/044 retest the production provider/storage on actual devices. Its completion does not certify hardware, backup or lock behavior.

Implemented candidate, based on merged MC-016 `4730f017b4996fa7793d27582346f01f1d4d7899`. Review and required hosted native checks are pending; this ticket is not complete.

- Rust `IdentityKeySession` performs the pinned Ed25519/X25519 operations, derives the public identifiers, rejects noncanonical/noncontributory peers and clears its owned seeds on invalidation/drop. The native provider imports a temporary 64-byte unwrapped seed pair through UniFFI and destroys the session after each operation. It exposes no seed-export method; application operations take opaque generation handles. Managed-runtime/FFI copies prevent any claim of complete memory erasure or hardware curve execution. MC-019/020 own protocol transcripts and provider-internal HPKE integration; the foundation callback is not yet a shipping application integration.
- Android persists a bounded authenticated envelope in `noBackupFilesDir`, using AndroidKeyStore AES-256-GCM with `setUnlockedDeviceRequired(true)` and OS SecureRandom. Capability metadata reports actual KeyInfo hardware/software wrapping. A process-wide operation lock serializes provider instances; the application must keep identity operations in its single process. Cross-process use is unsupported.
- iOS persists a bounded envelope with complete file protection and verified backup exclusion. It requires Secure Enclave P-256 ECIES and `WhenUnlockedThisDeviceOnly`, checks the loaded key's token, and uses SecRandomCopyBytes. The authenticated plaintext includes the version/generation header because the ECIES API has no AAD parameter. There is no production simulator/software wrapping fallback. MainActor serialization covers provider instances.
- Creation refuses retained artifacts. Reopen never creates material. Missing, locked, invalidated, malformed, stale-handle and interrupted-operation states refuse key use. Explicit reset durably writes a recovery marker before invoking the mandatory `IdentityResetStore.clearIdentityState()` callback and rotating both keys. MC-018 must implement that durable/idempotent invalidation of all prior pins, trust, history and store state; there is no production no-op default. Callback or later failure retains a recovery barrier; retrying reset clears state again. File writes sync data and directory entries before the marker is removed.
- Test doubles live only in `tests/integration/identity/`. JVM tests compile both actual Android sources and generated Kotlin bindings and call the real Rust DLL, with injected test storage/AES wrapping. They cover stable reopen, sign/agree, malformed/corrupt envelope refusal, lock/unavailable/key loss, stale handles, callback failure, entropy failure, four interrupted creation mutations and seven interrupted reset mutations. Swift has the equivalent suite with CryptoKit signature verification; the existing iOS security binding build invokes it on the Mac host. Neither injected suite proves native key-store or physical-device behavior.

Local Windows evidence (2026-09-15; final source revision to be recorded in PR): Rust 1.85.1/cargo 1.85.1, Python 3.14.4, cargo-deny 0.20.2, JDK 17.0.15+6, Kotlin 2.2.0, JNA 5.17.0 and Android API 36 compile stubs. Commands/results:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo test --workspace --all-features --locked --release
cargo build --workspace --all-features --locked --release
cargo-deny --all-features --locked --config src/core/deny.toml check
git diff --exit-code -- Cargo.lock
python -B src/core/build_bindings.py host --security-probe
python -B tests/integration/identity/run_kotlin.py
python -B tests/ticketboard/validate.py --write
python -B tests/ticketboard/validate.py
python -B -m unittest discover -s tests/ticketboard -v
git diff --check
```

All listed checks passed: 84 Rust checks per debug/release profile (82 runtime plus two compile-fail), Kotlin lifecycle suite, 12 board tests and 46-ticket/127-dependency validation. Advisory refresh initially could not reach GitHub in the sandbox; the authorized network retry passed advisories/bans/licenses/sources, with two existing unused-license allowances. Local logs are `.work/mc017/rust-debug.log` and `rust-release.log`. Native Android packaging/lint/ABI/alignment/security-emulator checks and pinned Xcode 16.4 device/simulator builds plus Swift regressions remain required before merge because this changes shared FFI and both consumers. No failed/unavailable required native check is treated as passing.

## Review and merge

- Branch: `ticket/MC-017-identity-provider-and-key-lifecycle`.
- Review/PR: pending.
- Squash commit title: `MC-017: Identity provider and key lifecycle`.
- Completion becomes effective only when the reviewed squash commit lands on main.
