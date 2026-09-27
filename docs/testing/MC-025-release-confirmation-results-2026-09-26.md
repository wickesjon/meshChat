# MC-025 deferred-release confirmation — 2026-09-26

## Finding and bounded repair

The [shared-role candidate](MC-025-shared-role-results-2026-09-26.md) delivers all 28 paired exchanges, but tablet restarts recover in 54.770/50.346 seconds. Its W02 phone framework log shows `onServiceChanged` at 1790487717.167, client cancellation/close at .226–.228, deferred server cancellation at .230, and successful server DISCONNECTED at 1790487718.279. The peripheral is already retired, so the successful confirmation is unmatched and does not refresh the empty server. A later incoming connection is cancelled, then the existing delayed epoch refresh and outgoing retry permit recovery. W02/W04 are retained as the unchanged-candidate physical baseline; no new baseline paired trial is substituted for them.

The repair retains one nullable release address in the current server epoch, only after closing the final client and successfully issuing its deferred same-address server cancellation while the driver is empty. The matching successful server DISCONNECTED callback may refresh that still-empty epoch once. Matching error/disconnect consumes the target; any new native-admitted client attempt or server CONNECTED callback invalidates it; reset clears it. An unrelated callback cannot consume or use the target. If the confirmation never arrives, existing delayed cleanup remains available.

Refresh still replaces the entire epoch. There is no retirement clearing in place, retry/idle deadline shortening, native credit reset/refund, identity assertion or permission to admit payloads. Pure refusals, arbitrary unmatched/old/error callbacks, surviving links/setups and intervening connection activity do not acquire immediate refresh. The target is a bounded teardown correlation, not an authenticated identity or a claim to identify a physical ACL generation. No Rust, native library, wire/FFI, storage, dependency or iOS source changes.

## Baseline and callback fixtures

The unchanged app fails `GattReleaseConfirmationTest#lastClientReleaseRefreshesAfterConfirmedDisconnect` at the expected fresh-epoch assertion in **2.608 seconds**. The client/peripheral teardown and successful server disconnect have occurred, but the epoch remains unchanged. Baseline app SHA-256 is `7a9e37e29bcb531f2b9f23bde25029a77dc2f9e1d9a2ae6642be73fc15a0e178`; test APK is `cab2a5411cb7abaf73f5100eecc0184cc8e46d1b8fc1db8e61f31559bcc2b300`. Build and failure logs are retained under `.work/mc025/2026-09-26-release-confirmation/`.

Seven opt-in methods cover final release confirmation, matching error consumption, unrelated disconnect, intervening client attempt, retired reconnect, surviving setup and old server epoch. Each runs separately with the other endpoint stopped/Bluetooth disabled. Synthetic callbacks use the real adapter and production core; no native time/credits are changed, and no profile/history/permissions are reset. The callback monitor prevents asynchronous fixture interference. Tests assert unchanged server before confirmation, required refresh or conservative retention, no new token from late callbacks and one-use consumption. These fixtures are not over-the-air fault or continuous-session admission stress tests.

The existing four shared-role, six retirement, one refusal and three Bluetooth shutdown cases also run separately per device, for **42 isolated checks** in total. Both devices now pass all 21 checks each (42/42); all six Bluetooth repetitions record full STATE_OFF before re-enable.

## Paired plan and validation status

The plan recorded before execution specifies Y01 phone initiator/phone first, Y02 tablet initiator/tablet first, Y03 phone initiator/tablet first, Y04 tablet initiator/phone first. Each plans three short foreground pairs, a five-second initiator stop/start, three 280-byte foreground pairs and one short pair with the responder activity stopped. Retain all **28 planned pairs**, including any failed/unattempted outcomes, missing readiness and explicit interruptions. All four declared trials now pass, as detailed below.

