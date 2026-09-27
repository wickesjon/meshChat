# MC-025 reconnect investigation — 2026-09-26

## Scope and baseline

Continue PR [#39](https://github.com/wickesjon/meshChat/pull/39) from `7d12daf` on the existing MC-025 branch. Galaxy S24 Ultra SM-S928U1 and Galaxy Tab S9 FE SM-X510, both Android 16/API 36, use their existing synthetic profiles over USB. This is a two-device diagnostic experiment, not the mixed-OEM, range, long-duration, battery or security certification gate. Both devices are externally powered; no battery-drain improvement is claimed.

The unchanged production Debug APK is `2bb9ab10c2fd9864e6c34ff622806fde38be10a39a249686a2578285e313953c`. Initial diagnostic test APK is `a09dd93feb739c49b73bf223373feb9b6f793948e131373e99a3c74f0cf47573`. `PhysicalRoundTripTest` retains the seven-pair short/280-byte/background workload. Optional `connectionTrace=true` samples bounded adapter state every five seconds without addresses, identities, keys or message content. Diagnostic timing is not comparable to the untraced [ready-link acceptance](MC-025-ready-link-results-2026-09-26.md).

| Unchanged-app attempt | Actual result |
|---|---|
| R01 | Both pass, 7/7 request/echo pairs; A 46.753 s, B 46.763 s |
| R02 | A fails, 5/7 in 106.018 s; first two requests were never sent while disconnected. B echoed five and was explicitly interrupted rather than waiting for requests that could not arrive |
| R03 | A fails initial catch-up settling in 184.907 s; B fails peer discovery in 362.052 s. No measurement pairs attempted |

All attempts are retained under `.work/mc025/2026-09-26-reconnect/`. R02's stop reason is recorded alongside its partial responder log. Sanitized native callback captures retain event ordering on each device; their separate clocks are not subtracted to infer one-way latency or exact cross-device ordering.

## Defect and repair under validation

The Android adapter replaced its whole GATT server whenever any admitted peripheral closed. Closing a duplicate or failed peripheral therefore changed services for other links. Native traces show service-change callbacks invalidating the reverse central link, followed by repeated server replacement and retired-address refusal. R01 also observed an unready extra peripheral expiring while the useful central survived. This explains one teardown cascade; it does not prove every discovery failure has the same cause.

The repair preserves the server when the closed peripheral owns no in-flight native notification and the existing six-address retirement set has room. It removes that token's mapping, subscription and unsent notification; retires its address until a new server epoch; and cancels the Android connection. Late MTU, subscription, connection and notification callbacks cannot attach to a replacement token at that address. Other peers retain their services and queue state. Active notifications and full retirement capacity still use whole-server replacement, since safe callback attribution or characteristic reuse cannot otherwise be guaranteed. The existing isolated-node retry remains at least 60 seconds after first retirement and requires zero driver connections.

Native admission budgets, proof-based duplicate consolidation, five-minute idle blocking, measured MTU, message pacing and protected storage are unchanged. There is no wire-format change. An additional test-harness fix waits for Bluetooth `STATE_OFF` before enabling it: `isEnabled == false` alone includes the intermediate shutdown state.

`GattPeripheralRetirementTest` exercises the real Android adapter and native admission using injected synthetic callback addresses. Its first baseline attempt failed the isolation precondition because a real peer still existed. After temporarily disabling the other endpoint's Bluetooth, the unchanged app fails the intended same-server assertion in 1.237 s. This is callback regression evidence, not physical exchange with those synthetic peers.

## Retry contention

The retirement-only candidate (`03354e84fce284ccfd8c82bc142904257fa8f0ac3a796408bd716e25c9598871`, test `8bb8a614aae1a8d4ea33908ae75eef2f64a85c137b08b41974dbc884739aa7b7`) passes F01, seven pairs on both endpoints, A 46.467 s/B 46.348 s. The useful central survives expiration of an extra unready peripheral without a server-epoch change. F02 adds a five-second stop/start on the initiator after three successful pairs while the responder stays running. It exposes continued reciprocal dial/refusal loops across subsequent server epochs. Thus the first repair alone is insufficient for reconnect reliability.

Both roles reserve the same native observed-address bucket: capacity three, refill one per 20 seconds, with no refund on cancellation. The five-second outgoing retry schedule can consume a newly available credit before the other endpoint's incoming connection is admitted. This source-level explanation is consistent with the observed reciprocal dial/refusal pattern; the native callback trace alone does not expose the exact admission-refusal reason.

The second change uses a 40–60-second independently randomized outgoing retry delay, allowing two address-credit refill periods and breaking reciprocal retry timing. First discovery and eligible incoming connections remain immediate. Existing deadlines are never shortened by an attempt/disconnect; the five-minute idle block, bounded retirement and native budgets remain authoritative. This deliberately trades retry speed for recovery opportunities. The host regression checks staggered opportunities, incoming eligibility during the outgoing wait, minimum/maximum delay bounds and preservation of a longer existing deadline.

## Validation status

The same-server baseline fails on the old production adapter and passes on the retirement-only fix. All three injected retirement cases pass on both devices: preserving a second peer, pending-notification epoch replacement, and full-retirement epoch replacement. The tablet suite takes 5.473 s. A combined phone suite initially exhausts native admission credits before the older refusal test's seventh reservation; that failure is retained. Running the existing refusal test separately passes in 1.189 s, without any production budget reset/bypass. Bluetooth off/on passes on both devices with `STATE_OFF` explicitly recorded before re-enabling.

App Debug/Release/test builds, lint, 26 app JVM tests and APK native/16-KiB checks pass for the retirement-only change. The standalone BLE check initially catches an old 146-byte frame assertion from before negotiated-capacity activation; correcting that fixture to the actual 182-byte negotiation, while asserting exactly two fragments and exact reassembly in both directions, yields 28/28 host tests, lint and both probe APK checks. Notification queue tests cover active-retirement refusal, unsent removal, no stale-callback advancement, preserved other-peer work and reentrant completion. The initial mistaken use of the transport-only APK checker on the full app rejects its additional expected SQLCipher/Compose libraries; the correct app-specific checker passes. Final backoff candidate validation is pending below.

### Combined candidate

Source checkpoint `1872d57ea281d9e03fac8e6868a2e5642a439322`:

- Debug APK `d32ccafe916f8efe9ba6456c4206569277bb3956cbeed5aee46c252df8c5c8e3`.
- Release APK `2c89d6c6e11d6cfe3b5f73f47984533e9311f2cf576c972d08723b42d0770e03`.
- Test APK `8bb8a614aae1a8d4ea33908ae75eef2f64a85c137b08b41974dbc884739aa7b7`.

Local checks use JDK 17.0.15+6, Gradle 8.13, AGP 8.11.1, Kotlin 2.2.0, API 36/build tools 35.0.0, existing native libraries and repository-local caches/temp directories. App `assembleDebug assembleRelease assembleDebugAndroidTest lintDebug testDebugUnitTest` passes in 2m10s (26 JVM tests); standalone BLE `lintDebug` passes in 1m17s (29 JVM tests, Debug/Release builds and native APK checks). App-specific `tests/integration/android-ui/check_apk.py` and `zipalign -c -P 16 4` pass for both app variants. Ticketboard/default and whitespace checks pass. Logs: `.work/mc025/reconnect-jitter-app.log`, `reconnect-jitter-ble.log`. No Rust, FFI, storage, crypto, dependency or iOS implementation changed in this delta; rerunning those unrelated suites would not resolve the measured Android radio failure. Their broader outstanding gates remain open.

| Combined-candidate trial | Actual result |
|---|---|
| J01 | Initial peer appears, then both links disappear. A fails the pre-interruption delivery guard in 106.668 s: first three requests were not ready and never sent. No intentional stop/start occurred; B was explicitly interrupted |
| J02 | Harness invocation error: existing smoke fixture rejects `initiator=B`. Both fail before starting the radio; not a connectivity result |
| J03 | Both pass 7/7, A 149.711 s/B 149.467 s. Physical roles exchanged: A tablet, B phone. Tablet recovers in 104.764 s after starting the radio again; all four subsequent pairs deliver, including background reception |
| J04 | Repeats J01's pre-interruption startup failure in 106.480 s; three requests not ready, none sent. No intentional stop/start occurred; B explicitly interrupted |
| J05 | Both pass 7/7, A 102.266 s/B 102.095 s. Phone initiator, tablet responder launched first. Phone recovers in 59.006 s after starting its radio again; all four subsequent pairs deliver, including background reception |

J01's sanitized callback history and snapshots show an additional unresolved setup issue. The tablet's central reports measured MTU 517 while its admitted peripheral remains at capacity zero. The phone's opposite central times out approximately ten seconds after setup; its local disconnect is followed by both roles dropping. Server epoch stays unchanged during this loss, so preserving the server does not fix this case. Android documents that [only the first MTU request on an ACL connection triggers negotiation on Android 14+](https://developer.android.com/reference/android/bluetooth/BluetoothGatt#requestMtu(int)); that is relevant platform context, not proof that it explains the missing server callback. We do not invent a peripheral MTU, reuse a measurement across an unproven generation, or waive the capacity floor.

The four valid combined-candidate paired attempts schedule 28 pairs: J03/J05 deliver 14 pairs (28 directed messages), while J01/J04 never transmit their 14 scheduled requests before aborting. J02 is an invocation error, not an executed measurement plan. Thus two controlled stop/start recoveries pass, but cold-start reliability is not accepted. Different launch order, retained history, random timing and the tiny uncontrolled sample prevent attribution of a population-level reliability gain solely to backoff. The measured recovery times are observations, not a promised bound.

Terra/medium independently reviewed `1872d57` against `7d12daf` and found no actionable implementation/security/test issues. Evidence-only follow-up `36fd8dee8268f43812d291edf22acaa14eb362fb` also has no actionable findings: the reviewer checked raw plans/logs, pair counts, recovery times, installed hashes and the final 14/14 regression total. This verifies the recorded evidence, not hardware certification or an independent security assessment. MC-025 and PR #39 remain unmerged; no release or independent security gate is closed by this investigation.

Final isolated regressions on the exact combined candidate pass **14/14**: three retirement scenarios, one existing refusal test and three Bluetooth off/on cycles per device. Phone retirement/refusal durations are 3.668/1.237 s; tablet 5.115/2.382 s. All six toggle runs record `STATE_OFF=10` before enabling and preserve the protected profile. Both adapters are restored enabled. These are isolated callback/lifecycle checks, not six paired Bluetooth-outage recoveries. The installed candidate remains on both devices; normal app screens are reopened after tests.

## Reproduction and interpretation

Use the existing synthetic profiles, both unlocked. Upgrade app and test APKs in place. For paired trials, invoke `org.meshchat.ui.PhysicalRoundTripTest` on each device with `physicalMeasure=true`, `connectionTrace=true`, `mode=smoke`, `initiator=A`, one shared unique `runId`, and distinct `role=A` / `role=B`. Add `restartAfterPair=true` for the deliberate five-second stop/start after pair index 2. The fixture accepts only A as smoke initiator; reverse the physical A/B assignment to interrupt the tablet. Keep the mapping and launch order in the run plan. Normal mapping is A phone/B tablet; J03 reverses it, while J05 retains the normal mapping and launches B first.

The initiator records the reconnected peer on its own monotonic clock from the new start request, excludes the intentional five-second offline interval, then requires catch-up status to settle and four further request/echo deliveries. A peer-count observation alone is not the delivery result. Warm-up round trips after reconnect are not the previously established ready-link latency benchmark: J03's first two post-reconnect round trips take 5.203 s and 3.494 s, then 1.009 s and 0.717 s. Cold-start peer observation is also not proof that setup remains stable; J01/J04 show why.

Run `GattPeripheralRetirementTest`, `GattServerRejectionTest`, and `BluetoothShutdownTest` as separate instrumentation invocations with `physicalConnect=true`; the Bluetooth case also requires `physicalBluetooth=true`. Stop the other app and temporarily disable its Bluetooth for the isolated callback fixtures, then restore both adapters. Separate invocations preserve the tests' defined fresh-process admission precondition; do not erase profiles or refund native counters inside a running test. Retain precondition failures and interrupted responders explicitly.
