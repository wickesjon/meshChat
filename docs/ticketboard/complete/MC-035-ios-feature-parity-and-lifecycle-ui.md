---
id: "MC-035"
title: "iOS feature parity and lifecycle UI"
depends_on: ["MC-026","MC-022","MC-029","MC-030","MC-031","MC-032","MC-042"]
kind: "ios"
branch: "ticket/MC-035-ios-feature-parity-and-lifecycle-ui"
---

# MC-035 — iOS feature parity and lifecycle UI

## Objective

Implement SwiftUI parity for onboarding/channels, friends/DMs, event trust, sharing, themes and Supporter flows using the shared core.

## Dependencies

`MC-026`, `MC-022`, `MC-029`, `MC-030`, `MC-031`, `MC-032`, `MC-042` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/ios/UI/**`, `src/ios/**`, `tests/integration/ios-ui/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

### Implementation plan and standing scope approval — 2026-09-17

All hard dependencies are complete on main. Branch starts from MC-042 squash `1861fb6bf20ecaff03deb5f9b79dfb9d79c78a11`. Integrate the actual shared native core, Keychain/SQLCipher adapters, CoreBluetooth driver and StoreKit policy behind a serialized SwiftUI feature owner. Preserve explicit confirmation for channel joins, friend pins/replacement, event roots and staff imports; plaintext channel and authenticated encrypted DM boundaries; generation-bound short-lived private operations; protected failure/reset behavior; iOS foreground catch-up and honest background/Beacon limits. Settings, themes, power and contribution panels use protected settings and existing components.

Necessary additional scope under standing repository-local approval: `.github/workflows/ci.yml` registers native UI/integration acceptance and production device/simulator builds; `tests/bench/ios/**` may extend existing driver checks for protected egress hooks while retaining their original coverage. Production and test entry points remain separate: synthetic wrapping/storage and deterministic radio inputs live only under `tests/integration/ios-ui/`, never as a production fallback. Mac/Xcode compilation and synthetic UI/integration checks are required; real-device flow/restoration and protection remain MC-027/044. Record gaps as blockers rather than reducing the v1 requirements.


- Implement SwiftUI parity for onboarding/channels, friends/DMs, event trust, sharing, themes and Supporter flows using the shared core.
- Integrate Keychain-backed storage/lifecycle behavior and explicit iOS background degradation/foreground SYNC states.
- Provide an explanatory disabled Beacon Mode entry and native accessibility/confirmation protections.

## Exit criteria

- [x] Each shipping v1 flow passes native UI/integration tests with the production frozen core and synthetic inputs, plus applicable Mac/Xcode builds. Real-device flow/lifecycle acceptance remains mandatory in MC-027.
- [x] Key reset, background restoration, offline purchase cache and foreground catch-up behave as specified in automated/native checks; physical lifecycle acceptance remains MC-027/044.
- [x] Native iOS screens preserve the same plaintext/encrypted and verified/unverified boundaries as Android.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If a platform capability differs, provide the approved equivalent behavior with honest UI wording.
- Do not silently remove a v1 feature; record a release-blocking gap or request a scope decision.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Scheduling approved 2026-09-14 in [the validation policy](../../decisions/local-validation-policy.md#physical-acceptance-scheduling--approved-2026-09-14). Implementation consumes the native driver and independently frozen core before physical interop. MC-027 now follows this ticket and owns every shipping flow on the real iOS device matrix; no native build waiver is implied.

Implementation and applicable native acceptance pass, with separate Terra code review recorded below. This completion record is staged for final evidence review and squash merge; the combined final checkbox remains open until that merge. Physical device/OS/duration reports remain the responsibility of MC-027/044; no physical acceptance is claimed.

## Review and merge

- Branch: `ticket/MC-035-ios-feature-parity-and-lifecycle-ui`.
- Review/PR: https://github.com/wickesjon/meshChat/pull/38 — ready PR; applicable checks and Terra code review pass. The PR records the exact final documentation revision and actual review outcome before squash merge.
- Squash commit title: `MC-035: iOS feature parity and lifecycle UI`.
- Completion becomes effective only when the reviewed squash commit lands on main.

### Additional shared-core integration scope

Inspection found that the native transport serves canonical SYNC requests but does not expose a requesting/received-history path to native feature owners. MC-035 explicitly requires foreground catch-up; merely changing its label would not satisfy that requirement. Necessary repository-local scope extension: `src/core/src/native_transport.rs`, its private native integration modules and registrations, plus `tests/integration/ios-ui/**` and shared native FFI regressions as needed, to expose the existing bounded SYNC session machinery without changing the frozen wire contract or trusting unverified content. Validate link-bound requesting, first-native-attempt deadline, received stored history/authentication, continuations, cancellation/restoration and ordinary transport budgets. This requirement remains open until the implementation and tests pass.

### Initial implementation checkpoint

Initial native application build on `c58f7e4` identified Swift state-wrapper declarations and a lost MainActor annotation; these are corrected, with native rerun pending. The catch-up wrapper compiles and passes strict workspace clippy. Four targeted Rust tests now pass: actual two-page native catch-up of six cached messages without live relay/count inflation; rejection of unsolicited/stale wrappers and admission of stored TTL zero; request timeout and no quota reset; and real signed-friend/encrypted-DM history through canonical crypto and pin owners. Full validation, native UI tests and separate review remain pending. No acceptance checkbox is closed.

Additional exact test registration path under standing scope approval: `tests/integration/ffi/main.swift` may call new Swift driver/feature regression helpers, and `src/core/Cargo.toml` registers the requester integration test. The new native integration module and existing channel/organizer history readers use StoredChat parsing only after canonical SYNC admission; no wire grammar or cryptographic construction changes.

### Automated validation checkpoint

Core revision `8a205b7`: local Rust 1.85.1 read-only formatting check, strict workspace/all-target/all-feature clippy, complete workspace debug and release suites, release build and 14 storage policy checks pass. An existing native-transport fixture reused a process-ID-based temporary database name on Windows; removing only its ignored `.work/friends-tests/*-native_transport-database-*.sqlite*` files resolved the schema setup failures, and all 14 native transport tests and the full rerun pass. No production storage fallback was added. Hosted Rust also passes in run `35273161582`.

Mac native builds exposed and corrected Swift state declaration/actor annotations, two unnecessary throwing annotations, and an existing diagnostic SYNC event expectation. The separate synthetic native acceptance host now compiles the production SwiftUI/model/SQLCipher/core/driver with test-only wrapping and framed-radio inputs. Its generated project and evidence stay under `.work/ios-ui/`, and no test switches or fallback protection are in the production entry point. Native test execution and final device/simulator builds remain pending.

Initial separate Terra medium review of `ba7d9befe418b6bab772408c430755d75630c3ba` found P2: own and peer DM rows may share an ID, so SwiftUI cannot key only by that ID. Corrected with direction-plus-ID keys and a native UI collision/removal regression. Review outcome was FAIL/pending until the fix is reviewed and native checks pass; no other code-level finding was reported. Author also removed an unintended exported history helper so only the canonical Rust requester can call the stored-channel intake. Targeted catch-up/channel tests pass after that API restriction. Follow-up review will cover the final revision.

Additional necessary test scope under standing repository approval: `tests/integration/friends/database.rs`, the shared synthetic SQLite adapter consumed by the new catch-up tests and existing native suites. Repeated validation on Windows reused process IDs and reopened previous test databases, causing schema failures before tested logic ran. Reserve new fixture files atomically instead of assuming PID/counter names are unused; validate all affected suites and debug/release tests. No production store or schema changes.

With collision-safe fixture creation, all 209 debug and 209 release tests, clippy/format/release build and 14 storage-policy checks pass. Regenerated Android bindings/ABIs and application builds/lint/JVM tests also pass after restricting the internal history helper. Mac production Debug/Release simulator/device builds pass on `16f7ee8` and `bddd0fa`; native acceptance remains open. The first simulator run entered recovery during onboarding; a subsequent run found a test fixture using an array where UniFFI requires Data, now corrected. Protected-file verification now compares the documented NSString/String value against the exact complete-protection raw value ([Apple API contract](https://developer.apple.com/documentation/foundation/fileattributekey/protectionkey)); the required protection level and fail-closed behavior are unchanged. Native preflight diagnostics and reruns must confirm protected creation and all flows before merge.

### Follow-up review and reproducible validation

Separate `gpt-5.6-terra` medium follow-up review of `58b9469cf899097a40df75b01c1f51b100eae193`: **PASS for code**, no new findings. The reviewer confirmed direction-plus-ID row identity and its native regression, internal-only stored-history intake, synchronous scoped storage batching, collision-safe test databases, strict complete-protection metadata comparison and metadata-only test diagnostics. The review explicitly retains the current-revision Mac native acceptance gate; this is not merge approval while that gate is unresolved.

Local Windows checks covering the current Rust/Android source: Rust/Cargo 1.85.1 `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, `cargo test --workspace --all-features --locked` with and without `--release`, `cargo build --workspace --all-features --locked --release`, and `python -B tests/integration/storage/run_policy.py` all pass (209 debug, 209 release, 14 policy tests). Cargo-deny 0.20.2 advisory/bans/license/source checks pass; no lockfile change. Android bindings and arm64/x86_64 ABIs were regenerated after the export restriction. With JDK 17.0.15+6, Gradle 8.13, AGP 8.11.1, Kotlin 2.2.0 and NDK 27.3.13750724, app `assembleDebug assembleRelease assembleDebugAndroidTest lintDebug testDebugUnitTest` pass. `tests/integration/android-ui/check_apk.py` and `zipalign -c -P 16 4` pass for both app APKs. Ticketboard validation, 12 ticketboard unit tests and `git diff --check` pass. Logs remain in ignored `.work/mc035/`.

Mac run [35276552031](https://github.com/wickesjon/meshChat/actions/runs/35276552031), source `58b9469`, uses Xcode 16.4 / Swift 6.1.2 / iOS SDK 18.5. Swift host FFI/driver and Supporter checks, all four unsigned application builds (Debug/Release, simulator/device), and the hosted Rust job pass. Native UI/integration execution and subsequent applicable Mac checks remain pending. No failed or canceled run is represented as passing evidence.

That run's native result is **FAIL**: the opposite-direction collision/removal UI test passes, but onboarding and feature integration stop at protected storage creation (`provider`, database protection metadata absent). Subsequent Mac checks are skipped, not passed. This demonstrates that the metadata comparison change alone did not resolve onboarding. Test-only file-adapter tracing now preserves original behavior/errors and records operation names plus protection/backup metadata to narrow the earlier failure. No keys, envelopes or message content are included. All exit criteria remain open.

The backup exclusion guard now explicitly clears URL resource caches after setting exclusion, so its strict `true` check reads filesystem metadata. Apple documents that URL metadata may persist within one run-loop pass; the synchronous creation path must not rely on a stale cached value. This preserves the required exclusion and fail-closed result. It is a candidate correction, not a claimed resolution until native tests pass; the added tracing remains available if creation still fails.

Run `35278323296` at `b0b1844` again passes all four production app builds, host regressions, Rust and the row-collision UI regression, but native onboarding fails. Tracing reports no file-adapter failure, folder present, backup exclusion true and absent database protection metadata. Fresh backup verification was not the runtime blocker. Terra reviewed exact `b0b1844b489dd003f369ffa91b833aadf68c8226`: PASS for code, native acceptance still required.

The synthetic acceptance boundary now explicitly includes the hardware-dependent database file-protection operation, alongside the existing synthetic wrapping/radio ports. Keep real SQLCipher, filesystem persistence/backup exclusion, generation binding and protected lifetime checks. The native factory must always select the unchanged strict Complete-protection setter/verifier; no simulator fallback enters the shipping app. Test the native verifier's refusal when Complete metadata is absent and inject a test-only availability gate for feature flows, including gate failure/recovery. Record this as simulated file protection, never physical certification; actual protection remains MC-044 under the existing scheduling policy. This is within the approved iOS/test paths and preserves the previously required physical gate.

Terra follow-up on `f4cfc4641b9540f2c37799f00bbe5e9737bb37ae` accepted the explicit synthetic boundary but raised P1 after inspecting launch execution: the full synchronous integration suite ran inside the test application's initializer, before its first screen. That supersedes its initial code PASS. The harness now renders a running result, starts an async MainActor task, yields between meaningful phases and publishes completion. XCUITest waits for a completed result and still requires the exact PASS value. These changes are confined to the test host; follow-up review and native execution remain required.

Terra follow-up PASS for code at `457d766dc481174eca3353abd3fbbfaa1afc77c5`. Native run `35279942245` passes all four application builds, Swift/Supporter regressions, Rust, the full production feature integration suite (40 seconds), and the opposite-direction row regression. Onboarding and channel confirmation now pass in the screen test. The remaining screen-test failure is an offscreen, lazily created Settings Form row: the test queried the disabled Beacon control without scrolling. The test now waits for Settings and uses at most six upward scrolls before retaining both existence and disabled-state assertions. No product behavior or acceptance assertion is removed. The remaining screen flow and later Mac checks still require a passing run.

### Current native acceptance

Terra follow-up reviewed exact `1c46d16148de09564fadfa85f9952475315b5dd3`: **PASS for code**, no findings. Run [35281637468](https://github.com/wickesjon/meshChat/actions/runs/35281637468) at that revision passes all three native acceptance tests on the iPhone 16 Pro simulator, iOS 18.5: shipping screen navigation/confirmations (73.924 s), opposite-direction DM row identity/removal (11.586 s), and full production feature integration (94.606 s); zero failures, 180.117 s total. Durations describe test execution, not physical performance certification. Retained `mc035-ios-ui` artifacts include the full native log/result bundle and disposable public screenshots. The channel plaintext warning, public friend QR/fingerprint, and disabled Beacon explanation were visually inspected. Local copies are under ignored `.work/mc035/native-evidence-1c46/`.

The entire Mac job is **PASS** on this revision: all four production application builds, Swift host/driver and Supporter regressions, four BLE probe builds, four security probe builds, identity/organizer import regressions, Swift curve interoperability, and real SQLCipher create/reopen/negative/key-loss/reset and native crypto integration. The hosted Rust job also passes. Commands are the pinned `ios`/`rust` steps in `.github/workflows/ci.yml`, including `python3 -B tests/integration/ios-ui/run_apple.py`, `tests/integration/storage/run_apple.py`, `tests/integration/organizer-tools/run_apple.py` and the warning-as-error Swift host compiles. Full Mac log is retained locally in `.work/mc035/ios-final.log`. Remaining hosted Android emulator checks are still running; final merge remains pending that applicable gate and final evidence review.

### Android acceptance blocker and necessary test scope

Run `35281637468` passes Android builds/lint, security and SQLCipher lifecycle, and all channel UI phases, but fails the contribution reset UI assertion at `StatsUiTest.kt:35`: it requires the published elapsed window to be below five seconds. The full production refresh performs protected database operations before publishing; software emulation can exceed this arbitrary interval. Investigate and validate a test-only correction in `tests/integration/stats/android/StatsUiTest.kt` under standing repository-local scope approval: compare the reset window against its observed pre-reset value, retain zero-counter assertions and all export checks, and retain the exact zero-time/credit-preservation native core regression. This changes no product behavior or physical performance threshold. Required evidence: rebuilt test APK, actual emulator execution of the corrected flow and remaining native consumer suites, followed by Terra review. The Android job is FAIL; later suites were skipped, never passed. Merge remains blocked until resolved.

Terra medium follow-up at exact `0145cb272aad6adc2757c4f0b2446a2d9c74a4b1`: **PASS for code**, no findings. The reviewer confirms that monotonic `now - since` makes a smaller observed window positive reset evidence, while native tests retain exact reset/no-credit-refill coverage. Rebuilt test APK and corrected local contribution test pass. A first organizer run used expired disposable credentials from an earlier build and was explicitly stopped; regenerate with `cargo test --locked -p meshchat-core --test native_organizer generate_native_organizer_fixtures` before packaging. Fresh-fixture channel/contribution and all three organizer phases pass. This setup correction changes no tracked fixture or expiry policy. Remaining consumer checks and final evidence review are pending.

The final local consumer sequence passes channel, contribution, organizer, friends/DM, sharing and all three synthetic billing phases. Beacon enable/reopen-accessible-exit/re-enable pass, but the external touch runner exhausts ten startup polls before protected load publishes its button. A subsequent read-only UI dump shows the correct visible target. Necessary test-only scope extension: `tests/bench/beacon/run_android.py`, replace the arbitrary startup poll count with the existing 120-second monotonic deadline already used for protected post-touch persistence. Retain the real 2.3-second touch, target bounds checks, explicit failure, and restart assertions. This is harness readiness, not a waived device-performance gate; rerun all Beacon phases and obtain Terra follow-up.

### Final development acceptance and merge packet

Source `5a018c109002aeb3119fa914e512e07dfe7d8598`: separate Terra medium follow-up **PASS for code**, no findings. All Beacon phases now pass, including the external 2.3-second stationary touch and force-stop/reopen verification of both disabled preferences. The final changes after fully validated production revision `1c46d16148de09564fadfa85f9952475315b5dd3` affect only the two Android test readiness assertions and ticket evidence; shared core, bindings, production Android and all iOS inputs are identical.

Local Windows Android evidence uses emulator 37.1.11.0 (build 15917651), Android 10/API 29 x86_64, isolated serial `emulator-5580`, 4 KiB runtime pages. Fresh disposable organizer credentials were generated using the existing command above; `:app:assembleDebugAndroidTest` passes with the already recorded pinned JDK/Gradle/Kotlin toolchain. The following commands all pass with `--serial emulator-5580`:

- `python -B tests/integration/android-ui/run_emulator.py`: all three channel phases.
- `python -B tests/integration/stats/run_android.py`: production panel, explicit selection/export bounds, privacy and reset.
- `python -B tests/integration/android-ui/run_organizer_emulator.py`: import, reopen and forget.
- `python -B tests/integration/android-ui/run_friends_emulator.py`: pin/DM/reopen/replacement and protected archive coverage.
- `python -B tests/integration/sharing/run_android.py`: offline canonical QR/link/export and confirmation lifecycle.
- `python -B tests/integration/billing/run_android.py`: synthetic purchase, reopen/refund and link-cap phases.
- `python -B tests/bench/beacon/run_android.py`: enable, reopen/accessibility exit, enable and real touch/persisted exit.

Logs remain under ignored `.work/mc035/local-*.log`; the authoritative final Beacon log is `local-beacon-corrected.log`. Public synthetic screenshots remain in their runner evidence directories. Debug production APK SHA-256: `2f167a92e6aec3081182028f57a8df373715d1828fa340d6f2001579654294b0`; fresh-fixture test APK: `bbf7fce5b5766304b2f7a2f0201539d87f861f1af85b56908d5beb482981c5b7`.

Under the approved local-validation policy, the all-passing Mac job and hosted Rust/security/storage/channel results on identical production inputs combine with the final local Android consumer results, build/alignment/lint/JVM/core/security evidence above and required Terra review. Hosted Android run `35281637468` remains FAIL; subsequent supplemental runs are not claimed passed. Both observed harness failures were investigated, corrected without production changes, independently reviewed and actually rerun successfully. No failed applicable source test is waived. Static 16 KiB APK checks and 4 KiB emulator runtime are distinct evidence.

The final revision changes only this completion record and generated ticketboard state. Regenerated/default board validation, all 12 board unit tests and `git diff --check` pass. Final exact-revision review is recorded in PR #38; completion becomes effective only upon its squash merge. Physical MC-025/027/043/044, real store/domain integration and independent integrated security assessment remain separate required gates; none is certified here.
