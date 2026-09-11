# MC-005 — Protected-key and encrypted-store probe plan

State: planning and isolated dependency preflight, 2026-09-11. No key-provider implementation, database test, protected-hardware result or final protection-model selection is claimed. The user replaced MC-004's early BLE gate only; MC-005 security evidence remains required.

## Interface and implementation to evaluate

The core must distinguish platform-operated keys from wrapped software keys. A proposed provider uses opaque key handles and explicit capabilities: generate, public key, sign, agree, delete/reset and protection metadata. Errors distinguish unavailable/locked, invalidated/missing, invalid input and provider failure. No operation promises extraction of a non-exportable platform private key. This is a probe-interface proposal; MC-017 owns the production provider.

For platform-operated keys, cryptographic work remains in native APIs and the core receives operation results. For wrapped software keys, encrypted key material is unwrapped into application memory for curve operations; at-rest protection must not be presented as resistance to a compromised running process. No identity/message material is introduced in this spike, and private/shared/database keys must never enter logs, traces or repository fixtures.

Compare Ed25519 signing/verification and X25519 agreement through the shared Rust core and native providers. Use published test vectors for deterministic interoperability and fresh ephemeral secrets for lifecycle tests; report only checks/outcomes. This does not select a DM construction or transcript, which belongs to MC-008.

## Platform evidence inputs

- [Android Keystore](https://developer.android.com/privacy-and-security/keystore) describes non-exportable operation and hardware support conditional on algorithm/parameters. Query actual provider/security-level capabilities; do not infer StrongBox or curve support from Android version alone. Probe API 29 minimum and newer devices, including lock and invalidation behavior.
- [Apple Secure Enclave guidance](https://developer.apple.com/documentation/security/protecting-keys-with-the-secure-enclave) and [SecureEnclave.P256](https://developer.apple.com/documentation/cryptokit/secureenclave/p256) identify P-256 support. P-256 is not a substitute for the specified Ed25519/X25519 curves. Compare platform-operated APIs with a separately protected software-key path; [CryptoKit Keychain guidance](https://developer.apple.com/documentation/cryptokit/storing-cryptokit-keys-in-the-keychain) is a storage input, not proof that Curve25519 runs inside the Secure Enclave. Minimum iOS remains 15.
- [SQLCipher Android upstream](https://github.com/sqlcipher/sqlcipher-android) supplies the native database integration; [Zetetic documentation](https://www.zetetic.net/sqlcipher/documentation/) is the integration reference for both platforms. Pin selected native packages/source before implementation and record their licensing, build and Android 16 KB implications. No commercial purchase or signing/distribution is authorized by this plan.

## Required scenarios

| Scenario | Evidence to retain |
|---|---|
| Curves | Core/native sign/verify and agreement outcomes, provider/OS and protection metadata; no private/shared bytes |
| SQLCipher | Confirm cipher implementation/version, create synthetic row, close/restart/reopen and verify it; wrong key must fail an actual schema/data read |
| Backup | Inspect exclusions and attempt the supported backup/restore path; exclude database, journals/WAL and wrapped secrets as applicable |
| Lock/reboot | Before/after first unlock, screen lock with open and closed DB, process restart; distinguish API refusal from data protection |
| Key loss | Delete/invalidate wrapping key, verify fail-closed behavior and explicit reset; never silently create a replacement key over retained ciphertext |
| Reinstall/reset | Observe platform persistence differences, explicit deletion and restore mismatch; no identity-continuity claim after reset |

Record device/OS/build, configured accessibility/authentication policy, procedure and observed results under `docs/decisions/`. Put harnesses under `tests/bench/security/`. A simulator can help functional testing but cannot attest hardware-backed behavior. Device/Mac/signing availability is unknown; no unavailability is inferred.

## Concrete scope blocker and prepared proposal

The existing ticket permits `src/core/**` and standalone native probe paths, but excludes root `Cargo.lock` and `.github/workflows/ci.yml`. Reproducible shared-core curve dependencies require a root lockfile change. Standalone native security probes also need explicit CI coverage rather than relying on the existing parent-app/BLE builds. Under AGENTS.md, these two paths need a scope decision before application.

Prepared ignored proposals are in `.work/mc005-scope-proposal/`: `core-manifest.patch`, `lockfile.patch` and `ci.patch`. The CI proposal adds standalone Android Debug/Release assembly/lint and iOS Debug/Release simulator/device builds; signing remains a command-line override. These commands target the planned probe projects and cannot run until those projects exist. Neither real root lockfile nor real workflow has been changed for MC-005.

Proposed optional `security-probe` dependencies: [ed25519-dalek 2.2.0](https://docs.rs/ed25519-dalek/2.2.0/ed25519_dalek/), [x25519-dalek 2.0.1](https://docs.rs/x25519-dalek/2.0.1/x25519_dalek/), `zeroize 1.8.1` and the already-locked `getrandom 0.4.3`. OS randomness supplies software key seeds, with failures propagated. The feature is intended for the feasibility harness, not activation of production identities. These versions were selected and checked with the pinned Rust toolchain; adoption remains subject to scope approval and implementation review.

An isolated copy under `.work/` resolved the dependencies and passed `cargo check --locked --features security-probe --lib` with Rust 1.85.1. Existing cargo-deny 0.20.2 policy passed advisories, bans, licenses and sources with `--all-features --locked`; only unused-license-allowance warnings appeared. This checks dependency compatibility and policy, not crypto correctness, native packaging or storage behavior. Actual curve vectors, native builds and lifecycle scenarios remain unimplemented.
