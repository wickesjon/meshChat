# MC-033 Android phone Beacon Mode

## Implemented behavior

Android exposes a manual Beacon setting and an encrypted auto-beacon-while-charging preference. The status display uses the free dark palette at window-local 6% brightness, moves its content once per minute, and requires a two-second press or the accessibility long-click action to exit. It restores the previous window brightness and keep-screen-on setting on exit. It does not change system settings. A warning precedes activation without assuming a battery benefit. Enabling manual or automatic mode, and restoring either preference at app launch, starts the existing transport service through its normal permission and lock checks. The status screen offers permission review and a connection retry when the service is unavailable.

The existing shared power/relay/cache policies remain authoritative: up to eight setup/live connections, 10–40 ms relay hold-off, no additional mode forwarding bucket, and all aggregate ingress/egress, signature, connection-attempt and memory limits. Scan failures, permission/lifecycle restrictions and the RSSI anti-clustering heuristic remain enforced. Fresh ANNOUNCE peer-count claims only prefer retaining sparsely connected peers during bounded slot rotation; they grant no trust or rate privilege. Claims expire after one minute.

Manual Beacon continues unplugged above 30%, with the infra hint immediately cleared on the next power sample. At 30% it returns to normal and reports the transition. Automatic mode enters while charging and leaves on unplug without the ordinary Auto policy's one-minute hysteresis; normal Auto Saver selection still uses that hysteresis. Reconfiguration preserves identity, protected history, ingress/egress credits, and retained links within the new ceiling. Shrinking intentionally evicts cache data outside ordinary retention/caps; it never resets protected history.

The native transport now owns the existing transient forward cache: 60 minutes/5,000 CHAT/5 MiB encoded/6 MiB allocated in Beacon, otherwise 15 minutes/500/256 KiB encoded/320 KiB allocated. Only admitted incoming or locally composed CHAT is inserted; pending signed/encrypted content stays pending. The existing bounded Sessions owner serves requests admitted by the same ingress, four objects per page and at most eight/8 KiB per session. Native completion/failure closes the corresponding served item. Stored TTL is preserved and unsolicited history wrappers remain deferred, never relayed or displayed as live traffic. Full history presentation remains with its feature owner; this change implements Beacon cache serving, not an alternate history/authentication path.

ANNOUNCE receives battery-tier and externally-powered infra bits before signing; cosmetics and trust are unaffected. The exported metrics are local counts only: cached objects/bytes, allocated cache bytes, relay frame attempts, attempted egress bytes, refused and queued objects, and mode uptime. There are no message bodies, identifiers, addresses, RSSI histories or unique-person estimates in these metrics. Attempts are not successful deliveries.

## Development validation

Native/core tests in `tests/bench/beacon/native.rs` exercise six logical hours with a genuine native HELLO connection, admitted traffic from three synthetic senders, forwarding to another native link, exact first-arrival expiry, bounded memory/queues, manual downgrade, automatic charge/unplug transitions, eight-link setup capacity, and a fragmented SYNC request with four-item page boundary. A separate ANNOUNCE test checks powered/unpowered/cutoff hints and iOS refusal. Android policy tests cover continuous low-latency scanning with permission/backoff gates and bounded sparse-peer retention. The UI fixture checks protected preference restart, rejection of an ordinary tap, accessible exit and brightness restoration. The runner also performs a real Android 2.3-second stationary touch outside Compose's virtual clock, waits for the protected save and normal screen, then verifies both Beacon preferences remain disabled after restart.

