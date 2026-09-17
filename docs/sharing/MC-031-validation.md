# MC-031 validation record

Base: main `81170ad54b481f483f3227f138147e2a4d2ef2d6`. Source/review revisions are recorded in the ticket and PR; no result below establishes physical-device or production-domain acceptance.

## Implementation and fixtures

The only Rust addition is an exported, inert `channel_link` adapter to `links::parse`; codec, word lists, crypto, storage, radio and wire rules are unchanged. Android reuses MC-029's scanner and friend confirmation, adds channel proposals and explicit join/cancel, and validates share exports before generating QR/HTTPS forms. White QR images have a four-module quiet zone. PNG exports use bounded private cache files and temporary read-only provider grants. Clipboard metadata marks copied links sensitive; the UI explains that this does not prevent clipboard reading.

The static fallback page has no network API calls or third-party resources. Local fixtures cover every one of the 8,000 channel triples, exact friend encoding and malformed inputs. Association templates accept configured identifiers; absent identifiers produce empty grants and disabled store links. An uninstalled-app flow keeps the original code and requires an explicit return after installation. No auto-redirect, app-install detection or deferred-link transfer is claimed.

## Local evidence — Windows, 2026-09-17

Tooling: Rust/cargo 1.85.1, cargo-deny 0.20.2; JDK 17.0.15+6, Gradle 8.13, Kotlin 2.2.0, Android Gradle Plugin 8.11.1, compile/target API 36, minimum API 29, build tools 35.0.0, NDK 27.3.13750724; Python 3.14.4, Node 24.15.0. The existing pinned aligned SQLCipher 4.17.0 and graphics-path 1.0.1 AARs are reused unchanged from MC-029.

