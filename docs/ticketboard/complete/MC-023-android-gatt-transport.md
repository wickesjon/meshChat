---
id: "MC-023"
title: "Android GATT transport"
depends_on: ["MC-004","MC-016","MC-019"]
kind: "android"
branch: "ticket/MC-023-android-gatt-transport"
---

# MC-023 — Android GATT transport

## Objective

Implement scanner/advertiser, GATT server/client, CCCD setup, concrete UUIDs and runtime write/notify flow control.

## Dependencies

`MC-004`, `MC-016`, `MC-019` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/android/ble/**`, `tests/bench/android/**`, `docs/testing/**`, `src/core/src/native_transport.rs`, `src/core/src/lib.rs`, `src/core/Cargo.toml` (test registration only), `tests/integration/transport/**`, `tests/integration/ffi/**`. The additional core/binding paths are limited to the user-approved [native transport boundary](../../testing/MC-023-native-core-scope-proposal.md); no dependency or wire change is authorized.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Implement scanner/advertiser, GATT server/client, CCCD setup, concrete UUIDs and runtime write/notify flow control.
- Feed per-direction capacity and link lifecycle events to the core; preserve contiguous fragment transfer and disconnect cleanup.
- Integrate the foreground service and approved permission manifest for supported Android versions.

## Exit criteria

- [x] Automated native-adapter tests exchange whole/fragmented packets in both directions and exercise runtime capacity/backpressure through controlled callbacks; applicable Android build/lint/tests pass. Two-device physical exchange and measured backpressure remain mandatory in MC-025.
- [x] Notify subscription, busy/error callbacks, stalled writes, MTU changes and disconnects recover as specified.
- [x] The native driver forwards untrusted bytes without duplicating core parsing.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If a device exposes a smaller capacity, use the agreed supported-capacity behavior rather than oversized sends.
- On stalled links cancel bounded work and reconnect via policy; avoid unbounded retry loops.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Scheduling approved 2026-09-14 in [the validation policy](../../decisions/local-validation-policy.md#physical-acceptance-scheduling--approved-2026-09-14). Physical driver acceptance transfers to MC-025; implementation evidence must distinguish controlled callbacks/emulation from physical radio results.

Started on the dedicated branch from main `835a7966f4edb2dead99f5768ec24b75059a337c`; all three dependencies are complete. Implementation and validation are recorded below; completion still requires all remaining gates.

Integration scope approved 2026-09-16: generated Kotlin exports only the foundation byte-count/send boundary; the existing Rust HELLO/framing/ingress owners are not exposed to native callers. The user approved the [scoped core bridge and shared binding tests](../../testing/MC-023-native-core-scope-proposal.md), including the MC-019 dependency. Rust retains protocol parsing and admission; Kotlin owns native callbacks and flow control. Shared exported API changes require both Kotlin and Swift checks. MC-022's independent assessment remains a separate gate.

### Initial bridge work — incomplete ticket

`NativeTransport` now wraps the existing Friends, Ingress and Relay owners with generation-scoped connection admission, runtime directional capacity checks, fresh HELLO construction, short-lived provider proof operations, bounded send effects, completion tokens and disconnect/timeouts. Received logical effects explicitly remain unverified/opaque/pending; no display or delivery authority is inferred. The scheduler currently uses a conservative 146-byte encoding ceiling on every admitted link, while HELLO advertises measured native limits and ingress enforces the negotiated direction. This preserves the existing scheduler's pacing/credits and eight-fragment bound without claiming higher throughput. Android Normal mode is the current six-link boundary; later power integration remains MC-024.

Initial Windows evidence: Rust/cargo 1.85.1, Python 3.14.4, JDK 17.0.15+6, Kotlin 2.2.0/Gradle 8.13. Five new Rust integration tests pass (real HELLO/proof; bidirectional whole/fragmented traffic; capacity refusal; bounded connection admission; paced retry, timeout, stale-token and disconnect/clock cleanup). Workspace all-feature compilation and clippy with warnings denied pass. `python -B src/core/build_bindings.py host` generates Kotlin and Swift bindings successfully. The two `NativeTransportTest` Kotlin tests pass through the actual Android `:app:testDebugUnitTest` task, using a synthetic plaintext SQLite callback double. Initial SDK-path and nullable-Java warnings were corrected; no failed invocation is counted as a pass. Logs are under `.work/mc023/`.

At this initial snapshot, production Android GATT/scanner/advertiser/service wiring, controlled callback coverage, Swift execution/Mac native checks, release/security validation and required Terra review were outstanding. The current implementation and evidence below supersede that historical status. No physical-device result is claimed. That initial snapshot predates the separately approved MC-022 remediation, now completed on main through PR #26 (`18da5542a7c1ad19fe00109e80efafc9956890f8`).

## Review and merge

- Branch: `ticket/MC-023-android-gatt-transport`.
- Review/PR: [#27](https://github.com/wickesjon/meshChat/pull/27).
- Squash commit title: `MC-023: Android GATT transport`.
- Completion becomes effective only when the reviewed squash commit lands on main.

### Native implementation and current validation

Main through MC-022 is merged into this ticket branch. `org.meshchat.transport` in the existing BLE module now contains the actual Android scanner/advertiser, service/characteristics, GATT client/server callbacks and connected-device foreground service. The original MC-004 probe remains a separate test surface. The service accepts a protected core supplied by the future application integration; it never creates an unencrypted store or retains an unlocked identity session. Implementation contract and remaining feature ownership are recorded in [the driver guide](../../testing/MC-023-android-gatt-driver.md).

The callback driver uses measured MTU-minus-three capacities capped at 512, requires successful CCCD subscription before HELLO, bounds startup staging to two values/link, dispatches established input synchronously to Rust and preserves terminal effects on enqueue refusal. Notification submission is globally serialized for the shared Android characteristic; pending frames remain bounded to one/link, six total. Missing completions, capacity changes, disconnect, permission/radio/lock loss tear down the generation. Server teardown replaces the server epoch and closes its affected peripheral links, since Android server callbacks lack a per-connection token. Refused addresses cannot be reused within that epoch; a six-address refusal ceiling blocks further native admissions until server replacement.

Current Windows evidence after MC-022 integration: all 156 core tests pass in debug and release; core all-target/all-feature clippy and formatting pass. Host generation and both arm64-v8a/x86_64 Android release libraries build with Rust 1.85.1 and NDK 27.3.13750724. Refreshed cargo-deny advisories/bans/licenses/sources pass with existing unmatched allowance warnings only. Final `assembleDebug assembleRelease lintDebug` passes, including 14 JVM tests after the review correction: eight real-core controlled-callback cases, two production notification-queue cases, two transport-binding cases, the foundation test and the nested crypto-vector runner (two additional native-vector tests). Debug/release APKs contain the correct Rust/JNA libraries for both ABIs with 16 KiB ELF LOAD/RELRO alignment; ZIP alignment checks also pass. The lint entry point now requires callback tests and APK verification. Swift/Mac execution and Terra review subsequently passed as recorded below. Setup failures from sandboxed NDK access and missing repository-local rustup on PATH were corrected; they are not passing checks. Logs remain under `.work/mc023/`.

The review candidate uses the same source as the local checks. Source-adjacent test directories were not introduced. All new tests are under the approved integration/bench paths. No native radio or physical provider result is claimed.

### Terra review correction

The separate Terra medium reviewer inspected candidate `3dfd85910c6dc33a3afa68bd6eb632220a06ea65` against main `18da5542a7c1ad19fe00109e80efafc9956890f8` and requested one P1 correction: the server ignored the Boolean result of Android's CCCD response submission. It could enable subscription and begin HELLO despite a refused response. The adapter now commits subscription only after successful response submission; a refusal closes the connection. The production driver exposes that same acknowledgement/commit sequence to a controlled regression, which verifies refused response => no subscription/HELLO and successful response => HELLO. The reviewer did not claim a fresh Cargo run.

After this correction, local `assembleDebug assembleRelease lintDebug` passes with all 14 JVM cases and both APK ABI/alignment checks. The change is limited to Android callback handling, its regression and evidence; Rust and Swift sources are unchanged from the initial candidate. Follow-up Terra review passed as recorded below.

### Final review and native gates

The separate Terra medium reviewer returned PASS for corrected production/test revision `95414bd771a9bfd94e1adcd2e815bcd0df46bef3`: the original CCCD P1 is fixed and no actionable findings remain. A follow-up concern about Kotlin short-circuit evaluation was explicitly withdrawn by the reviewer after checking the actual left-to-right expression; no false finding is represented as a code fix. Final metadata/board review is recorded in PR #27 against its final head.

Hosted run [35171360291](https://github.com/wickesjon/meshChat/actions/runs/35171360291) validates candidate `3dfd85910c6dc33a3afa68bd6eb632220a06ea65`. The Mac job uses Xcode 16.4 (16F6), Swift 6.1.2 and pinned Rust 1.85.1. It passes shared device/simulator library builds, the new real HELLO/proof/capacity/stale-callback/timeout Swift regression, unsigned app/BLE/security builds, CryptoKit interoperability, SQLCipher create/reopen/check/key-loss/reset, public replay/DM direction separation and native crypto-vector/error parity. Rust debug/release/storage and dependency security gates pass. All four hosted jobs pass, including Android native builds/lint/packaging, the three MC-005 emulator phases (create/reopen/key-loss) and the five MC-018 SQLCipher phases (create/reopen/checks/key-loss/reset). These use synthetic fixtures and establish functional emulator behavior only.

Applicability after review: only Kotlin callback handling, its regression and documentation changed from that hosted candidate. Shared Rust, generated API inputs, Swift, security providers, persistence, native dependency versions and CI scripts are identical. Corrected Android source at `95414bd` separately passes local debug/release/lint, all 14 JVM cases, both ABI ELF LOAD/RELRO checks and both APK 16 KiB ZIP-alignment checks. Thus the earlier Mac/Rust/security evidence remains applicable under the approved local-validation policy; no unavailable or cancelled run is claimed as a pass. Physical-device evidence remains with MC-025/027/043/044.

The final commit changes only this ticket's conditional completion location and evidence plus generated board files. Board regeneration/default validation, all 12 validator unit tests and `git diff --check` pass. Completion is effective only after the reviewed squash merge of PR #27; the merge checkbox is deliberately not asserted before that event.
