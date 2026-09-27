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

Raw final-candidate evidence is under `.work/mc025/2026-09-26-empty-server/`. The corrected isolated schedule passes **20/20 checks**: six retirement/disconnect cases, one refusal case and three Bluetooth repetitions per device. Every Bluetooth repetition logs full `STATE_OFF` before re-enable. The four declared paired trials V01–V04 are complete, including the explicit responder interruption below. V01/V03 restart the phone and V02/V04 the tablet; each orientation runs with both first-command launch orders. Each plans seven request/reply pairs, including three short foreground pairs, a five-second initiator stop/start, three 280-byte pairs and a final short background-responder pair.

## Paired outcomes and timings

**Three paired trials pass; V04 fails. Across all 28 scheduled pairs, 27 are observed (54 directed deliveries), and one fails its readiness check before submission.** The missing V04 request is never sent; do not classify it as a delivered message, a successful 5 ms RTT, or an over-the-air loss. Six endpoint runners pass, the V04 initiator fails its 7/7 assertion in 101.025 seconds, and its waiting responder is explicitly stopped after the initiator ends. The responder cannot complete the missing index-0 request. All four runs contain a real stopped/background-responder receive for their final pair.

| Run | Initiator / restarted device | First command | First peer A / B (s) | First actual request from A (s) | Recovery (s) | Runner A / B (s) | Result |
|---|---|---|---|---|---|---|---|
| V01 | Phone | Phone | 3.507 / 3.269 | 14.671 | 3.525 | 47.512 / 46.877 | Pass, 7/7 |
| V02 | Tablet | Tablet | 4.804 / 4.880 | 16.108 | 53.371 | 99.200 / 98.988 | Pass, 7/7 |
| V03 | Phone | Tablet | 3.459 / 3.321 | 14.772 | 2.084 | 46.384 / 46.355 | Pass, 7/7 |
| V04 | Tablet | Phone | 3.050 / 3.158 | 62.799 | 2.782 | 101.025 failure / interrupted | Fail, 6/7 |

A is the message initiator and B the responder. First-peer and first-request intervals start at that endpoint's own plan log; first peer does not mean stable messaging readiness. V04 initially sees a peer, loses it, and exhausts the 30-second readiness wait for index 0 at plan +42.415 seconds. It later regains a link and sends index 1 at +62.799 seconds. Its eventual fast intentional-restart result must not hide this earlier startup failure.

Recovery measures the restarting endpoint's own monotonic interval from radio start to first observed peer; it excludes the deliberate five-second outage and later catch-up settling. The three 2.084–3.525-second observations show this narrower path can recover much sooner than the previous 60.960–124.690-second series. V02 still takes 53.371 seconds. This is a small uncontrolled diagnostic comparison, not a statistically established improvement or guarantee; the failed V04 remains included.

V01's responder callback log records a successful server disconnect, fresh server registration 32 ms later, then later admission of the restarted peer. That supports actual execution of the new fast path. V04's initial failure is preceded on the phone by a peripheral cancellation, followed by a client disconnect with status 19; the tablet then receives its successful disconnect and refreshes its empty server. The new refresh therefore follows the observed initial link loss, rather than demonstrating its cause. The origin of that duplicate-role/early close remains an open investigation; these bounded native logs do not establish the exact driver/core close decision. V02 also retains the slower retired-address/retry path. No retry timer or refusal protection is shortened to hide either case.

Full measured RTTs in pair order, milliseconds:

| Run | Short foreground before restart | 280-byte foreground after restart | Short background responder |
|---|---|---|---|
| V01 | 510 / 1097 / 1167 | 5220 / 3350 / 925 | 522 |
| V02 | 672 / 1037 / 1024 | 5490 / 3014 / 1118 | 930 |
| V03 | 831 / 1191 / 1087 | 5420 / 3283 / 926 | 843 |
| V04 | Not sent / 5527 / 2985 | 5417 / 3098 / 1093 | 702 |

Each observed pair requires the initiator's accepted outgoing request and received reply plus the responder's matching incoming request and completed native echo. RTT uses only the initiating device's clock, from request submission through reply observation, with 100 ms polling and scheduling overhead. The pre-send public-channel cooldown is outside that timer. Connection snapshots are enabled; detailed latency tracing is disabled. The long post-restart samples remain included; this does not rerun the earlier ready-link 30-short/30-long median acceptance workload or transfer its sub-second claim to this APK.

Plans, both endpoint logs, bounded sanitized callback traces, `paired-summary.json` and V04's explicit `interruption.txt` remain in the evidence root. The normal apps are reopened afterward, both adapters enabled, debugger forwarding empty and all four installed app/test hashes read back matching the candidate table. Existing profiles remain intact. No source changes follow the reviewed source checkpoint.

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

Terra/medium independently reviewed source checkpoint `36e6ce99953875f171be0283a73fca9e16f92c4c` against `245cccc`, with **no actionable findings**. Review confirmed the successful/current-epoch/empty-driver trigger, re-entrant teardown protection, preserved refusal/error/surviving-link paths, unchanged budgets/security gates, meaningful regressions and accurate experimental failure records.

Terra/medium independently reviewed final physical-evidence checkpoint `e21e949d2b1bf2ef79325ae6884daf12d3920475` against that source revision, also with **no actionable findings**. Raw plans/logs confirm the 27/28 denominator, pre-submission failure, explicit responder interruption, all background events, timing definitions, callback chronology and candidate/readback hashes. Both reviews preserve the withdrawn experiment, invalid grouped fixture, slow V02 and failed V04; neither is hardware or independent security certification. Final default ticketboard validation, all 12 board tests, 56 relative file references and `git diff --check` pass. This metadata update changes no production/test source.

Devices are Galaxy S24 Ultra SM-S928U1 and SM-X510, both API 36, with existing synthetic profiles and USB power. No battery-drain claim is possible from these trials. Small two-endpoint timing samples cannot establish mixed-OEM/large-mesh reliability, power acceptance or recovery guarantees. MC-025 stays in progress and PR #39 unmerged.