Commands (all caches and outputs inside the repository):

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo test --workspace --all-features --locked --release
cargo build --workspace --all-features --locked --release
python -B tests/integration/storage/run_policy.py
cargo-deny --all-features --locked --config src/core/deny.toml check
python -B src/core/build_bindings.py android
src/android/gradlew.bat -p src/android --no-daemon --max-workers=2 :app:assembleDebug :app:assembleRelease :app:assembleDebugAndroidTest :app:lintDebug :app:testDebugUnitTest
python -B tests/integration/android-ui/run_emulator.py --serial emulator-5580
python -B tests/bench/beacon/run_android.py --serial emulator-5580
src/android/gradlew.bat -p src/android/ble --no-daemon --max-workers=2 assembleDebug assembleRelease lintDebug testDebugUnitTest verifyTransportApks
python -B tests/integration/android-ui/check_apk.py src/android/app/build/outputs/apk/debug/app-debug.apk src/android/app/build/outputs/apk/release/app-release-unsigned.apk
zipalign -c -P 16 4 <each-production-apk>
```

Rust formatting, strict clippy, 193 debug and 193 release tests, release build, fourteen storage-policy tests and cargo-deny passed on core revision `c47c028448d64e8d4b19df3d8b4aafdecac70f31`. The four native Beacon tests are included. Kotlin/Swift generation and both Android ABIs passed. Android app Debug/Release/test-APK builds, lint and eighteen JVM tests passed again after the final gesture change; BLE Debug/Release/lint and twenty-six JVM tests passed on the unchanged core/driver inputs. Production APK native-set, ELF LOAD/RELRO and ZIP 16 KiB alignment checks passed. Static alignment does not establish runtime 16 KiB device support.

The full [Mac native job](https://github.com/wickesjon/meshChat/actions/runs/35251216171/job/105303956941) passed at that core revision: Swift FFI/driver/Beacon/Supporter regressions, app and BLE Debug/Release builds, and security-probe device/simulator builds and crypto checks. Later changes affect only Android UI, its test runner and evidence; Mac/core inputs are identical. The first Mac run exposed the removed SYNC-request diagnostic event; the final revision restores it while keeping direct requests out of relay digest/cache/activity state, with regression coverage. Hosted Android provisioning failed while downloading the unchanged Compose graphics source (HTTP 503); it is unavailable evidence. Local builds use the existing pinned SQLCipher/graphics dependencies under the approved local-validation policy. No branch-protection override is authorized.

The emulator uses a 320x640/density160 viewport. Channel, friend, sharing and synthetic billing suites passed; final Beacon and channel results and the exact final reviewed revision are recorded in the ticket/PR. An early touch runner killed the app before its asynchronous protected save completed; it now waits for the published normal screen before checking persistence. The stable gesture key keeps status recomposition from restarting an active hold. Toolchain: Rust1.85.1, cargo-deny0.20.2, JDK17.0.15+6, Gradle8.13, Kotlin2.2.0, AGP8.11.1, SDK36/build-tools35.0.0, NDK27.3.13750724; emulator API29 x86_64/default revision8, emulator37.1.11, adb37.0.1, WHPX; native Mac uses macOS15 arm64, Xcode16.4 (16F6), Apple Swift6.1.2. Synthetic timing and emulator results are not physical certification.

Final production APK SHA-256: Debug `9e0a486e95a832b9d9b1ef5b0bf08f2838478e8078ed7295bf18161b9df56305`; unsigned Release `e8a7d8cbead0108915e3d9c6c9dd7522867dfa706ca369468d4f5c6f9b27a829`. Detailed local logs and synthetic screenshots remain ignored under `.work/mc033/`.

## Required MC-025 physical procedure

Use only synthetic traffic until MC-043 permits sensitive data. Record build/APK hashes, named phone model/OS, battery health, ambient temperature, charger/power meter, actual sustained native connection count, foreground/background/lock state and discovery permissions. Never disable protected-store locking or security settings to obtain a passing run. A lock/lifecycle stop is a recorded limitation/failure for the relevant scenario, not evidence of screen-off relay continuity.

1. Run a powered Android Beacon for six real hours. Maintain a paced mixed workload and fresh arriving peers; record aggregate snapshots each minute and peak queue/cache reservations. Check eight links only where measured directional capacity, backpressure and device support allow them. Report any lower measured limit explicitly. Verify the expected 60-minute cache age and bounded page/session serving, never promise full-hour replay to each arrival.
2. Unplug above 30%: manual mode remains active, infra clears within the power-sampling interval. At/below 30%, confirm downgrade notice, six-link ceiling and ordinary cache shrink while protected history remains. Repeat with auto-beacon: plug enters, unplug leaves, replug re-enters. Exercise locked/screen-off/OEM termination and missing permissions separately; a stopped service is not a power-success result.
3. Predeclare a sparse corridor with endpoints out of direct range and a fixed intermediate Beacon location. Repeat identical topology/workload with Beacon enabled, ordinary relay mode and relay absent, randomized order, at least three repetitions per condition. Record actual received/delivered counts and latency at endpoints using synthetic message IDs in the test harness only, plus each phone's aggregate TX attempts/bytes and battery/load. Include same-device idle controls and the MC-007 battery protocol. Keep screen-on status-display and screen-off runs separate.
4. Compare paired delivery, gap coverage, TX attempts, energy (when measured), raw and incremental percentage-point drain/hour, load and failures. Report uncertainty and runs that did no required work. No battery offload, range or coverage improvement is claimed before these observations. Retain raw synthetic bench logs under ignored `.work/`; publish only aggregate reports under docs/testing with named evidence.

No real handset endurance, battery, OEM/background, radio range, physical capacity or hardware-protection result is claimed by MC-033. MC-025 owns these gates before MC-034; independent assessments remain separate.
