# MC-025 connection setup order — 2026-09-26

## Finding and repair

Following the [MTU trials](MC-025-mtu-callback-results-2026-09-26.md), a deterministic regression reproduces a shared transport defect: two live connections can complete HELLO in reverse creation order, but the native owner previously registered their SYNC bookkeeping at HELLO admission. The session table intentionally rejects registration at or below its generation high-water mark. Admitting the newer generation first therefore makes admission of the older live generation return `TransportError::Unavailable`. Android's driver responds to that error by stopping all its connections.

The repair moves the bounded SYNC registration to native connection creation, where generation order is authoritative. It retains the session table's stale-generation protection, the same scheduler and admission credits, measured capacity activation, HELLO/proof checks, and catch-up/intake authorization. Registration failure closes the newly created link; ordinary setup failure/disconnect also releases the reserved bookkeeping. No wire or FFI signature, dependency, retry interval, storage/key lifecycle or Android error policy changes.

The regression in `tests/integration/transport/native.rs` completes each local HELLO send, then delivers the newer connection's remote HELLO before the older one's. On unchanged core it fails with `Unavailable` (`reverse-hello-baseline.log`). With the repair it passes in both Android and iOS host transport modes. It additionally verifies that early catch-up is refused, both connections admit without close events, the older generation can request catch-up after admission, disconnect makes the old handle stale, and a subsequent generation starts unadmitted. These host tests exercise production Rust and do not certify an iOS native build or hardware behavior.

## Unchanged-app physical diagnosis

Devices remain Galaxy S24 Ultra SM-S928U1 and SM-X510, Android 16/API 36, with existing synthetic profiles, USB powered. Production Debug app remains `5fd6ec9ade1d28353cf4399c3e42b2e932609166d34bac7645c3c07954985dad` throughout D01–D07.

The opt-in `PhysicalSetupTraceTest` records bounded setup call results, local tokens, close call paths and snapshots while forwarding every real radio operation and event exactly once. The refined observer owns an extra generated UniFFI reference to the same Rust transport object; it observes native creation/receive without replacing native behavior, then restores fields and releases its own reference. No addresses, messages, identities, key material or exception messages are recorded. Diagnostics flush after the radio stops; overflow fails the capture. Attachment requires no existing connections. A completed 40-second capture is explicitly **not** a connectivity acceptance pass.

D01 reproduces the early tablet shutdown: accepted MTU/subscription/send operations are followed by `GattDriver.stop` from `GattDriver.value` while handling a server characteristic write. Both local connections close. This identifies the receive-exception path that the earlier five-second snapshots missed. The original trace did not record the exception subtype or native registration branch. The deterministic generation-order failure is consistent with this path; do not claim that a debugger captured the exact Rust branch in M02 or D01.

| Capture | Physical A / B | A / B runner seconds | Outcome |
|---|---|---|---|
| D01 | Tablet / phone | 43.278 / 41.949 | Both captures complete; tablet receive-path driver stop reproduced |
| D02 | Tablet / phone | 43.866 / 41.593 | Debugger-assisted capture; no receive-path driver stop |
| D03 | Tablet / phone | Incomplete / 41.993 | Tablet flushed trace but exceeded host runner deadline; explicit cleanup |
| D04 | Tablet / phone | 43.133 / 41.808 | Refined capture complete; no receive exception observed |
| D05 | Tablet / phone | 42.926 / 41.687 | Refined capture complete; no receive exception observed |
| D06 | Phone / tablet | 41.712 / 43.550 | Refined capture complete; no receive exception observed |
| D07 | Tablet / phone | 2.605 failure / 41.627 | Tablet's attachment guard correctly rejects a connection that arrived before tracing |

Initial test APK for D01–D03: `76727fa45b96133ff8efb8c335cd07b29a5e971877553b6f7337c151b4e65744`. Refined D04–D07 test APK: `5f973ace5fc748332814c1061b9ab63834b086a6a2ea55b3c7d392d0be4e1eb4`. The later final test also records native creation/admission generations and guarantees cleanup after stop-wait failures; that refinement is not retroactive evidence for these earlier captures.

D03's debugger paused at normal driver shutdown, not at the target exception; the tablet produced no final runner summary after its trace flush. The host deadline expired at 150 seconds and the tablet was explicitly force-stopped before further runs. Preserve `D03/interruption.txt`; this is debugger-affected incomplete diagnostic evidence, not a demonstrated production crash. D07 is an attachment failure, not a passed trace. All other retained trace summaries report zero dropped entries. Temporary debugger forwarding was removed.

