# MC-025 empty-server recovery — 2026-09-26

## Finding and scoped repair

The preceding [setup-order candidate](MC-025-setup-order-results-2026-09-26.md) passes four paired trials but takes 60.960–124.690 seconds to recover. Its K02 callback traces show retirement surviving the last peer's disconnect, local outgoing attempts colliding with that retirement, and new retry/retirement cycles. Timed retry is necessary for refused connections; treating a confirmed last-peer disconnect the same way can needlessly strand an otherwise empty server.

The final repair in `AndroidGattRadio` refreshes the server after a successful platform `STATE_DISCONNECTED` callback removes a still-tracked peripheral, only when the radio is running, the original callback epoch is still current, and the driver has no remaining central/peripheral setup or established connections. Refresh replaces the server epoch; it never clears retired addresses in place. If teardown already replaced the epoch, it does not replace it again. Refused, unmatched, old-epoch and error callbacks retain the existing path. Native admission credits, outgoing 40–60-second retry, five-minute idle blacklist, measured MTU, subscription, proof and other admission requirements are unchanged. No shared Rust, wire/FFI, storage, dependency or iOS implementation changes.

## Withdrawn first experiment

An initial candidate deferred outgoing selection of retired addresses while isolated. Its injected regression fails before that change (2.896 s) and passes afterward (2.874 s), but physical Q01 demonstrates that this local deferral is insufficient: both devices can still miss the other's refresh window. The last phone snapshot has no peer 207.228 seconds after `restart_begin` (this interval includes the deliberate five-second stop). Both runners were explicitly force-stopped; neither passed or naturally reached its 360-second reconnect deadline.

Q01 delivers its first three pairs; its remaining four are unattempted. The other three declared runs Q02–Q04 are withdrawn unrun after that regression. Preserve the entire plan denominator: **28 planned pairs, three observed, 25 unattempted; zero passing paired runs**. This experiment is not included among the final candidate's results. Its source changes to candidate selection and policy tests were withdrawn; the final production delta is confined to confirmed-disconnect handling.

Raw plan, failure-aware summary, source patch, logs and interruption record remain under `.work/mc025/2026-09-26-retry-sequence/`. The first `*-gatt-bounded.log` capture ended at the last ordinary event, before the long wait; the supplemental `*-gatt-interrupted.log` uses the final connection snapshot's monotonic time mapped to that device's logged UTC offset and includes the wait. Neither capture logs message bytes or Bluetooth addresses.

Experiment APK identities:

| Artifact | SHA-256 |
|---|---|
| Debug app | `07283a9774c3d125eb42a455e1941143f687df1383124cd69fe28fdbed565c68` |
| Unsigned Release | `7fd028b7b8eab1c6cd07e954d38f75201baf5af1ab25a2e01c38474efb500040` |
| Test APK | `9a01940dde5ba6137060acc8acee3f69aadfd03cb2ce9767cc0e4bb0467e436b` |

The first experiment's baseline production app is the previous `f06adf5b46601cae15c374b88d316bd7cece1fca20c04a60fa987c0b9f4ecc84`. The confirmed-disconnect regression then fails on experimental app `07283a…` in 2.796 seconds at the expected fresh-epoch assertion, using test APK `a332c68272392200b5639e9c863cc461875266f3601ecdd6eb8cdcaa70f062bf`. That regression exercises the unchanged disconnect path; it does not depend on the experimental outgoing filter.

## Final candidate and validation

The final opt-in callback tests cover preserving another peripheral's service, active-notification/full-retirement epoch fallback, confirmed last-peer disconnect, error/unmatched disconnect, surviving central setup and old-epoch callbacks. The existing refusal test retains its delayed empty-server retry and native accounting assertions.

The first expanded six-method phone invocation passes two methods, then four fail during fixture admission (`expected 2` peers, observed one or zero), before their production assertions. The methods share the process's six-attempt node bucket, and the central-setup case reserves an extra attempt. This is an invalid grouped fixture schedule, not four demonstrated production regressions. Preserve `candidate-retirement-A.log`: six attempted checks, two pass/four fail, and the remaining fourteen planned checks not run by that aborted script. Do not reset native budgets or advance native time to make that batch pass.

