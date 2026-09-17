# MC-033 Android phone Beacon Mode

## Implemented behavior

Android exposes a manual Beacon setting and an encrypted auto-beacon-while-charging preference. The status display uses the free dark palette at window-local 6% brightness, moves its content once per minute, and requires a two-second press or the accessibility long-click action to exit. It restores the previous window brightness and keep-screen-on setting on exit. It does not change system settings. A warning precedes activation without assuming a battery benefit.

The existing shared power/relay/cache policies remain authoritative: up to eight setup/live connections, 10–40 ms relay hold-off, no additional mode forwarding bucket, and all aggregate ingress/egress, signature, connection-attempt and memory limits. Scan failures, permission/lifecycle restrictions and the RSSI anti-clustering heuristic remain enforced. Fresh ANNOUNCE peer-count claims only prefer retaining sparsely connected peers during bounded slot rotation; they grant no trust or rate privilege. Claims expire after one minute.

Manual Beacon continues unplugged above 30%, with the infra hint immediately cleared on the next power sample. At 30% it returns to normal and reports the transition. Automatic mode enters while charging and leaves on unplug without the ordinary Auto policy's one-minute hysteresis; normal Auto Saver selection still uses that hysteresis. Reconfiguration preserves identity, protected history, ingress/egress credits, and retained links within the new ceiling. Shrinking intentionally evicts cache data outside ordinary retention/caps; it never resets protected history.

The native transport now owns the existing transient forward cache: 60 minutes/5,000 CHAT/5 MiB encoded/6 MiB allocated in Beacon, otherwise 15 minutes/500/256 KiB encoded/320 KiB allocated. Only admitted incoming or locally composed CHAT is inserted; pending signed/encrypted content stays pending. The existing bounded Sessions owner serves requests admitted by the same ingress, four objects per page and at most eight/8 KiB per session. Native completion/failure closes the corresponding served item. Stored TTL is preserved and unsolicited history wrappers remain deferred, never relayed or displayed as live traffic. Full history presentation remains with its feature owner; this change implements Beacon cache serving, not an alternate history/authentication path.

ANNOUNCE receives battery-tier and externally-powered infra bits before signing; cosmetics and trust are unaffected. The exported metrics are local counts only: cached objects/bytes, allocated cache bytes, relay frame attempts, attempted egress bytes, refused and queued objects, and mode uptime. There are no message bodies, identifiers, addresses, RSSI histories or unique-person estimates in these metrics. Attempts are not successful deliveries.

## Development validation

Native/core tests in `tests/bench/beacon/native.rs` exercise six logical hours with a genuine native HELLO connection, admitted traffic from three synthetic senders, forwarding to another native link, exact first-arrival expiry, bounded memory/queues, manual downgrade, automatic charge/unplug transitions, eight-link setup capacity, and a fragmented SYNC request with four-item page boundary. A separate ANNOUNCE test checks powered/unpowered/cutoff hints and iOS refusal. Android policy tests cover continuous low-latency scanning with permission/backoff gates and bounded sparse-peer retention. The UI fixture checks protected preference restart, rejection of an ordinary tap, accessible exit and brightness restoration.

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
```

Final exact revisions, outcomes and native Mac evidence are recorded in the ticket/PR. Initial production Android build/lint/JVM checks and the initial three core Beacon scenarios passed; final checks including the additional ANNOUNCE test and UI execution are in progress. Toolchain remains Rust1.85.1, JDK17.0.15+6, Gradle8.13, Kotlin2.2.0, AGP8.11.1, SDK36/build-tools35.0.0; local emulator API29 x86_64/default revision8, emulator37.1.11, adb37.0.1, WHPX. Synthetic timing and emulator results are not physical certification.

## Required MC-025 physical procedure

Use only synthetic traffic until MC-043 permits sensitive data. Record build/APK hashes, named phone model/OS, battery health, ambient temperature, charger/power meter, actual sustained native connection count, foreground/background/lock state and discovery permissions. Never disable protected-store locking or security settings to obtain a passing run. A lock/lifecycle stop is a recorded limitation/failure for the relevant scenario, not evidence of screen-off relay continuity.

1. Run a powered Android Beacon for six real hours. Maintain a paced mixed workload and fresh arriving peers; record aggregate snapshots each minute and peak queue/cache reservations. Check eight links only where measured directional capacity, backpressure and device support allow them. Report any lower measured limit explicitly. Verify the expected 60-minute cache age and bounded page/session serving, never promise full-hour replay to each arrival.
2. Unplug above 30%: manual mode remains active, infra clears within the power-sampling interval. At/below 30%, confirm downgrade notice, six-link ceiling and ordinary cache shrink while protected history remains. Repeat with auto-beacon: plug enters, unplug leaves, replug re-enters. Exercise locked/screen-off/OEM termination and missing permissions separately; a stopped service is not a power-success result.
3. Predeclare a sparse corridor with endpoints out of direct range and a fixed intermediate Beacon location. Repeat identical topology/workload with Beacon enabled, ordinary relay mode and relay absent, randomized order, at least three repetitions per condition. Record actual received/delivered counts and latency at endpoints using synthetic message IDs in the test harness only, plus each phone's aggregate TX attempts/bytes and battery/load. Include same-device idle controls and the MC-007 battery protocol. Keep screen-on status-display and screen-off runs separate.
4. Compare paired delivery, gap coverage, TX attempts, energy (when measured), raw and incremental percentage-point drain/hour, load and failures. Report uncertainty and runs that did no required work. No battery offload, range or coverage improvement is claimed before these observations. Retain raw synthetic bench logs under ignored `.work/`; publish only aggregate reports under docs/testing with named evidence.

No real handset endurance, battery, OEM/background, radio range, physical capacity or hardware-protection result is claimed by MC-033. MC-025 owns these gates before MC-034; independent assessments remain separate.