Android Debug/Release/test builds, app lint and **26 app JVM tests** pass; standalone BLE builds/lint, **29 BLE JVM tests** and both probe APK checks pass. App Debug/Release native-set/ELF and 16-KiB ZIP alignment checks pass. The final strengthened old-epoch test APK is rebuilt and linted. Both installed app/test pairs match the candidate hashes below. All 42 isolated checks and four declared paired trials pass. Terra/medium independently reviewed physical-evidence checkpoint `bab2a29f7637e30c74e3aa666e6305bb59d50b77` against `f6cc5a10e3e5855729a7c7cbf86b5ebaa632c811`, with **no actionable findings**. Raw logs and manifests confirm all 42 isolated checks, six full Bluetooth-off cycles, 8/8 runners, 28/28 pairs, readiness/background receipts, timing/RTT tables, Y02/Y04 callback chronology and installed candidate identities. This limited evidence review does not replace broader hardware or independent security certification. Terra/medium independently reviewed source checkpoint `f6cc5a10e3e5855729a7c7cbf86b5ebaa632c811` against `8127e86`, with **no actionable findings**. An initial exception-flow concern was withdrawn after verifying that cancellation and target assignment share one cleanup lambda, so an exception skips assignment; no source change was needed. This is source review, not physical or independent security certification. Raw evidence remains under the new root. Existing broader hardware/energy/mixed-OEM/Mac/iOS/independent-security gates remain open, MC-025 stays in progress and PR #39 unmerged. Both Samsung API 36 devices use existing synthetic profiles and USB power; these trials cannot establish battery drain.

## Paired outcomes and callback evidence

**All four trials and eight endpoint runners pass. All 28 planned request/reply pairs are observed (56 directed deliveries), with no missing readiness, failed/unattempted pair or interrupted/replaced run.** All four final requests are actually received while the responder activity is stopped.

| Run | Initiator / restarted device | First command | First peer A / B (s) | First actual request (s) | Recovery (s) | Runner A / B (s) | Result |
|---|---|---|---|---|---|---|---|
| Y01 | Phone | Phone | 3.762 / 3.661 | 14.844 | 2.346 | 46.57 / 46.463 | Pass, 7/7 |
| Y02 | Tablet | Tablet | 3.505 / 3.628 | 14.802 | 4.496 | 49.295 / 48.951 | Pass, 7/7 |
| Y03 | Phone | Tablet | 4.301 / 4.322 | 15.609 | 2.341 | 47.255 / 47.237 | Pass, 7/7 |
| Y04 | Tablet | Phone | 3.204 / 3.225 | 14.424 | 11.675 | 55.776 / 55.631 | Pass, 7/7 |

Phone restart recovery is 2.346/2.341 seconds; tablet restart recovery is 4.496/11.675 seconds, compared with 54.770/50.346 seconds on the previous candidate. The slow tablet path does not recur in this batch. This is a small uncontrolled two-device comparison, not a general recovery guarantee or mixed-OEM certification. Earlier failures remain in their original reports.

A is the message initiator, B the responder; the physical devices swap as shown. First-peer and first-request intervals start at that endpoint's own plan record. First peer is not stable messaging readiness. Recovery uses the initiating/restarting device's monotonic clock from radio start to first observed peer, excluding the deliberate five-second outage and later catch-up settling.

Y02 provides evidence of the repaired path itself. Phone snapshots show a retired address with only a ready central role before the stop. The phone's service-change callback is at 1790489000.040, client cancelOpen at .043 and deferred server cancelConnection at .053. Successful server DISCONNECTED arrives at 1790489001.145; unregisterCallback follows at .150 and fresh registerCallback at .151, about **6 ms after confirmation**. This is a previously retired peripheral address, so the new final-client confirmation path permits that refresh. Subsequent incoming setup completes without waiting for the old minute-long retirement deadline. The baseline W02 chronology remains recorded above.

Y04 retains a slower 11.675-second recovery despite prompt server refresh. Its phone uses the existing still-tracked-peripheral confirmation path and starts fresh registration about 9 ms after disconnect. On the tablet, `connect()` is logged at 1790489125.419 and the successful client connection callback at 1790489135.106, a **9.687-second platform connection interval**. This locates most of the remaining delay after the connection request; it does not establish why the Bluetooth stack took that long or authorize changing setup/admission limits.

