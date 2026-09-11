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

- [ ] Both native skeletons call the same core through generated bindings.
- [ ] A deterministic event trace produces identical commands across repeated runs.
- [ ] Malformed input and disconnect sequences return defined errors without unwinding into native callers or leaking link state.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If callback-heavy bindings are fragile, use pull-based event/command batches with the same bounded semantics.
- If the selected panic strategy cannot recover, keep parser paths total and document the process-level failure policy; never claim catch_unwind catches aborts.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Implementation started from main `f5acefe4ea201cb7aa4ce6b0c71e1a202b234901`, where MC-002 is complete after reviewed squash merge [PR #4](https://github.com/wickesjon/meshChat/pull/4). MC-003 is submitted for review; hosted native checks and independent review remain required before completion.

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
- Local Kotlin/JVM execution of the generated bindings passed its deterministic binary round-trip and typed-error regression against the host Rust DLL. Strict lint then caught the generator's default JVM cleaner referencing API 33; enabled UniFFI's Android-specific cleaner configuration, preserving minSdk 29 and the NewApi lint gate. Final Android rerun is pending.
- Full-feature cargo-deny passed advisories, licenses, versions and sources. Expected unused baseline license allowances remain warnings.
- Ticketboard validation, actionlint 1.7.7 (without shellcheck) and `git diff --check` passed.

## Review and merge

- Branch: `ticket/MC-003-sans-io-and-uniffi-foundation`.
- Review/PR: pending.
- Squash commit title: `MC-003: Sans-IO and UniFFI foundation`.
- Completion becomes effective only when the reviewed squash commit lands on main.
