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

### Scope extension approved — 2026-09-17

Dependency MC-021 is complete on main. This branch starts from `a3ccac421f0ec3bfb0fbafabbc454bd09be8e7b2`, after reviewed MC-033 was squash merged. Implementation started after the scope approval below.

The offline tool will be a separate Rust workspace package using the existing pinned crypto dependencies and canonical core parsers/verification. Root and staff generation require explicit expiry/validity and labels; production randomness comes from the operating system. Public adoption data may be exported; private provisioning is an explicit local one-time operation, with no secret in command arguments, logs, tracked artifacts or relay state. Tests will generate synthetic inputs only, reject invalid/expired/mismatched chains, and rehearse two staff identities through a relay that does not adopt the root. Full mobile UI/protected staff lifecycle remains MC-030/035; this ticket still requires native import fixtures.

Approved narrow extension: `Cargo.toml` and `Cargo.lock` for workspace registration and the new package lock entry; `.github/workflows/ci.yml` and `src/core/build_bindings.py` for selecting the existing mobile core explicitly and registering the offline/native fixture checks; `src/core/src/security_probe.rs` for a test-only adapter to the canonical organizer import path if needed by Kotlin/Swift fixtures; and `src/android/security/build.gradle.kts` for registering those fixture sources under this ticket's existing test directory. The user approved this extension with “yes” on 2026-09-17 before implementation. No new production organizer API, wire format, trust rule, independent-security waiver or physical acceptance waiver is authorized.

- Build offline root/key generation and staff credential issuance using the canonical core formats; require explicit input for validity and labels.
- Produce public adoption QR data and controlled one-time staff provisioning display without committing private keys or QR images.
- Implement validation/dry-run output and documented rotation/expiry procedures; keep signing keys off relay beacons.

## Exit criteria

- [x] Tool-generated public/staff bundles round-trip through core verification and mobile import fixtures.
- [x] Expired, mismatched and malformed credentials are rejected; private material is absent from logs and tracked outputs.
- [x] An offline rehearsal provisions two staff identities and verifies their updates through a non-trusting relay.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If safe QR display/export is unavailable, provide a controlled local workflow and block staff provisioning UI integration until verified.
- Do not use a shared permanent root private key on staff phones or beacons as a shortcut.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Implemented: a separate offline Rust issuer and local Python/Tk console, explicit UTC dates and labels, an authenticated encrypted root vault with a separately recorded random unlock key, public adoption data export and an in-memory 60-second staff QR display. Existing files are never overwritten; no root private key crosses to the console, no staff QR/seed is saved by the console, and no secret is supplied through command arguments or logged in errors. See [operator workflow, format, rotation, limitations and rehearsal](../../organizer/offline-console.md). Root/staff wire formats and production trust APIs are unchanged. The only shared API addition is a synthetic test-only adapter under the existing security-probe feature.

Validation on 2026-09-17:

