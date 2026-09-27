# MC-025 shared Bluetooth role teardown — 2026-09-26

## Finding and scope

The preceding [empty-server recovery report](MC-025-empty-server-recovery-results-2026-09-26.md) retains V04's initial link loss: phone peripheral cancellation precedes a client status-19 disconnect. The following unchanged-app captures D01–D03 identify the actual close entry points. D01 closes an unused role through the idle policy; D02 closes duplicate roles through native effects during receive/write completion; D03 closes through notification completion/receive. Each leaves its surviving role until cleanup. No observer overflow or receive exception occurs. These captures do **not** reproduce V04's collateral drop or establish the exact Rust branch that selected each close.

Android exposes server [cancelConnection](https://developer.android.com/reference/android/bluetooth/BluetoothGattServer#cancelConnection(android.bluetooth.BluetoothDevice)) by Bluetooth device. This is a reason to avoid unnecessary cancellation while another local role holds that device; it is not proof that every cancellation drops a shared physical connection. The narrow repair defers that call after safe peripheral retirement or a bounded recorded refusal while a same-address client GATT object remains tracked, including setup. The last such client's teardown releases the deferred cancellation. Repeated retired CONNECTED callbacks cannot cancel the survivor or create another peripheral token.

The existing six-address retired set carries the deferral; no new unbounded collection is introduced. Unrelated clients and unrecorded addresses at full capacity retain immediate cancellation. Active-notification/full-retirement epoch fallback, native admission budgets, retry/idle deadlines, measured capacity, proof, callback identity and subscription requirements remain unchanged. Callback exceptions retain the existing guarded stop behavior; teardown still attempts cleanup. A Bluetooth address is only an unauthenticated transport association. No core, wire/FFI, storage, dependency or iOS changes.

## Baseline and regression method

The opt-in `GattSharedRoleTest` runs on the real Android adapter/core with the counterpart stopped and Bluetooth disabled. It creates a real native-reserved client for a synthetic address, injects server callbacks under the existing serialization gate, and observes actual framework cancellation API calls. Each method has its own process and normal native admission budget. Native time/credits are never changed or refunded. The test verifies a positive cancellation-log control, caps consumed process-filtered records at 256, and uses distinct synthetic address suffixes because Samsung redacts the prefix. It never emits raw addresses or message content. Logging unavailable or overflow fails the diagnostic rather than producing a false pass.

The first two harness attempts fail the positive control (3.273/3.009 seconds), before the target assertion. The first used a global log tail and full address, the second removed the global tail but still expected an unredacted address. Preserve both failures. With suffix matching corrected, the unchanged app fails the intended assertion in **3.503 seconds**: closing the peripheral makes **one cancellation call while the client is still tracked**, where the regression requires zero. This establishes API sequencing, not direct over-the-air causality for V04.

Baseline Debug app SHA-256: `e97dd82f24b70f4c5adf4d25945e4b1de8300ec4aa7c2f9a37a99fb13f4e1fbe`. Corrected baseline test APK: `d209ca104d83a1e4c6155e716dc0219f4ea496630522dae70c56312865648275`.

The four new methods cover safe retirement, newly refused inbound, unrelated-client immediate cancellation and full-table unrecorded immediate cancellation. Deferred cases also cover repeated CONNECTED/late MTU/notification callbacks, unchanged server/epoch, last-client cancellation and later cancellation without a client. Existing retirement/refusal/Bluetooth shutdown checks remain part of the candidate validation.

## Candidate validation and paired plan

Final Android Debug/Release/test APK builds and app lint pass; **26 app JVM tests** pass with zero failures/errors/skips. Standalone BLE Debug/Release builds, lint, **29 BLE JVM tests** and both probe APK native checks pass. App Debug/Release native-set/ELF and 16-KiB ZIP alignment checks pass. Both installed app/test pairs have been read back and match the candidate below. Both devices pass **28/28 isolated checks**: four new role methods, six retirement cases, one refusal case and three Bluetooth off/on repetitions per device. All six Bluetooth repetitions record full STATE_OFF before re-enable. Each method runs separately without changing native budgets/time inside the fixture. After the user unlocked the phone, its fourteen checks and all four declared paired trials completed on the unchanged reviewed candidate. Raw evidence is retained under `.work/mc025/2026-09-26-shared-role/`, including all D01–D03 captures and baseline harness/target failures.

The plan recorded before execution specifies W01 phone initiator/phone first, W02 tablet initiator/tablet first, W03 phone initiator/tablet first and W04 tablet initiator/phone first. Each plans seven request/reply pairs: three short foreground, five-second initiator radio stop/start, three 280-byte foreground, and one short background-responder pair. All 28 planned outcomes remain in the denominator, including readiness failures or explicit responder interruptions. Isolated checks run separately: four new role methods, six retirement methods, one refusal case and three Bluetooth shutdown repetitions per device, 28 total.

Both devices are Samsung API 36 (Galaxy S24 Ultra SM-S928U1 and SM-X510), USB-powered with existing synthetic profiles. These trials cannot establish battery drain, mixed-OEM/large-mesh reliability or independent security certification. MC-025 remains in progress and PR #39 unmerged.

## Paired results

**All four paired trials and all eight endpoint runners pass. All 28 scheduled request/reply pairs are observed (56 directed deliveries), with no failed or unattempted pair.** All 28 pre-send readiness checks pass, and every trial logs actual receipt while the responder activity is stopped. No run is interrupted or replaced. These are four small two-device trials, not general reliability certification or a causal proof that V04 can never recur.

| Run | Initiator / restarted device | First command | First peer A / B (s) | First actual request (s) | Recovery (s) | Runner A / B (s) | Result |
|---|---|---|---|---|---|---|---|
| W01 | Phone | Phone | 5.080 / 5.032 | 16.647 | 2.192 | 48.119 / 48.166 | Pass, 7/7 |
| W02 | Tablet | Tablet | 5.402 / 5.494 | 16.609 | 54.770 | 101.64 / 100.91 | Pass, 7/7 |
| W03 | Phone | Tablet | 3.920 / 3.780 | 15.325 | 2.746 | 47.769 / 47.148 | Pass, 7/7 |
| W04 | Tablet | Phone | 4.999 / 5.112 | 16.351 | 50.346 | 96.386 / 96.327 | Pass, 7/7 |

A is always the message initiator, B the responder; physical devices swap as shown. First peer and first request are measured from that endpoint's own plan log. First peer does not imply settled messaging readiness. Recovery uses the restarting endpoint's monotonic time from radio start to first observed peer, excluding the deliberate five-second outage and subsequent catch-up settling.

Phone restart recovery is 2.192/2.746 seconds; tablet restart recovery remains 54.770/50.346 seconds. The previous candidate's initial readiness failure does not recur in this batch, but the longer recovery path is still present. This small uncontrolled comparison establishes neither a reliable speedup nor a recovery guarantee. The earlier V04 failure remains in the preceding report.

W03's phone snapshots show one retired address alongside the ready central role from plan +5.233 seconds. Its framework log records no server cancellation until the deliberate stop: client cancelOpen at 1790487832.334 precedes server cancelConnection at 1790487832.340. This supports the intended lifetime sequencing on physical hardware, in addition to the controlled API regression. W02's phone later cancels an incoming connection to its retired address while it has no surviving client, then refreshes the server at the existing delayed deadline. The new client-hold exception does not bypass this empty-node retirement/retry path. Those logs motivate further investigation of slow recovery; no guard or timer was relaxed to make these trials pass.

Full RTTs in pair order, milliseconds:

| Run | Short foreground before restart | 280-byte foreground after restart | Short background responder |
|---|---|---|---|
| W01 | 657 / 993 / 1120 | 5460 / 3350 / 1084 | 613 |
| W02 | 661 / 1018 / 1124 | 5426 / 3268 / 1019 | 621 |
| W03 | 639 / 990 / 1159 | 5227 / 3357 / 1063 | 735 |
| W04 | 714 / 1060 / 1179 | 5336 / 3364 / 1182 | 547 |

An observed pair requires accepted outgoing request and received reply on the initiator plus matching request receipt and native echo completion on the responder. RTT uses only the initiator clock, from submission to observed reply, including 100 ms polling/scheduling overhead. The pre-send public-channel cooldown is outside that timer. Connection snapshots are enabled; detailed latency tracing is disabled. Post-restart 3–5-second RTT samples remain included. This workload does not rerun the earlier 30-short/30-long ready-link median acceptance, and its sub-second claim is not transferred to this APK.

Both endpoint logs, predeclared plans, bounded sanitized framework callbacks, `paired-summary.json`, `paired-detail.json`, all 28 isolated logs and final package readbacks remain under the evidence root. Battery counters are USB-powered and cannot measure battery drain.

## Candidate identity and local checks

| Artifact | SHA-256 |
|---|---|
| Debug app | `7a9e37e29bcb531f2b9f23bde25029a77dc2f9e1d9a2ae6642be73fc15a0e178` |
| Unsigned Release | `5855a3b7218115c41b0e1c806fd68630fd9c2a59671b2da82081b9bce68b3e34` |
| Instrumentation APK | `d209ca104d83a1e4c6155e716dc0219f4ea496630522dae70c56312865648275` |

```text
src/android/gradlew.bat -p src/android --offline --no-daemon --max-workers=2 :app:assembleDebug :app:assembleRelease :app:assembleDebugAndroidTest :app:lintDebug :app:testDebugUnitTest
src/android/gradlew.bat -p src/android/ble --offline --no-daemon --max-workers=2 lintDebug
python -B tests/integration/android-ui/check_apk.py <Debug APK> <Release APK>
zipalign.exe -c -P 16 4 <each app APK>
. .work/mc025/isolated-role-regressions.ps1 -Roles B
. .work/mc025/isolated-role-regressions.ps1 -Roles A
python -B .work/mc025/run-role-pair.py W01 --mode smoke --restart
python -B .work/mc025/run-role-pair.py W02 --mode smoke --restart --swap-devices
python -B .work/mc025/run-role-pair.py W03 --mode smoke --restart --responder-first
python -B .work/mc025/run-role-pair.py W04 --mode smoke --restart --swap-devices --responder-first
```

Toolchain remains JDK 17.0.15+6, Gradle 8.13, AGP 8.11.1, Kotlin 2.2.0, SDK 36/build tools 35.0.0, Rust 1.85.1 and Python 3.14.4; caches/temp/outputs stay within the repository. Normal/security native libraries and shared Rust sources are unchanged; their preceding native validation remains applicable with Android consumption/vector checks here. No fresh iOS/shared-core build is required for this Kotlin-only callback delta. Mac/native iOS and independent security gates remain open. Terra/medium independently reviewed source checkpoint `01e7a9f525b0b305638ccba3e550eb7359f13a90` against `4a7875f`, with **no actionable findings**. The review confirmed bounded retirement/client deferral and final release, unchanged immediate/epoch fallbacks, native admission and callback safeguards, preserved exception behavior, meaningful baseline/candidate fixtures and the raw tablet/build/hash evidence. Phone/paired evidence was pending at that source checkpoint. Terra/medium independently reviewed physical-evidence checkpoint `e3751a665e33f5abbfad525d606fb0ea6bd86129` against `70ae1e9`, with **no actionable findings**. Raw logs confirm 8/8 passing runners, 28/28 pairs, all readiness/background checks, six full Bluetooth-off cycles, every timing in the tables, cancellation chronology and matching installed hashes. This evidence review preserves the earlier V04 failure, remaining slow recovery and all broader hardware/Mac/security gates. Neither review is hardware or independent security certification.

Final default ticketboard validation, all 12 board tests, 58 relative file references and `git diff --check` pass. Both final app/test pairs match the candidate hashes, adapters are restored enabled, normal apps reopened and debugger forwarding is empty. Both devices completed their declared checks and the normal apps were reopened afterward. No production or test source changed after the reviewed checkpoint.
