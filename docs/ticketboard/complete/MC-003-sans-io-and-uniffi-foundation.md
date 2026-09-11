---
id: "MC-003"
title: "Sans-IO and UniFFI foundation"
depends_on: ["MC-002"]
kind: "foundation"
branch: "ticket/MC-003-sans-io-and-uniffi-foundation"
---

# MC-003 — Sans-IO and UniFFI foundation

## Objective

Define events for peer connection, disconnection, inbound bytes, link capacities, time and power changes; return bounded send commands and UI events.

## Dependencies

`MC-002` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/core/**`, `src/android/**`, `src/ios/**`, `tests/integration/ffi/**`, `Cargo.lock`, `.github/workflows/ci.yml`.

The user explicitly approved adding `Cargo.lock` for UniFFI dependencies and `.github/workflows/ci.yml` for Kotlin/Swift binding checks on 2026-09-11. No other scope expansion is implied.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Define events for peer connection, disconnection, inbound bytes, link capacities, time and power changes; return bounded send commands and UI events.
- Inject monotonic time and randomness for reproducible tests; keep radio and platform key access behind explicit interfaces.
- Prove generated Kotlin and Swift bindings with a small round trip and defined FFI error behavior; inspect UniFFI panic handling before adding redundant wrappers.

## Exit criteria

- [x] Both native skeletons call the same core through generated bindings.
- [x] A deterministic event trace produces identical commands across repeated runs.
- [x] Malformed input and disconnect sequences return defined errors without unwinding into native callers or leaking link state.
- [x] Implementation checks and independent review are complete; completion is staged for the squash merge and becomes effective only on main. The exact final PR revision must pass CI before merge.

## Potential fallbacks

- If callback-heavy bindings are fragile, use pull-based event/command batches with the same bounded semantics.
- If the selected panic strategy cannot recover, keep parser paths total and document the process-level failure policy; never claim catch_unwind catches aborts.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Implementation started from main `f5acefe4ea201cb7aa4ce6b0c71e1a202b234901`, where MC-002 is complete after reviewed squash merge [PR #4](https://github.com/wickesjon/meshChat/pull/4). MC-003 implementation is tested and independently reviewed, with completion staged pending the final CI run and squash merge.

### Implementation choices and boundaries

- UniFFI 0.29.4 generates Kotlin and Swift from the same Rust library metadata. Runtime and CLI are pinned together; the CLI feature is not enabled in Android/iOS release libraries. The locked transitive versions `textwrap 0.16.2`, `serde 1.0.228` and `clap 4.5.32` retain Rust 1.85.1 compatibility and one `syn` version. Eight exact-version MPL-2.0 allowances cover the explicitly selected UniFFI packages; they must be reassessed on upgrade. Cargo-deny now checks all features, including generator dependencies. No advisory or duplicate-version exclusion was added.
- Core calls accept established connections, directional capacities, inbound values, injected monotonic time and power events. An explicit outbound admission call returns a driver command. Each result contains at most one command/event, the core retains no payloads or queues, and connection count/value size are bounded by caller configuration. Native skeleton values (one link, 64 bytes) are FFI exercise limits, not the MC-007 mesh budget or MC-004 supported BLE capacity.
- Native code supplies a nonzero random instance nonce and monotonic time. Tests inject fixed values. Handles combine that nonce with a monotonically increasing generation; disconnect removes link state and old callbacks cannot act on a replacement link. The nonce is not peer identity, a cryptographic key or a wire field. Drivers must serialize lifecycle/event dispatch, discard emitted commands on disconnect, and avoid reusing an instance nonce when replacing an engine.
- Incoming bytes are only size-checked and observed as a diagnostic count, never displayed as authenticated messages or echoed over a radio. Wire parsing, reassembly, ingress scheduling, transmission completion and power policy belong to later tickets. Empty/oversized values, invalid capacity, time regression and unknown/stale handles return typed errors before state mutation.
- `PlatformKeyProvider` defines an opaque platform handle/public-key/signing boundary without exporting private key material. This foundation does not call a key operation or choose an algorithm; MC-005/008/017 own feasibility, cryptographic construction and the implemented provider. No protected-key or cryptographic evidence is claimed.
- Inspected UniFFI 0.29.4 `uniffi_core/src/ffi/rustcalls.rs`: generated entry points already use `catch_unwind`, serialize expected errors and convert caught unwinding panics to unexpected errors. All fallible owned entry points expose `Result`; no redundant catch wrapper is added. A poisoned state mutex returns `Unavailable`. Abort, OOM, native faults and destructors are not recoverable-error guarantees. No intentional panic endpoint is shipped. Generated binding allocation happens before core validation, so native adapters must cap value sizes at their platform boundary; this is not a hostile arbitrary-FFI-caller memory sandbox.
- `src/core/build_bindings.py` builds host metadata and generates ignored bindings, then cross-compiles Android arm64/x86_64 or iOS device/simulator static libraries. Android uses NDK 27.3.13750724 and pinned JNA 5.17.0; packaging refuses missing Rust libraries. iOS links generated Swift plus Rust while preserving developer signing defaults. Run the script from the repository root before native builds, using the repository-local tool/cache environment in CI. Host mode supports the local Kotlin regression without requiring an NDK.
- Regression sources reside under `tests/integration/ffi/`: Cargo explicitly points to the Rust test target, Android's unit-test source set points to Kotlin/JUnit 4.13.2, and macOS compiles the Swift executable against the host Rust library. Fixtures cover bytes 0/127/255, defined malformed-input and lifecycle errors, resource release and deterministic repetition. CI separately builds both Android variants and iOS simulator configurations; no phone execution or BLE evidence is claimed.

### Validation

- Local Rust format, strict all-feature Clippy, and all four lifecycle/limit regression tests in debug and release passed. Generated Kotlin and Swift sources were produced from the compiled library.
- Local Kotlin/JVM execution of the generated bindings passed its deterministic binary round-trip and typed-error regression against the host Rust DLL. Strict lint then caught the generator's default JVM cleaner referencing API 33; enabled UniFFI's Android-specific cleaner configuration and its required AndroidX annotations 1.9.1 / `android.useAndroidX` flag, preserving minSdk 29 and the NewApi lint gate. Final local `:app:testDebugUnitTest :app:lintDebug` passed (32 tasks, 13 executed).
- Full-feature cargo-deny passed advisories, licenses, versions and sources. Expected unused baseline license allowances remain warnings.
- Native packaging follow-up: [Android's NDK r27 guidance](https://developer.android.com/guide/practices/page-sizes) requires explicit 16 KB maximum/common page linker flags. Added them to the new Rust cross-build and a CI check of both APKs for the exact Rust/JNA ABI set, ELF LOAD/RELRO alignment and ZIP alignment. The pinned JNA 5.17.0 arm64/x86_64 artifacts pass the local ELF alignment check. This is binary compatibility validation, not a claim of physical 16 KB device execution.
- Ticketboard validation, actionlint 1.7.7 (without shellcheck) and `git diff --check` passed.
- [Initial hosted CI](https://github.com/wickesjon/meshChat/actions/runs/34617237786), revision `8c46df19dca4596a1113ed81ec19096e6aba9bdd`: macOS compiled all three Rust iOS targets, ran the Swift FFI lifecycle/binary/error regression successfully, and built the SwiftUI app in Debug and Release for the simulator. Android cross-compilation passed; native Gradle gates need the AndroidX flag follow-up. The final full CI run remains required.
- [Passing hosted CI](https://github.com/wickesjon/meshChat/actions/runs/34618021150), revision `c0599fad947f6431701f44164f88ff027ebc93e1`: all four jobs passed from fresh checkouts. Rust format/strict Clippy/debug and release tests/build/full-feature audit passed. Android built Debug and Release, ran Kotlin FFI regression and strict lint (2m 43s), and both APKs passed native-library presence, LOAD/RELRO and ZIP alignment checks. macOS passed Swift FFI regression and both iOS simulator builds. The later architecture assertion was checked locally against real positive and mislabeled-negative JNA binaries and independently reviewed; final CI on the staged completion revision remains required. All 12 ticketboard unit tests also passed locally.

## Review and merge

- Branch: `ticket/MC-003-sans-io-and-uniffi-foundation`.
- Review/PR: [PR #5](https://github.com/wickesjon/meshChat/pull/5), published ready for review. A separate gpt-5.6-terra worker at medium effort reviewed `8c46df19dca4596a1113ed81ec19096e6aba9bdd` against `f5acefe4ea201cb7aa4ce6b0c71e1a202b234901` and returned no actionable findings. It reviewed code, scope and evidence without running builds or modifying files. Follow-up review of the AndroidX fix `6bc3019347f0c1f90d76a939eae6b058d047054c` also returned no findings.
- Review of `c0599fad947f6431701f44164f88ff027ebc93e1` identified a packaging-check gap: ELF architecture was inferred from the ZIP folder. Added explicit `e_machine` checks (AArch64 183 / x86_64 62). Both pinned JNA binaries pass; an actual x86_64 binary presented under an arm64 path is rejected. Final follow-up review of `75d20edcb580a59f526b2e7e3178a985bc9082f1` confirmed the fix and returned no actionable findings. This final completion change updates only ticket status/evidence and the generated index. Each worker completion was awaited without polling its status.
- Squash commit title: `MC-003: Sans-IO and UniFFI foundation`.
- Completion becomes effective only when the reviewed squash commit lands on main.
