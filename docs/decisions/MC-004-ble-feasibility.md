# MC-004 physical BLE measurement plan

State: procedure and probe source prepared. Android probe assembly/lint passed locally; iOS source is unbuilt. No device execution, measured capacity or supported-platform decision is claimed.

## Required access and inventory

The required Android/Android, Android/iOS and iOS/iOS cases need two physical Android phones, two physical iPhones, and a Mac with Xcode and development signing access. Record model, OS/build, supported ABI, Android page size where applicable, app commit/build, and Xcode/SDK versions. Use anonymous bench labels A1/A2/I1/I2; do not commit serial numbers, Bluetooth addresses, private keys or signing material. Device availability and Mac access are awaiting user input.

The baseline project targets Android API 29+ and iOS 15+, but those build settings are not evidence of supported physical devices. Missing minimum-OS coverage remains an explicit matrix gap. Simulators and hosted compilation do not close this ticket.

## Probe implementation contract

Temporary standalone probes belong under `src/android/ble/` and `src/ios/BLE/`; their manifests and project files must remain inside those permitted directories. Keep the existing app's project settings untouched. Proposed test-only service/characteristic UUIDs must be identical on both probes and recorded with the build; MC-006 owns the eventual production UUID/wire decision.

Expose central and peripheral roles, service-UUID-filtered discovery, subscribe/unsubscribe, one write without response, one notification, disconnect/reconnect and stop controls. Advertisements contain only the test service UUID. Use bounded synthetic counter/pattern values, never user messages or identity/crypto material. The event recorder uses monotonic timestamps and bench-local connection generations. Record API capacities, attempted/received lengths, readiness transitions, callback outcomes and counts; omit nearby-device identifiers and raw message content.

The Android probe must serialize GATT operations and retain both callback/API failure outcomes. The iOS probe must distinguish central write readiness from peripheral notification readiness. A queue-full return cannot count as a successful send. Clear pending operations on disconnect and verify that old callbacks do not affect the replacement connection. The sources implement this probe contract; iOS compilation and all physical execution remain unverified. Callback timeout handling is a measurement/refusal path, not the later production retry policy.

## Cases and durations

Run each applicable case three times, recording every attempt and timeout. The following durations are measurement windows, not promised product performance.

| Case | Procedure | Required observations |
|---|---|---|
| Discovery | Start disconnected; scan for 60 seconds, then reverse central/peripheral roles | Found/not found, elapsed time, permission and radio state |
| Established link | Connect in foreground, exchange synthetic values in both directions for 10 minutes | Separate writes/notifications, loss/duplicates, readiness and disconnect times |
| Background | Repeat discovery and held-link cases with one then both apps backgrounded | Separate discovery and connection-survival outcomes |
| Screen lock / suspension | Lock screens after establishment; retain a 10-minute timeline | Record what process state is actually observable; never equate background with confirmed suspension |
| User termination | Explicitly terminate each app, then test discovery and existing-link behavior | User action, disconnect time, manual relaunch and recovery; distinguish Android task dismissal from force-stop |
| Directional capacity | Record both platform-reported write/notify limits; try boundary-sized synthetic values | Accepted, rejected and received lengths separately for each direction |
| Low MTU | Request the smallest controllable negotiation, record actual result, retry boundary values | Do not claim MTU 23 if the OS negotiates a different value |
| Backpressure | Send a bounded burst exceeding immediate readiness, then allow it to drain | Queue bound, readiness transitions, failures and eventual receive counts; no busy retry loop |
| Duplicate connection | Both peers initiate concurrently, then disconnect one path | Both connection generations, surviving path and stale-callback behavior |
| Recovery | Toggle radio and revoke/restore permission through normal OS controls | Refusal, cleanup, fresh permission/connection path and retained state |

Cover A1/A2, I1/I2, and at least one Android/iPhone pairing in both role directions. Record OS battery restrictions and service/background configuration with each result. Cases impossible under a platform's documented discovery rules remain explicit unsupported cases, not silent removals or successful tests.

## Permission decision inputs