- Production/shared native source revision: `0b1438e4899bc4150c572b8c9cd5330241df6a6b`. Windows Rust 1.85.1: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, `cargo test --workspace --all-features --locked` (198 passed), the same tests with `--release` (198 passed), and `cargo build --workspace --all-features --locked --release` all pass. `python -B tests/integration/storage/run_policy.py` passes 14 checks. cargo-deny 0.20.2 with `--all-features --locked --config src/core/deny.toml check` passes advisories, bans, licenses and sources; only the two existing unused license allowances warn.
- Five organizer Rust tests cover authenticated vault/nonce/ciphertext/tag/truncation failures, wrong keys, expired/future/mismatched chains, canonical imports, exact event-end plus 24-hour bounds and overflow, disposable native fixture generation, and the two-staff rehearsal through the actual non-adopting relay scheduler. Final additional vault-tamper assertions were rerun in debug/release (5 passed each), with strict workspace clippy and formatting passing. The relay/SQL callback doubles are synthetic evidence, not physical radio or protected-storage certification.
- `python -B tests/integration/organizer-tools/check_console.py` passes two actual release-process pipe/export tests. `python -B tests/integration/organizer-tools/check_display.py` passes on Python 3.14.4 / Tcl-Tk 8.6.15: hidden real Tk layout, masked and cleared unlock entry, QR quiet zone, and the actual modal close callback erasing QR items before destruction. The 60,000-ms timeout is verified using an accelerated UI clock; no screenshot-protection or wall-clock endurance claim is made.
- `python -B src/core/build_bindings.py android --security-probe` passes arm64-v8a/x86_64 with NDK 27.3.13750724. Security Gradle `assembleDebug assembleRelease assembleDebugAndroidTest lintDebug` passes, followed by `testDebugUnitTest lintDebug` on the final test wiring: one Kotlin test passes all 11 import cases and actual ZXing decoding of the generated QR matrix. Generated Kotlin bindings call the canonical Rust importer with synthetic SQL callbacks. Test-only desktop JNA 5.17.0 and a strict-nullability fixture fix were necessary; both are covered by this final run. JDK 17.0.15+6, Gradle 8.13, AGP 8.11.1, Kotlin 2.2.0, compile SDK 36/build tools 35.0.0. Both security APKs pass `tests/bench/security/check_android_apk.py` ELF/native-set checks and `zipalign -c -P 16 4`; runtime 16-KiB device behavior is not claimed.
- [Mac job 105325850872](https://github.com/wickesjon/meshChat/actions/runs/35257750899/job/105325850872) passes on the production source revision above: macOS 15.7.9 arm64, Xcode 16.4 (16F6), Swift 6.1.2. Rust/Swift generation, FFI/supporter regressions, unsigned skeleton/BLE/security device and simulator builds, security curves and storage/crypto regressions all pass. The new `cargo test --locked -p meshchat-organizer --test organizer_tools generate_disposable_native_import_fixtures` plus `python3 -B tests/integration/organizer-tools/run_apple.py` passes all 11 generated Swift import/negative cases. Existing corruption fixtures intentionally catch panics; their error-parity regression passes.
- Hosted Rust and ticketboard jobs on that revision pass. The hosted Android job is unavailable due to an emulator graphics download HTTP 503 before tests; it is not recorded as a pass. Under the approved local-validation policy, the affected local Android builds/lint/package checks and actual host Kotlin import/QR checks supply the applicable evidence. This ticket changes no Android production adapter or device behavior. Existing physical/security acceptance remains MC-025/027/043/044 and the independent security gates.
- Final changes after the native source revision contain test-only JNA/fixture fixes, stronger host assertions/UI checks and ticket metadata. Relevant changed tests were rerun; mobile core, cross-build scripts, production dependencies and Swift inputs are unchanged, so the successful native build evidence applies. Final exact revision and reviewer confirmation are recorded in PR #35.
- Ticketboard generation/default validation, its 12 unit tests and `git diff --check` pass before final review. Logs remain in ignored `.work/mc041/`; no private fixture, QR image, unlock key or seed is tracked.

The pinned matrix-only qrcode 0.14.1 dependency adds no image/file/network capability; existing crypto dependency versions are unchanged. No physical phone, hardware-backed storage, complete memory-erasure or screenshot-prevention result is claimed. Full production mobile organizer workflows remain MC-030/035.

## Review and merge

- Branch: `ticket/MC-041-offline-organizer-key-and-credential-tooling`.
- Review/PR: [PR #35](https://github.com/wickesjon/meshChat/pull/35). Separate `gpt-5.6-terra` medium review of `9b6bb55902020e0b0d459cb13a8497ed93b7b6ae` found one blocker: issuance did not enforce the design's mandatory event-end plus 24-hour bound. The fix requires an explicit event end, authenticates it inside the local vault, bounds root and staff expiry, and adds exact-boundary/overflow tests. No other source findings were reported. Follow-up Terra medium review of `0b1438e4899bc4150c572b8c9cd5330241df6a6b` confirmed the blocker resolved with no new source issues. Final exact-revision/evidence review is recorded in PR #35 before merge; this staged completion remains pending that review and squash merge. The dependency gate also identified unversioned local core dependencies; these now use the exact workspace package version without changing any dependency version.
- Squash commit title: `MC-041: Offline organizer key and credential tooling`.
- Completion becomes effective only when the reviewed squash commit lands on main.
