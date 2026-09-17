# MC-031 validation record

Base: main `81170ad54b481f483f3227f138147e2a4d2ef2d6`. Source/review revisions are recorded in the ticket and PR; no result below establishes physical-device or production-domain acceptance.

## Implementation and fixtures

The only Rust addition is an exported, inert `channel_link` adapter to `links::parse`; codec, word lists, crypto, storage, radio and wire rules are unchanged. Android reuses MC-029's scanner and friend confirmation, adds channel proposals and explicit join/cancel, and validates share exports before generating QR/HTTPS forms. White QR images have a four-module quiet zone. PNG exports use bounded private cache files and temporary read-only provider grants. Clipboard metadata marks copied links sensitive; the UI explains that this does not prevent clipboard reading.

The static fallback page has no network API calls or third-party resources. Local fixtures cover every one of the 8,000 channel triples, exact friend encoding and malformed inputs. Association templates accept configured identifiers; absent identifiers produce empty grants and disabled store links. An uninstalled-app flow keeps the original code and requires an explicit return after installation. No auto-redirect, app-install detection or deferred-link transfer is claimed.

## Local evidence — Windows, 2026-09-17

Tooling: Rust/cargo 1.85.1, cargo-deny 0.20.2; JDK 17.0.15+6, Gradle 8.13, Kotlin 2.2.0, Android Gradle Plugin 8.11.1, compile/target API 36, minimum API 29, build tools 35.0.0, NDK 27.3.13750724. The existing pinned aligned SQLCipher 4.17.0 and graphics-path 1.0.1 AARs are reused unchanged from MC-029. Python/Node versions and final artifact hashes are recorded below when final verification finishes.

| Check | Result |
|---|---|
| `cargo fmt --all -- --check`; strict all-target/all-feature clippy; workspace debug/release tests; release build, all locked | Passed: 186 debug and 186 release checks |
| `python -B tests/integration/storage/run_policy.py` | Passed: 14 shared storage-policy checks |
| cargo-deny all-feature advisory/license/source/version gates | Passed after fetching current RustSec database; two existing unused-license-allowance warnings |
| `python -B src/core/build_bindings.py android` | Passed: generated Kotlin/Swift bindings plus both Android native ABIs |
| Android Debug/Release app, Debug test APK, lintDebug, testDebugUnitTest | Passed; final cache-regression rerun pending |
| Local website build; Python template suite; Node page/URI fixtures | Passed: 5 Python checks and exhaustive 8,000-channel JavaScript fixtures |
| Production channel emulator baseline | Passed: create, reopen and composer phases |
| Sharing emulator | Passed: share and reopen phases; final bounded-cache regression rerun pending |
| Browser inspection | Local fallback rendered correctly; Open installed app with no handler retained the page and code, with unavailable-store explanation |
| Native APK set/ELF and ZIP alignment | Pending final artifacts |
| Ticketboard validator, 12 unit checks and `git diff --check` | Passed; repeat on final metadata |
| iOS generated consumer/native build | Required on this new exported API revision; pending hosted run |
| Terra medium review | Required; pending |

Scripts/logs remain under ignored `.work/mc031/`. Android environment matches [the Windows build guide](../testing/windows-android-build.md). Run `tests/integration/android-ui/run_emulator.py --serial emulator-5580` first to install the current app/test artifacts and create the protected synthetic baseline, then `tests/integration/sharing/run_android.py --serial emulator-5580`. The sharing run assumes that fresh baseline, so repeat both after source changes. Existing friend regressions run sequentially after it. Never select a physical phone implicitly.

The isolated emulator is API 29 x86_64 image revision 8 on WHPX, emulator 37.1.11 (15917651), adb 37.0.1 (15733141), explicit port 5580. The app has no INTERNET permission. Tests decode production QR bitmaps/PNG streams with the scanner's actual ZXing backend; they do not inject a camera result or claim physical optics. Native implicit ACTION_VIEW resolution is exercised with the app package constrained, so it proves manifest routing and confirmation, not default association. The clipboard sensitive flag is read back as metadata on API 29; Android 13+ visual preview suppression is documented platform behavior, not certified by this emulator. Screenshots are synthetic and remain ignored.

## Applicability and pending external gates

The exported shared adapter requires both Kotlin and Swift compilation. The existing iOS CI job supplies generated consumer and native simulator build evidence; MC-035 still owns actual iOS sharing UI integration. Android app/native checks and protected-store friend/channel regressions cover the changed feature path. Separate BLE/security probe sources, storage schema, crypto and dependencies are unchanged, so their earlier physical or independent-assessment results are not newly claimed by this PR. The local-validation policy allows relevant passing local checks plus actual Terra review, but cannot waive the new API's iOS build.

Actual camera scanning, OEM/lifecycle behavior, device protection and website association remain with MC-025/027/043/044 and release gates. Production domain/store activation remains blocked by missing owned identifiers and separate deployment authorization. See [the activation guide](README.md) for exact procedures and the frozen HTTPS hostname constraint. No deployment, publication, payment or domain verification was performed.