| Check | Result |
|---|---|
| `cargo fmt --all -- --check`; strict all-target/all-feature clippy; workspace debug/release tests; release build, all locked | Passed: 186 debug and 186 release checks |
| `python -B tests/integration/storage/run_policy.py` | Passed: 14 shared storage-policy checks |
| cargo-deny all-feature advisory/license/source/version gates | Passed after fetching current RustSec database; two existing unused-license-allowance warnings |
| `python -B src/core/build_bindings.py android` | Passed: generated Kotlin/Swift bindings plus both Android native ABIs |
| Android Debug/Release app, Debug test APK, lintDebug, testDebugUnitTest | Passed: 12 JVM checks including all 8,000 channel triples and exact friend tuple/name HTTPS round trips; test APK/lint rerun after viewport fixture fix |
| Local website build; Python template suite; Node page/URI fixtures | Passed: 5 Python checks and exhaustive 8,000-channel JavaScript fixtures |
| Production channel emulator baseline | Passed: create, reopen and composer phases |
| Sharing emulator | Passed on `6a3d51e`: share and reopen phases, including final bounded-cache refusal/expiry recovery regression |
| Existing protected friend/DM emulator | Passed on `6a3d51e`: pair, reopen and replace; includes actual camera denial, encrypted exchange, 64-entry archive refusal and replacement/removal |
| Browser inspection | Local fallback rendered correctly; Open installed app with no handler retained the page/code; query-bearing link rejected with no open action |
| Native APK set/ELF and ZIP alignment | Passed for Debug/Release: required libraries in both ABIs, 16 KiB LOAD/RELRO and `zipalign -c -P 16 4` |
| Ticketboard validator, 12 unit checks and `git diff --check` | Passed; repeat on final metadata |
| iOS generated consumer/native build | Passed on `6a3d51e`: [hosted iOS job](https://github.com/wickesjon/meshChat/actions/runs/35198518986/job/105127381808), including generated Swift consumer, Debug/Release app simulator builds, BLE/security device/simulator builds, curve/identity checks and encrypted storage integration |
| Terra medium review | Source approved at `6a3d51e7e0f5c7f287cd92c3751a13243476f7e6`, no blocking source findings; final fixture/evidence review outcome and exact revision are recorded in [PR #32](https://github.com/wickesjon/meshChat/pull/32) before merge |

Scripts/logs remain under ignored `.work/mc031/`. Android environment matches [the Windows build guide](../testing/windows-android-build.md). Run `tests/integration/android-ui/run_emulator.py --serial emulator-5580` first to install the current app/test artifacts and create the protected synthetic baseline, then `tests/integration/sharing/run_android.py --serial emulator-5580`. The sharing run assumes that fresh baseline, so repeat both after source changes. Existing friend regressions run sequentially after it. Never select a physical phone implicitly.

Final app APK SHA-256: Debug `895df275234805889483138a49276572dec4e6d062254fba4d87ef7ea3ab8d4f`; unsigned Release `b3d7c7e93df02fd6bb327d019eab4de0cd5be246b4843e9bae7d21ad78547fc8`. Later fixture/document changes do not modify app inputs.

The prior MC-029 [hosted Android run](https://github.com/wickesjon/meshChat/actions/runs/35195542587/job/105117854898) completed during this task and failed in the channel baseline's click helper, not provisioning. Its saved `failed-create.png` shows a healthy channel list on a 320-pixel-wide screen with the join action below the visible LazyColumn. The test attempted `performScrollTo()` on an item that was not yet composed. The approved Android UI regression path now uses the same container `performScrollToNode()` approach already present in the friend tests before addressing the individual item. This preserves the assertion and performs the real user scroll; no production behavior or timeout is weakened. All three baseline and both sharing phases passed again at 320×640 / 160 dpi, with display overrides restored afterward. Logs are `small-channel-emulator.log` and `small-sharing-emulator.log`. Four small-screen sharing screenshots were inspected alongside the full-size QR/action screens; confirmation actions and white quiet zones remain visible, and longer content scrolls. The failed hosted run is not claimed passed.

The isolated emulator is API 29 x86_64 image revision 8 on WHPX, emulator 37.1.11 (15917651), adb 37.0.1 (15733141), explicit port 5580. The app has no INTERNET permission. Tests decode production QR bitmaps/PNG streams with the scanner's actual ZXing backend; they do not inject a camera result or claim physical optics. Native implicit ACTION_VIEW resolution is exercised with the app package constrained, so it proves manifest routing and confirmation, not default association. The clipboard sensitive flag is read back as metadata on API 29; Android 13+ visual preview suppression is documented platform behavior, not certified by this emulator. Screenshots are synthetic and remain ignored.

## Applicability and pending external gates

The exported shared adapter requires both Kotlin and Swift compilation. The existing iOS CI job supplies generated consumer and native simulator build evidence; MC-035 still owns actual iOS sharing UI integration. Android app/native checks and protected-store friend/channel regressions cover the changed feature path. Separate BLE/security probe sources, storage schema, crypto and dependencies are unchanged, so their earlier physical or independent-assessment results are not newly claimed by this PR. The local-validation policy allows relevant passing local checks plus actual Terra review, but cannot waive the new API's iOS build.

The successful iOS job ran on the workflow's macOS 15 runner with pinned Xcode 16.4. Only the Android UI test click helper and documentation change after `6a3d51e`; all shared API, Swift, build/dependency and iOS regression inputs are identical to that passing revision. Hosted Rust and ticketboard also passed. Hosted Android is supplemental under the approved local-validation policy, and its initial revision still contains the diagnosed small-viewport fixture bug. The corrected local native/emulator tests cover the final test change; no failed hosted run is labeled a pass and no remote protection is overridden.

Actual camera scanning, OEM/lifecycle behavior, device protection and website association remain with MC-025/027/043/044 and release gates. Production domain/store activation remains blocked by missing owned identifiers and separate deployment authorization. See [the activation guide](README.md) for exact procedures and the frozen HTTPS hostname constraint. No deployment, publication, payment or domain verification was performed.
