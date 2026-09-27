# MC-025 deferred-release confirmation — 2026-09-26

## Finding and bounded repair

The [shared-role candidate](MC-025-shared-role-results-2026-09-26.md) delivers all 28 paired exchanges, but tablet restarts recover in 54.770/50.346 seconds. Its W02 phone framework log shows `onServiceChanged` at 1790487717.167, client cancellation/close at .226–.228, deferred server cancellation at .230, and successful server DISCONNECTED at 1790487718.279. The peripheral is already retired, so the successful confirmation is unmatched and does not refresh the empty server. A later incoming connection is cancelled, then the existing delayed epoch refresh and outgoing retry permit recovery. W02/W04 are retained as the unchanged-candidate physical baseline; no new baseline paired trial is substituted for them.

The repair retains one nullable release address in the current server epoch, only after closing the final client and successfully issuing its deferred same-address server cancellation while the driver is empty. The matching successful server DISCONNECTED callback may refresh that still-empty epoch once. Matching error/disconnect consumes the target; any new native-admitted client attempt or server CONNECTED callback invalidates it; reset clears it. An unrelated callback cannot consume or use the target. If the confirmation never arrives, existing delayed cleanup remains available.

Refresh still replaces the entire epoch. There is no retirement clearing in place, retry/idle deadline shortening, native credit reset/refund, identity assertion or permission to admit payloads. Pure refusals, arbitrary unmatched/old/error callbacks, surviving links/setups and intervening connection activity do not acquire immediate refresh. The target is a bounded teardown correlation, not an authenticated identity or a claim to identify a physical ACL generation. No Rust, native library, wire/FFI, storage, dependency or iOS source changes.

## Baseline and callback fixtures

The unchanged app fails `GattReleaseConfirmationTest#lastClientReleaseRefreshesAfterConfirmedDisconnect` at the expected fresh-epoch assertion in **2.608 seconds**. The client/peripheral teardown and successful server disconnect have occurred, but the epoch remains unchanged. Baseline app SHA-256 is `7a9e37e29bcb531f2b9f23bde25029a77dc2f9e1d9a2ae6642be73fc15a0e178`; test APK is `cab2a5411cb7abaf73f5100eecc0184cc8e46d1b8fc1db8e61f31559bcc2b300`. Build and failure logs are retained under `.work/mc025/2026-09-26-release-confirmation/`.

Seven opt-in methods cover final release confirmation, matching error consumption, unrelated disconnect, intervening client attempt, retired reconnect, surviving setup and old server epoch. Each runs separately with the other endpoint stopped/Bluetooth disabled. Synthetic callbacks use the real adapter and production core; no native time/credits are changed, and no profile/history/permissions are reset. The callback monitor prevents asynchronous fixture interference. Tests assert unchanged server before confirmation, required refresh or conservative retention, no new token from late callbacks and one-use consumption. These fixtures are not over-the-air fault or continuous-session admission stress tests.

The existing four shared-role, six retirement, one refusal and three Bluetooth shutdown cases also run separately per device, for **42 planned isolated checks** in total.

## Paired plan and validation status

Before execution: Y01 phone initiator/phone first, Y02 tablet initiator/tablet first, Y03 phone initiator/tablet first, Y04 tablet initiator/phone first. Each plans three short foreground pairs, a five-second initiator stop/start, three 280-byte foreground pairs and one short pair with the responder activity stopped. Retain all **28 planned pairs**, including any failed/unattempted outcomes, missing readiness and explicit interruptions. No candidate physical result is claimed yet.

Android Debug/Release/test builds, app lint and **26 app JVM tests** pass; standalone BLE builds/lint, **29 BLE JVM tests** and both probe APK checks pass. App Debug/Release native-set/ELF and 16-KiB ZIP alignment checks pass. The final strengthened old-epoch test APK is rebuilt and linted. Both installed app/test pairs match the candidate hashes below. Device checks are in progress. Required independent review is pending. Raw evidence remains under the new root. Existing broader hardware/energy/mixed-OEM/Mac/iOS/independent-security gates remain open, MC-025 stays in progress and PR #39 unmerged. Both Samsung API 36 devices use existing synthetic profiles and USB power; these trials cannot establish battery drain.

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
```

Toolchain remains JDK 17.0.15+6, Gradle 8.13, AGP 8.11.1, Kotlin 2.2.0, SDK 36/build tools 35.0.0, Rust 1.85.1 and Python 3.14.4, with all outputs/cache/temp inside the repository. Shared Rust and normal/security native libraries are unchanged; their preceding validation remains applicable with Android consumption/vector checks here. No fresh shared-core or iOS build is required for this Kotlin-only callback delta; broader Mac/native iOS and independent assessments remain open. The baseline positive method is unchanged in the final test APK; the added negative assertion verifies that a fresh epoch does not inherit the old release target as well as rejecting old-epoch callbacks.