Full measured RTTs in pair order, milliseconds:

| Run | Short foreground before restart | 280-byte foreground after restart | Short background responder |
|---|---|---|---|
| Y01 | 728 / 1171 / 967 | 5397 / 3416 / 1048 | 510 |
| Y02 | 726 / 1053 / 1072 | 5720 / 3090 / 1019 | 758 |
| Y03 | 712 / 1066 / 953 | 5190 / 3541 / 1093 | 720 |
| Y04 | 953 / 888 / 989 | 5329 / 3395 / 1045 | 702 |

Each observed pair requires the initiator's accepted request/received reply and the responder's matching received request/completed native echo. RTT uses only the initiating clock from submission to observed reply; the pre-send public-channel cooldown is outside it, while 100 ms polling and scheduling overhead remain included. Connection snapshots are enabled and detailed latency tracing is disabled. The long early post-restart samples remain included. This workload does not repeat the earlier 30-short/30-long ready-link median acceptance or transfer its sub-second claim to this APK.

Plans, both endpoint logs, bounded sanitized Bluetooth callbacks, `paired-summary.json`, `paired-detail.json`, `isolated-summary.json` and final package readbacks remain in the evidence root. Both installed app/test pairs match the candidate table. Normal apps are reopened, adapters enabled and debugger forwarding empty. No production/test source changed after the reviewed checkpoint. USB-powered counters do not establish battery drain.

## Candidate identity and commands

| Artifact | SHA-256 |
|---|---|
| Debug app | `5e7f5139fa38a96ed6d71af83d6faf29bb66c2344cb8ce52c442d7c34bb3a8aa` |
| Unsigned Release | `e901028d88dd5e5b37e9c85c461d92bed39e3de5ad81d13323c79069d2794340` |
| Instrumentation APK | `e00da10d96879e21c5d24cc6d339f58e4929dc5134eca2c6fec3f733995a515b` |

```text
src/android/gradlew.bat -p src/android --offline --no-daemon --max-workers=2 :app:assembleDebug :app:assembleRelease :app:assembleDebugAndroidTest :app:lintDebug :app:testDebugUnitTest
src/android/gradlew.bat -p src/android/ble --offline --no-daemon --max-workers=2 lintDebug
src/android/gradlew.bat -p src/android --offline --no-daemon --max-workers=2 :app:assembleDebugAndroidTest :app:lintDebug
python -B tests/integration/android-ui/check_apk.py <Debug APK> <Release APK>
zipalign.exe -c -P 16 4 <each app APK>
. .work/mc025/isolated-release-regressions.ps1
python -B .work/mc025/run-release-pair.py Y01 --mode smoke --restart
python -B .work/mc025/run-release-pair.py Y02 --mode smoke --restart --swap-devices
python -B .work/mc025/run-release-pair.py Y03 --mode smoke --restart --responder-first
python -B .work/mc025/run-release-pair.py Y04 --mode smoke --restart --swap-devices --responder-first
```

Toolchain remains JDK 17.0.15+6, Gradle 8.13, AGP 8.11.1, Kotlin 2.2.0, SDK 36/build tools 35.0.0, Rust 1.85.1 and Python 3.14.4, with all outputs/cache/temp inside the repository. Shared Rust and normal/security native libraries are unchanged; their preceding validation remains applicable with Android consumption/vector checks here. No fresh shared-core or iOS build is required for this Kotlin-only callback delta; broader Mac/native iOS and independent assessments remain open. The baseline positive method is unchanged in the final test APK; the added negative assertion verifies that a fresh epoch does not inherit the old release target as well as rejecting old-epoch callbacks.

Default ticketboard validation, all 12 board tests, 60 relative file references and `git diff --check` pass on the completed evidence update. All broader acceptance gates remain open; no merge or completion is authorized by these limited results alone.