Android 12+ uses runtime scan, advertise and connect permissions. The `neverForLocation` assertion is conditional on actual use and can filter some beacons; proximity/RSSI product behavior must be resolved before adopting it. Android 10/11 background discovery has additional location requirements. Record denied, revoked and granted outcomes and retain the permission rationale with the measured matrix. Source: [Android Bluetooth permissions](https://developer.android.com/develop/connectivity/bluetooth/bt-permissions).

Apple documents different background scan/advertising behavior and limited wake time. Test discovery separately from an already-established connection, and record background modes and actual process state. Do not infer background Android discovery of an iOS advertiser from foreground success. Source: [Core Bluetooth background processing](https://developer.apple.com/library/archive/documentation/NetworkingInternetWeb/Conceptual/CoreBluetooth_concepts/CoreBluetoothBackgroundProcessingForIOSApps/PerformingTasksWhileYourAppIsInTheBackground.html).

These are planning inputs, not the final permission ruling or proof of current hardware behavior.

## Evidence and completion

For each run retain: run ID, bench labels, device/OS/build inventory, source commit, role direction, start/end monotonic times, requested and observed lifecycle state, permissions, reported and measured capacities, sent/received/error counts, bounded trace path, reproduction steps and outcome. Separate API enqueue acceptance, transport callbacks and receiver observation.

Commit only reviewed sanitized evidence under `tests/bench/` and the resulting decision under `docs/decisions/`. Keep raw temporary captures inside ignored repository paths. Do not fill missing outcomes with zeros or simulation. MC-004 stays incomplete until real results establish the permission decision and supported matrix; MC-006/007 remain blocked until its reviewed squash merge.

## Build and run the prepared probes

Use the repository-local Rust/Android tool/cache environment documented in MC-002/003; keep Gradle/Java temporary directories, preferences and debug keystore under `.work/`. From the repository root:

```sh
bash src/android/gradlew -p src/android/ble --no-daemon assembleDebug assembleRelease lintDebug
```

Windows uses `src/android/gradlew.bat` with the same arguments. The debug APK is `src/android/ble/build/outputs/apk/debug/MeshChatBleProbe-debug.apk`. This standalone app uses no Rust/JNA library and does not change the main app.

On a Mac with Xcode 16.4, run a build before signing or hardware testing:

```sh
mkdir -p .work/ble-ios-derived-data .work/ble-module-cache .work/tmp
TMPDIR="$PWD/.work/tmp" xcodebuild -project src/ios/BLE/BLEProbe.xcodeproj -scheme BLEProbe \
  -configuration Debug -destination 'generic/platform=iOS Simulator' \
  -derivedDataPath "$PWD/.work/ble-ios-derived-data" \
  CLANG_MODULE_CACHE_PATH="$PWD/.work/ble-module-cache" CODE_SIGNING_ALLOWED=NO build
```

Repeat Release. For the actual iPhone, configure development signing locally and build the device destination without that command-line signing override. No checked-in target disables signing. A simulator build does not establish BLE behavior.

Test fixture UUIDs: service `5f45c0de-71a5-4f81-9f52-52f1ee004001`, write `...4002`, notify `...4003`. Android also supplies the standard CCCD. These are deliberately not a production UUID decision. Pair only controlled bench probes using this fixture service.

On Android grant permissions, start the foreground service, and separately select Scan or Advertise. Both roles may run concurrently. On iOS select the matching role; the system handles Bluetooth permission. Start with 20-byte single writes/notifications, then use the observed capacity and bounded burst controls. Values begin with a synthetic sequence counter when at least four bytes long. Trace records omit raw payloads, names and device addresses.

Enable two-second traffic before background/lock measurements. Inbound delivery is the evidence; API enqueue and callbacks alone do not prove reception. Record the source commit and device inventory alongside each copied sanitized report. A report with dropped rows or state restoration cannot be presented as a complete uninterrupted trace. Android revocation stops the engine: copy its report, stop the service, grant permission and start a fresh session to test recovery. iOS Stop removes services/advertising but its peer owns inbound disconnect.

No supported matrix or final Android permission ruling is selected until the physical runs are reviewed.