## Validation

Rust 1.85.1/cargo 1.85.1: formatting, strict workspace/all-target/all-feature clippy, **212 debug tests**, **212 release tests** and release build pass. The targeted regression fails before the repair and passes afterward; the session table's existing stale-generation tests remain unchanged. Dependency advisories were refreshed and cargo-deny 0.20.2 reports advisories, bans, licenses and sources passing. Existing unused license allowance warnings remain warnings.

Normal and security-feature Android native libraries rebuild for arm64-v8a and x86_64 with generated bindings. Android app Debug/Release/androidTest builds and lint pass; standalone BLE build/lint/APK checks pass. Because Gradle initially reused JVM results despite the rebuilt native DLL, app and BLE JVM tests were explicitly rerun: **26 app + 29 BLE tests pass**, zero failures/errors/skips. App Debug/Release native-set/ELF and `zipalign -c -P 16 4` checks pass. Both devices' installed app/test hashes match this candidate:

| Artifact | SHA-256 |
|---|---|
| Debug app | `f06adf5b46601cae15c374b88d316bd7cece1fca20c04a60fa987c0b9f4ecc84` |
| Unsigned Release app | `ec54503ec02967eb308e5f1106cad6d9f5e7f306945232926446d1baef4a8b26` |
| Instrumentation APK | `19fa50400043d8e2254c1a8b72ce9bae55dbf0c1c76d9fdc2ec2b2dbb8a290c6` |

Repaired setup capture E01 completes on tablet A / phone B in 43.332 / 41.844 seconds, zero dropped entries. Both devices create and admit generations 3 then 5 without receive exceptions. This first physical capture does not exercise reversed generation completion; the deterministic host regression does. Four paired message/stop-start plans K01–K04 (seven pairs each, both physical initiators and both command launch orders) were declared before execution in `paired-plan.json` and are in progress.

Commands and logs (all generated output remains inside the repository under `.work/mc025/2026-09-26-setup/`):

```text
cargo fmt --all -- --check
cargo clippy --offline --workspace --all-targets --all-features --locked -- -D warnings
cargo test --offline --workspace --all-features --locked
cargo test --offline --workspace --all-features --locked --release
cargo build --offline --workspace --all-features --locked --release
cargo-deny.exe --manifest-path src/core/Cargo.toml --config src/core/deny.toml fetch
cargo-deny.exe --offline --locked --manifest-path src/core/Cargo.toml --config src/core/deny.toml check
python -B src/core/build_bindings.py android
python -B src/core/build_bindings.py android --security-probe
src/android/gradlew.bat -p src/android --offline --no-daemon --max-workers=2 :app:assembleDebug :app:assembleRelease :app:assembleDebugAndroidTest :app:lintDebug :app:testDebugUnitTest
src/android/gradlew.bat -p src/android/ble --offline --no-daemon --max-workers=2 lintDebug
src/android/gradlew.bat -p src/android --offline --no-daemon --max-workers=2 :app:assembleDebugAndroidTest :app:lintDebug :app:testDebugUnitTest --rerun-tasks
src/android/gradlew.bat -p src/android/ble --offline --no-daemon --max-workers=2 testDebugUnitTest --rerun-tasks
```

The process-local environment uses repository Rust/Gradle/cache/temp directories, JDK 17.0.15+6, Gradle 8.13, AGP 8.11.1, Kotlin 2.2.0, SDK 36/build tools 35.0.0 and installed read-only NDK 27.3.13750724. Initial tool invocations failed because cargo-deny/rustup were absent from that shell's PATH and the cargo-deny config flag was placed after its subcommand. Corrected invocations use the existing repository executables and proper flag order; initial failures remain in `cargo-deny.log`, `cargo-deny-pinned.log` and `android-native-build.log`. No tool/dependency upgrade or global settings change was made.

## Limits and review

The shared core change has no FFI/wire change. Android native consumption is rebuilt; iOS host-mode tests are not Mac/Xcode compilation. Mac/native iOS checks are unavailable here and remain open alongside broader MC-025/027 physical, energy and independent security gates. USB-powered diagnostics establish no battery drain result. Source review and the declared paired message/reconnect trials are pending; PR #39 stays unmerged and MC-025 in progress.
