---
id: "MC-002"
title: "Build layout and continuous integration"
depends_on: ["MC-001"]
kind: "foundation"
branch: "ticket/MC-002-closeout"
---

# MC-002 — Build layout and continuous integration

## Objective

Create the Rust workspace under src/core and native project skeletons under src/android and src/ios; retain conventional platform source nesting.

## Dependencies

`MC-001` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/core/**`, `src/android/**`, `src/ios/**`, `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `.github/**`, `.gitignore`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Create the Rust workspace under src/core and native project skeletons under src/android and src/ios; retain conventional platform source nesting.
- Pin toolchains and dependency versions; enable forbidden unsafe code in owned core code, checked release overflow, formatting and lint gates.
- Configure Linux Rust and Android checks plus macOS Swift builds; run ticketboard validation on every PR and add dependency audit/license checks.

## Exit criteria

- [x] Rust tests, formatting, strict linting and dependency checks pass from a fresh checkout.
- [x] Android skeleton builds; the macOS job builds the iOS skeleton with recorded SDK/toolchain versions.
- [x] No generated build output, credentials, or signing material is tracked.
- [x] Relevant checks pass, evidence is recorded, required review is complete, and the implementation is squash merged to main through PR #3. The board closeout is staged for its reviewed squash merge.

## Potential fallbacks

- If macOS runners are unavailable, record the blocker and use a documented local Mac build for development; do not label the cross-platform gate passed.
- Document any audited dependency exception with scope and expiry instead of disabling the entire check.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

MC-002 implementation was accepted by the user and squash merged through PR #3. This closeout stages complete board status and records the user's new review workflow; board completion takes effect when the closeout lands on main.

- Baseline verified against remote main: `8346be06e4bcce39e1779e0f0d3982cd508dc39b`; MC-001 is complete. The checkout was clean before branch creation.
- Added an unpublished, dependency-free Rust workspace; Rust 1.85.1, edition 2024, inherited `unsafe_code = "forbid"`, release overflow checks, and a committed lockfile. MC-003 owns the first core API and behavior tests; MC-002 does not introduce placeholder protocol functions or meaningless unit tests.
- Android uses AGP 8.11.1, Gradle 8.13 (distribution and wrapper SHA-256 verified), Kotlin 2.2.0, Temurin 17.0.15+6, API 36 / build tools 35.0.0, and minSdk 29. This matches [AGP's compatibility table](https://developer.android.com/build/releases/agp-8-11-0-release-notes). The minimal native Activity will be replaced by the Compose shell in MC-028. No radio permissions or sensitive persistence are introduced.
- iOS uses a checked-in Xcode project and shared scheme, Xcode 16.4, Swift 6 language mode and iOS 15 minimum. CI records compiler and SDK versions and builds Debug and Release for the simulator without signing. Bundle IDs are local skeleton identifiers, not registered store identifiers.
- CI is configured for every PR and main push: board validation/tests, Rust format/strict Clippy/debug and release tests/build, cargo-deny 0.20.2 advisory/license/source/duplicate-version gates, Android debug and unsigned release assembly plus strict lint, and iOS builds. Action references are pinned to resolved commit hashes. Build-tool/dependency version-update suggestions alone are excluded from Android lint because this ticket intentionally pins a build baseline; other lint warnings remain errors.
- Dependency/license policy currently covers the Rust dependency graph (empty at baseline). Native skeletons use OS frameworks plus Kotlin's pinned standard library. No project license or distribution rights are assigned by this ticket; private workspace crates are excluded from third-party license classification.
- Build outputs, Gradle/Cargo/Rustup caches, temporary files and debug signing material are redirected to ignored repository paths. CI reads the runner's preinstalled Android SDK and disables Gradle SDK auto-downloads. No release signing material is created.
- Local checks: `python -B tests/ticketboard/validate.py` passed (42 tickets, 99 edges); `python -B -m unittest discover -s tests/ticketboard -v` passed (12 tests); `git diff --check` passed.
- Provisioned checksum-verified Rust 1.85.1, Temurin 17.0.15+6, Gradle 8.11.1 and Android API 35 revision 2 / build-tools 35.0.0 in ignored `.work/` paths after finding no relevant tools on PATH. The existing Visual Studio Build Tools supplied the Rust linker.
- Local Rust checks passed: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`; `cargo test --workspace --all-features --locked` in debug and release; `cargo build --workspace --all-features --locked --release`; unchanged `Cargo.lock`. The empty skeleton correctly reports zero Rust behavior tests; this is build evidence only.
- Local Android checks passed: `gradlew.bat -p src/android --offline --no-daemon :app:assembleDebug :app:assembleRelease :app:lintDebug` (87 tasks, build successful). Strict lint initially identified missing icon and Android 12+ extraction rules; added a platform placeholder icon and explicit backup/transfer exclusions, keeping MC-018's persistence gate intact. Kotlin compilation runs in-process to avoid a separate compiler daemon cache. No emulator/device behavior is claimed.
- Local dependency gate passed with the checksum-verified official cargo-deny 0.20.2 binary: `cargo-deny --locked --config src/core/deny.toml check` reports advisories/bans/licenses/sources OK. It emits expected unused-license-allowance warnings for the dependency-free baseline. Initial cargo-deny 0.18.3 failed on current CVSS 4.0 advisory data; replaced the tool with 0.20.2 rather than ignoring advisories. The tool's prebuilt binary keeps the core on Rust 1.85.1.
- `actionlint` 1.7.7 passed against `.github/workflows/ci.yml` (shellcheck unavailable on this host). Updated Gradle 8.13 wrapper JAR SHA-256: `81a82aaea5abcc8ff68b3dfcb58b3c3c429378efd98e7433460610fecd7ae45f`.
- Java's temporary directory and `user.home` are redirected into `.work/` as well as Android/Gradle preferences, preventing fallback analytics/settings writes outside the checkout. No global PATH, registry, Git settings or machine ACL changes were made.
- Publication authorization: after automatic approval review initially blocked publication, the user explicitly approved pushing this branch and opening the draft PR on 2026-09-11. [PR #3](https://github.com/wickesjon/meshChat/pull/3) is open; review and squash merge remain pending.
- [Initial hosted run](https://github.com/wickesjon/meshChat/actions/runs/34611415933), head `a04238cc306d7ab1948aa7c5dbee235f74e9300e`: Linux Rust and ticketboard jobs passed. macOS 15.7.9 arm64 built both iOS configurations with Xcode 16.4 (16F6), Swift 6.1.2 and iOS Simulator SDK 18.5. The empty app emits nonblocking Xcode notices for missing AppIntents metadata and the default header-map setting; no device or runtime result is claimed.
- The initial hosted Android job assembled both variants but strict lint failed `OldTargetApi` for API 35. Updated the build baseline to API 36 and compatible pinned AGP/Gradle/Kotlin versions; retained minSdk 29 and the target-API lint gate. The earlier local Android evidence above applies to the initial baseline. Updated-baseline local assembly and lint passed (87 tasks, 12 executed and 75 up-to-date); hosted rerun passed as recorded below.
- Online lint also suggested newer Gradle via `AndroidGradlePluginVersion`. Together with `GradleDependency`, this informational version-update rule is excluded from the pinned build gate; API compatibility, correctness and dependency-advisory checks remain enabled. This is not an advisory exception or a release-readiness claim. Reassess build-tool pins with the next foundation ticket; MC-039 owns release SDK/store policy.

- [Passing hosted run](https://github.com/wickesjon/meshChat/actions/runs/34612371470), head `3c7ddfd6ab63b539f67085f8392da8db383eec75`: all four jobs passed from fresh checkouts. Android on Ubuntu 24.04 used Temurin 17.0.15+6, API 36 revision 2, build-tools 35.0.0 and Gradle 8.13; both APK variants and strict lint passed (87 tasks executed, 2m 23s). Rust format/Clippy/debug and release tests/build/advisory-license gates, iOS Debug/Release builds and ticketboard validation/tests also passed. The move to inreview changes only ticket evidence and generated board status; source/build configuration is unchanged from this verified revision.

Reproduce from the repository root with Rust 1.85.1 installed: set `CARGO_HOME`, `RUSTUP_HOME`, `GRADLE_USER_HOME`, `ANDROID_USER_HOME` and temporary-directory environment variables to subdirectories of `.work/` before provisioning tools. Set Java's `-Djava.io.tmpdir` and `-Duser.home` to existing `.work/tmp` and `.work/java-home` directories through `JAVA_TOOL_OPTIONS`. Run the exact commands in `.github/workflows/ci.yml`. Android requires JDK 17.0.15+6 and a provisioned API 36 / build-tools 35.0.0 SDK; `bash src/android/gradlew -p src/android :app:assembleDebug :app:assembleRelease :app:lintDebug` builds it (use `gradlew.bat` on Windows). For iOS select Xcode 16.4 using `DEVELOPER_DIR` and use the workflow's `xcodebuild` commands with repository-local DerivedData. These builds do not establish BLE feasibility or release readiness.

## Review and merge

- User accepted the signing correction and merged [PR #3](https://github.com/wickesjon/meshChat/pull/3) at 2026-09-11T15:10:20Z. Squash commit `769619eefd06f4210b9f677042262a4da3cbe971` has one parent, `8346be06e4bcce39e1779e0f0d3982cd508dc39b`.
- [Final implementation CI](https://github.com/wickesjon/meshChat/actions/runs/34613945949), head `61ff26260168a2f4e36c9196e533fef4b4264ed2`: all four jobs passed, including both unsigned iOS simulator configurations after the signing fix. No physical-device signing/deployment result is claimed.
- Explicit scope addition for this closeout: the user's 2026-09-11 instruction to establish the ongoing ready-PR / Terra-medium review / wait / fix / squash-merge / continue workflow authorizes its persistent rules in `AGENTS.md`. No production changes are included.
- Closeout branch: `ticket/MC-002-closeout`; [PR #4](https://github.com/wickesjon/meshChat/pull/4). A separate gpt-5.6-terra worker at medium effort reviewed `9a8b73cd4db1f1f2b07da0166494b06610e40716` against main `769619eefd06f4210b9f677042262a4da3cbe971` and returned no findings. It independently confirmed whitespace checks, ticketboard validity and all 12 tests. This follow-up records that review without changing implementation or policy.

- User P1 review fix: removed `CODE_SIGNING_ALLOWED = NO` from both checked-in iOS target configurations so developer signing can be configured for MC-004 physical iPhone work. The existing CI `xcodebuild` command retains its unsigned-build override. Local inspection confirms the override exists only in CI; ticketboard validation and `git diff --check` passed. The hosted simulator rerun passed as recorded above; physical-device signing/deployment is not claimed.

- Branch: `ticket/MC-002-build-layout-and-continuous-integration`.
- Review/PR: [PR #3](https://github.com/wickesjon/meshChat/pull/3); user review accepted by merge. Closeout Terra review returned no findings as recorded above.
- Squash commit title: `MC-002: Build layout and continuous integration`.
- Completion becomes effective only when the reviewed squash commit lands on main.