Each retirement method must instead run in a separate instrumentation process using `-e class org.meshchat.ui.GattPeripheralRetirementTest#<method> -e physicalConnect true`. The six methods are `idlePeripheralCloseKeepsServerAndRetiresOldCallbacks`, `activeNotificationRequiresFreshServerEpoch`, `fullRetirementSetRequiresFreshServerEpoch`, `lastPeripheralDisconnectRefreshesEmptyServer`, `errorDisconnectKeepsRetirementDeadline` and `peripheralDisconnectPreservesCentralSetup`. Refusal and each Bluetooth repetition also use separate invocations; Bluetooth requires `physicalBluetooth=true`. The counterpart is stopped with Bluetooth disabled during isolated fixtures, then restored. These are synthetic callback fixtures using the real adapter/core; they do not certify continuous-session churn or over-the-air fault handling.

Final Debug/Release/androidTest builds and app lint pass; **26 app JVM tests** pass. Standalone BLE Debug/Release builds, lint, **29 BLE JVM tests** and both BLE APK native checks pass. App Debug/Release native-set/ELF and 16-KiB ZIP alignment checks pass. Both installed app/test pairs match the following candidate:

| Artifact | SHA-256 |
|---|---|
| Debug app | `e97dd82f24b70f4c5adf4d25945e4b1de8300ec4aa7c2f9a37a99fb13f4e1fbe` |
| Unsigned Release | `ac8ae5abbe3505ad06f83cffe2d2bbf87ea8e2ed0b7a36c45c40f7ecb0eba21d` |
| Instrumentation APK | `49f2dfe8b1a1ad5436a7d24d574b689f5de737c130b9f58fd2dcae1f166af566` |

Raw final-candidate evidence is under `.work/mc025/2026-09-26-empty-server/`. The corrected isolated schedule passes **20/20 checks**: six retirement/disconnect cases, one refusal case and three Bluetooth repetitions per device. Every Bluetooth repetition logs full `STATE_OFF` before re-enable. The four declared paired trials V01–V04 are in progress at this checkpoint. V01/V03 restart the phone and V02/V04 the tablet; each orientation runs with both first-command launch orders. Each plans seven request/reply pairs, including three short foreground pairs, a five-second initiator stop/start, three 280-byte pairs and a final short background-responder pair. Preserve all planned attempts and both endpoint summaries.

## Commands, applicability and limits

```text
src/android/gradlew.bat -p src/android --offline --no-daemon --max-workers=2 :app:assembleDebug :app:assembleRelease :app:assembleDebugAndroidTest :app:lintDebug :app:testDebugUnitTest
src/android/gradlew.bat -p src/android/ble --offline --no-daemon --max-workers=2 lintDebug
python -B tests/integration/android-ui/check_apk.py <Debug APK> <Release APK>
zipalign.exe -c -P 16 4 <each app APK>
. .work/mc025/isolated-empty-regressions.ps1
python -B .work/mc025/run-empty-pair.py V01 --mode smoke --restart
python -B .work/mc025/run-empty-pair.py V02 --mode smoke --restart --swap-devices
python -B .work/mc025/run-empty-pair.py V03 --mode smoke --restart --responder-first
python -B .work/mc025/run-empty-pair.py V04 --mode smoke --restart --swap-devices --responder-first
```

Toolchain remains JDK 17.0.15+6, Gradle 8.13, AGP 8.11.1, Kotlin 2.2.0, SDK 36/build tools 35.0.0, Rust 1.85.1 and Python 3.14.4, with cache/temp/output confined to the repository. The normal/security native libraries and shared Rust sources are unchanged from the preceding fully rebuilt and tested candidate; Android consumption/native-vector checks remain applicable. This narrow Android callback delta does not require a fresh iOS or shared-core build; previously outstanding Mac/native iOS validation and independent security assessments remain open.

Devices are Galaxy S24 Ultra SM-S928U1 and SM-X510, both API 36, with existing synthetic profiles and USB power. No battery-drain claim is possible from these trials. Small two-endpoint timing samples cannot establish mixed-OEM/large-mesh reliability, power acceptance or recovery guarantees. Independent review is pending; MC-025 stays in progress and PR #39 unmerged.
