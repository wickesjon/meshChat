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

Final Android Debug/Release/test APK builds and app lint pass; **26 app JVM tests** pass with zero failures/errors/skips. Standalone BLE Debug/Release builds, lint, **29 BLE JVM tests** and both probe APK native checks pass. App Debug/Release native-set/ELF and 16-KiB ZIP alignment checks pass. Both installed app/test pairs have been read back and match the candidate below. The tablet passes **14/14 isolated checks**: all four new role methods, six retirement cases, one refusal case and three Bluetooth off/on repetitions. Phone checks await its unlock; the phone still reports its lock screen, so no phone fixture is started and W01–W04 remain unrun. Thus **zero of 28 planned paired outcomes have been observed on this candidate**; no paired recovery/latency improvement is claimed. Raw evidence is retained under `.work/mc025/2026-09-26-shared-role/`, including all D01–D03 captures and baseline harness/target failures.

Before execution, declare W01 phone initiator/phone first, W02 tablet initiator/tablet first, W03 phone initiator/tablet first and W04 tablet initiator/phone first. Each plans seven request/reply pairs: three short foreground, five-second initiator radio stop/start, three 280-byte foreground, and one short background-responder pair. All 28 planned outcomes remain in the denominator, including readiness failures or explicit responder interruptions. Isolated checks run separately: four new role methods, six retirement methods, one refusal case and three Bluetooth shutdown repetitions per device, 28 total.

Both devices are Samsung API 36 (Galaxy S24 Ultra SM-S928U1 and SM-X510), USB-powered with existing synthetic profiles. These trials cannot establish battery drain, mixed-OEM/large-mesh reliability or independent security certification. MC-025 remains in progress and PR #39 unmerged.

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
```

Toolchain remains JDK 17.0.15+6, Gradle 8.13, AGP 8.11.1, Kotlin 2.2.0, SDK 36/build tools 35.0.0, Rust 1.85.1 and Python 3.14.4; caches/temp/outputs stay within the repository. Normal/security native libraries and shared Rust sources are unchanged; their preceding native validation remains applicable with Android consumption/vector checks here. No fresh iOS/shared-core build is required for this Kotlin-only callback delta. Mac/native iOS and independent security gates remain open. Terra/medium independently reviewed source checkpoint `01e7a9f525b0b305638ccba3e550eb7359f13a90` against `4a7875f`, with **no actionable findings**. The review confirmed bounded retirement/client deferral and final release, unchanged immediate/epoch fallbacks, native admission and callback safeguards, preserved exception behavior, meaningful baseline/candidate fixtures and the raw tablet/build/hash evidence. Phone/paired evidence remains pending; this review is not hardware or independent security certification.

Final default ticketboard validation, all 12 board tests, 58 relative file references and `git diff --check` pass. Both final app/test pairs match the candidate hashes, adapters are restored enabled, normal apps reopened and debugger forwarding is empty. The phone remains locked; no phone test or paired trial is claimed. No production or test source changed after the reviewed checkpoint.
