# MC-025 missing MTU callback — 2026-09-26

## Problem and source investigation

Continue the [reconnect investigation](MC-025-reconnect-results-2026-09-26.md) from `aba6a2e` on PR #39. Both devices remain Galaxy S24 Ultra SM-S928U1 and SM-X510, Android 16/API 36, existing synthetic profiles, USB powered. Previous J01/J04 exposed a tablet peripheral stuck at unknown capacity although its client received MTU 517. The opposite phone client timed out and both roles lost their shared connection. Neither launch-order workarounds nor the earlier backoff tests establish general startup reliability.

Pinned AOSP Android 16 Bluetooth revision `4b73ee6039271ffbf71ebdc8c109fc98eac8e137` supplies a concrete explanation. The [client response handler](https://android.googlesource.com/platform/packages/modules/Bluetooth/+/4b73ee6039271ffbf71ebdc8c109fc98eac8e137/system/stack/gatt/gatt_cl.cc#1100) records the negotiated ATT size in the connection control block and completes the client operation. The [server request handler](https://android.googlesource.com/platform/packages/modules/Bluetooth/+/4b73ee6039271ffbf71ebdc8c109fc98eac8e137/system/stack/gatt/gatt_sr.cc#855) updates that same property but emits server callbacks only on the incoming-request path. The [request logic](https://android.googlesource.com/platform/packages/modules/Bluetooth/+/4b73ee6039271ffbf71ebdc8c109fc98eac8e137/system/stack/gatt/gatt_api.cc#778) finds an existing measurement by remote address and LE transport. Thus a client-observed ATT size can be real even when the local server callback was absent. This is upstream implementation evidence consistent with the Samsung trace, not an audit of Samsung's complete vendor stack.

## Narrow repair

At the client's one MTU request, capture the already admitted same-device peripheral token and current local server epoch. Consume that association on the next MTU callback from the exact live BluetoothGatt object. Only a successful, in-range observed value can reach the captured token, and only while that token/address and server epoch remain unchanged. No target is created later, no size is cached for reconnection, and a second request cannot rebind an older response to a newer peer. Requested MTU 517 is never substituted for observed MTU.

The native driver still applies its 146-byte floor, 512-byte cap, conflicting-size teardown, independent CCCD readiness and HELLO/proof checks. Existing server callbacks remain supported. This changes adapter observation routing, not wire format, admission budgets, retry timing, lock/storage handling or authentication. No measured size is inferred across retired tokens, different addresses or server/client generations.

`GattMtuObservationTest` uses a real Android adapter/native admission with synthetic callback addresses and injected observations; it is not an over-the-air MTU test. Each method runs in its own instrumentation invocation with the other device's radio disabled, preserving the existing fresh-process admission precondition without refunding counters. On unchanged Debug APK `d32ccafe916f8efe9ba6456c4206569277bb3956cbeed5aee46c252df8c5c8e3`, the positive regression fails in 3.266 seconds at expected capacity 512 / actual 0. Raw evidence: `.work/mc025/2026-09-26-mtu/baseline-B.log`. Profile and identity were not reset.

## Candidate and local validation

Candidate SHA-256 identities:

| Artifact | SHA-256 |
|---|---|
| Debug app | `5fd6ec9ade1d28353cf4399c3e42b2e932609166d34bac7645c3c07954985dad` |
| Unsigned Release app | `67b8e8f264b080be90ce8bd4239374e69c13e48d71901b6750cee8159dbc5cf4` |
| Final instrumentation APK | `26db887cf943b0c7131ecff1c7615fdce43eb7d75cbba8cf7ee67065c7942d53` |

Using repository-local JDK 17.0.15+6, Gradle 8.13, AGP 8.11.1, Kotlin 2.2.0 and SDK 36, the app Debug/Release/androidTest builds, lint and 26 JVM tests pass (2m7s). Standalone BLE lint, builds, 29 JVM tests and probe APK verification pass (1m21s). After adding the final late-peripheral regression, androidTest build/lint passes again (30s). App Debug/Release `tests/integration/android-ui/check_apk.py` native-set/ELF checks and `zipalign -c -P 16 4` pass. Ticketboard validation and all 12 ticketboard tests pass. Logs are `.work/mc025/mtu-app-build.log`, `mtu-ble-build.log`, `mtu-test-final-build.log` and `.work/mc025/2026-09-26-mtu/`.

This delta changes Android callback routing and tests only: no Rust, FFI, cryptography, protected-storage implementation, wire format, dependency or iOS consumer change. Those components' prior applicable checks remain recorded in the ticket; they are not new passes for this delta. Independent security and other platform/hardware gates remain open.

Build/check commands (after `. .work/mc025/android-env.ps1` for the Android toolchain):

```text
src/android/gradlew.bat -p src/android --offline --no-daemon --max-workers=2 :app:assembleDebug :app:assembleRelease :app:assembleDebugAndroidTest :app:lintDebug :app:testDebugUnitTest
src/android/gradlew.bat -p src/android/ble --offline --no-daemon --max-workers=2 lintDebug
src/android/gradlew.bat -p src/android --offline --no-daemon --max-workers=2 :app:assembleDebugAndroidTest :app:lintDebug
python -B tests/integration/android-ui/check_apk.py src/android/app/build/outputs/apk/debug/app-debug.apk src/android/app/build/outputs/apk/release/app-release-unsigned.apk
.work/android-sdk/build-tools/35.0.0/zipalign.exe -c -P 16 4 <each app APK above>
python -B tests/ticketboard/validate.py
python -B -m unittest discover -s tests/ticketboard -v
python -B .work/mc025/check-mtu-links.py
git diff --check
```

The ignored link checker validates relative file targets in the changed design, report and ticket; 52 references resolve. Tablet callback invocations use `am instrument -w -e class org.meshchat.ui.GattMtuObservationTest#<method> -e physicalConnect true org.meshchat.app.test/androidx.test.runner.AndroidJUnitRunner`, one invocation per row below. Existing retirement/refusal/restart classes use `physicalConnect=true` and `physicalBluetooth=true`.

## Tablet callback regressions

The unchanged-app positive regression failed as recorded above. With the repaired app, the same installed test passes in 3.570 seconds. The final ten-case test package then passes **10/10**, one fresh instrumentation invocation per method, with the phone adapter disabled:

| Case | Result | Runner seconds |
|---|---|---:|
| Current requested observation reaches existing peripheral; CCCD still required | PASS | 3.150 |
| Unsolicited observation cannot supply peripheral capacity | PASS | 3.016 |
| Failed observation cannot supply peripheral capacity | PASS | 2.967 |
| Below-floor observation still refuses peripheral | PASS | 2.891 |
| Different address cannot supply peripheral capacity | PASS | 2.953 |
| Retired client cannot supply peripheral capacity | PASS | 2.890 |
| Retired peripheral cannot receive observation | PASS | 2.959 |
| Replaced server cannot receive old request observation | PASS | 2.877 |
| Second request cannot rebind old observation | PASS | 2.918 |
| Later peripheral cannot receive earlier observation | PASS | 2.889 |

Raw logs are `callback-B-<method>.log` under the current evidence directory. The 500 ms fixture registration wait allows Android's asynchronous client registration; the test requires actual request dispatch acceptance, not a fabricated dispatch result. The callback payload itself is deliberately injected. These results verify routing/lifecycle guards, not real two-device negotiation or reconnect reliability.

Existing tablet retirement (3 cases), refusal (1) and Bluetooth off/on recovery (1) regressions also pass **5/5**; logs are `existing-B-{retirement,refusal,bluetooth}.log`. Installed app and final test APK hashes were read back and matched to the candidate (`installed-tablet.json`).

The same app and final test package were subsequently installed on the phone without resetting its profile. All four installed package hashes match (`final-installed.json`). Both Bluetooth adapters were restored enabled and normal app activities reopened. At this first checkpoint the phone was locked; the following validation resumed after the user unlocked it.

## Phone callback regressions

With both devices verified unlocked, the tablet radio was temporarily disabled and the same ten callback cases passed **10/10** on the phone, each in a separate instrumentation invocation. Runner durations in table order above: 2.003, 1.806, 1.780, 2.248, 1.736, 1.727, 1.749, 1.801, 1.767 and 2.285 seconds. Existing retirement (3), refusal (1) and Bluetooth off/on recovery (1) cases also passed **5/5**, in 3.047, 1.703 and 2.519 seconds respectively. Logs are `callback-A-<method>.log` and `existing-A-{retirement,refusal,bluetooth}.log`. Both adapters were then enabled for paired trials. Total isolated results across both devices: **30/30**; these preserve the injected-callback limitations above.

## Paired physical validation

Before paired execution, `.work/mc025/2026-09-26-mtu/paired-plan.json` declared four diagnostic smoke runs, seven request/reply pairs each, with three short, three 280-byte and one background-responder pair. Each initiator deliberately stops its meshChat radio after pair index 2, remains stopped for five seconds, then restarts it. Recovery time starts immediately before `model.startRadio()` and ends when one peer is reported; it excludes the deliberate five-second outage and subsequent bounded catch-up settling. Startup time below runs from the test plan event to first reported peer and includes test/activity setup. All timing uses each device's own monotonic clock.

M01/M02 launch the phone/tablet initiator first respectively. M03/M04 launch the tablet/phone responder first respectively. The two instrumentation commands are dispatched consecutively without an artificial inter-device delay; command launch order is not a guarantee of OS callback order. Connection snapshots are enabled; detailed per-frame latency tracing is disabled. These are reconnect diagnostics, not repetitions of the earlier 60-pair-per-direction latency acceptance experiment.

| Run | Initiator / launched first | First peer, A / B (s) | Restart recovery (s) | Delivered pairs | Result / runner seconds A / B |
|---|---|---|---:|---:|---|
| M01 | Phone / phone | 6.903 / 6.204 | 61.087 phone | 7/7 | PASS, 109.054 / 108.171 |
| M02 | Tablet / tablet | Not observed / 23.631 transient | Not reached | 0/7 | Phone catch-up FAIL in 204.572; tablet explicitly interrupted |
| M03 | Phone / tablet | 2.894 / 2.872 | 63.276 phone | 7/7 | PASS, 106.631 / 106.624 |
| M04 | Tablet / phone | 3.567 / 3.677 | 99.629 tablet | 7/7 | PASS, 145.178 / 144.042 |

A always initiates; B responds. The physical devices swap roles in M02/M04. Total across the four declared plans: **28 scheduled pairs, 21 observed request/reply pairs (42 directed deliveries), seven unattempted pairs, three passing paired runs and one failed paired run**. No attempted pair timed out in these four runs. The failed setup is not excluded from the denominator. Each passing run includes actual background reception and radio-stop cleanup assertions on both endpoints. Existing profiles/history and native limits were preserved.

Full RTT samples in pair-index order (milliseconds; indices 0–2 short, 3–5 280-byte, 6 background):

| Run | RTTs |
|---|---|
| M01 | 510, 1195, 1180, 5442, 3422, 1008, 615 |
| M03 | 838, 981, 948, 5219, 3521, 962, 721 |
| M04 | 547, 1349, 1109, 5392, 3093, 976, 786 |

The first post-restart pairs remain slower; these small, sequential diagnostics do not demonstrate a new typical sub-second result or a statistically reliable success rate. Recovery includes the existing admission/retry/retired-address behavior and remains roughly one to two minutes in the passing samples, not a sub-second promise.

### Actual asymmetric callback path

M01's retained tablet Bluetooth log records a successful client `onConfigureMTU(..., 517, 0)` during initial setup and **no server MTU callback before the first connection teardown**. Its connection snapshots nevertheless show both roles at measured capacity 512, followed by successful messages. Thus the real-device trial exercises the missing-server-callback case, consistent with the one-shot forwarding repair; this is stronger than the injected fixture alone. Raw evidence: `M01/B-gatt-bounded.log`, `M01/B.log`, and the independently timed initiator messages in `M01/A.log`. This establishes that case on this device pair, not every Android/vendor callback ordering.

### Remaining startup failure

M02 is a different observed failure sequence. Both devices' retained native logs contain successful client MTU 517 and server MTU 517 callbacks. The tablet calls `cancelOpen`/`close` around 20:16:27.339–.345 local time, shortly after subscribing at 20:16:27.071, then cancels the peripheral. The phone initially reports two capacity-512 native-ready roles, later tears them down, and records its existing idle-peer block. The exact cause of the tablet's early close is **not yet established** by these snapshots; neither missing MTU nor a particular native close reason should be asserted from this evidence.

The phone's runner fails `Initial bounded catch-up did not settle` after 204.572 seconds and stops its radio in cleanup. With no requests sent and no remaining responder, the waiting tablet runner is explicitly force-stopped. Its `Process crashed` instrumentation message is the recorded operator interruption, not an independently observed spontaneous crash or an exhausted six-minute timeout. `M02/interruption.txt` records the reason. This run never reached the deliberate restart. Capture attempts using a UTC `-T` value initially returned empty files; `*-gatt-startup-localtime.log` contains the corrected, address-sanitized native callback evidence.

Next investigation should capture the close cause at the tablet's driver/native boundary during initial setup, including descriptor completion and first protocol-frame handling. Preserve MTU, CCCD, authentication, admission and idle-block requirements; the available evidence does not justify weakening any of them.

Raw per-run plans/logs, bounded native callback captures and `paired-summary.json` remain under `.work/mc025/2026-09-26-mtu/`. The aggregator confirms both endpoints' successful runner summaries and peer echo evidence for every claimed delivered pair. After the trials, all installed app/test hashes were reverified, both adapters were enabled, both normal app activities reopened, and no debugger forwards remained. No production/test source changed during these runs.

## Outstanding validation

Terra/medium independently reviewed published revision `d1814ea5e477522372db9e7a7bfaace912aca935` against `aba6a2e` with **no actionable correctness, security or test findings**. The reviewer checked one-shot target capture/consumption, exact live client identity, token/address/epoch guards, capacity and readiness invariants, and the raw tablet/build evidence. This review does not certify physical reliability or fulfill independent security assessments.

Phone callback and the four paired diagnostic trials are now complete as recorded above. The narrow MTU path works in M01; general startup reliability remains open because M02 fails, and successful recovery is still slow. Do not transfer the earlier ready-link latency acceptance result to this APK. USB-powered tests provide no battery-drain result. MC-025 stays in progress and PR #39 remains unmerged pending its broader physical/platform/security gates. The new physical evidence update requires its own independent review; production and test sources remain unchanged from reviewed revision `d1814ea`.
