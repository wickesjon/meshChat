---
id: "MC-002"
title: "Build layout and continuous integration"
depends_on: ["MC-001"]
kind: "foundation"
branch: "ticket/MC-002-build-layout-and-continuous-integration"
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

- [ ] Rust tests, formatting, strict linting and dependency checks pass from a fresh checkout.
- [ ] Android skeleton builds; the macOS job builds the iOS skeleton with recorded SDK/toolchain versions.
- [ ] No generated build output, credentials, or signing material is tracked.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If macOS runners are unavailable, record the blocker and use a documented local Mac build for development; do not label the cross-platform gate passed.
- Document any audited dependency exception with scope and expiry instead of disabling the entire check.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Implementation prepared and locally checked on Windows; hosted Linux/macOS validation and review remain pending. MC-002 stays in progress until those gates pass.

- Baseline verified against remote main: `8346be06e4bcce39e1779e0f0d3982cd508dc39b`; MC-001 is complete. The checkout was clean before branch creation.
- Added an unpublished, dependency-free Rust workspace; Rust 1.85.1, edition 2024, inherited `unsafe_code = "forbid"`, release overflow checks, and a committed lockfile. MC-003 owns the first core API and behavior tests; MC-002 does not introduce placeholder protocol functions or meaningless unit tests.
- Android uses AGP 8.9.2, Gradle 8.11.1 (distribution and wrapper SHA-256 verified), Kotlin 2.1.20, Temurin 17.0.15+6, API 35 / build tools 35.0.0, and minSdk 29. This matches [AGP's compatibility table](https://developer.android.com/build/releases/agp-8-9-0-release-notes). The minimal native Activity will be replaced by the Compose shell in MC-028. No radio permissions or sensitive persistence are introduced.
- iOS uses a checked-in Xcode project and shared scheme, Xcode 16.4, Swift 6 language mode and iOS 15 minimum. CI records compiler and SDK versions and builds Debug and Release for the simulator without signing. Bundle IDs are local skeleton identifiers, not registered store identifiers.
- CI is configured for every PR and main push: board validation/tests, Rust format/strict Clippy/debug and release tests/build, cargo-deny 0.20.2 advisory/license/source/duplicate-version gates, Android debug and unsigned release assembly plus strict lint, and iOS builds. Action references are pinned to resolved commit hashes. SDK update suggestions alone are excluded from Android lint because this ticket intentionally pins a build baseline; other lint warnings remain errors.
- Dependency/license policy currently covers the Rust dependency graph (empty at baseline). Native skeletons use OS frameworks plus Kotlin's pinned standard library. No project license or distribution rights are assigned by this ticket; private workspace crates are excluded from third-party license classification.
- Build outputs, Gradle/Cargo/Rustup caches, temporary files and debug signing material are redirected to ignored repository paths. CI reads the runner's preinstalled Android SDK and disables Gradle SDK auto-downloads. No release signing material is created.
- Local checks: `python -B tests/ticketboard/validate.py` passed (42 tickets, 99 edges); `python -B -m unittest discover -s tests/ticketboard -v` passed (12 tests); `git diff --check` passed.
- Provisioned checksum-verified Rust 1.85.1, Temurin 17.0.15+6, Gradle 8.11.1 and Android API 35 revision 2 / build-tools 35.0.0 in ignored `.work/` paths after finding no relevant tools on PATH. The existing Visual Studio Build Tools supplied the Rust linker.
- Local Rust checks passed: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`; `cargo test --workspace --all-features --locked` in debug and release; `cargo build --workspace --all-features --locked --release`; unchanged `Cargo.lock`. The empty skeleton correctly reports zero Rust behavior tests; this is build evidence only.
- Local Android checks passed: `gradlew.bat -p src/android --offline --no-daemon :app:assembleDebug :app:assembleRelease :app:lintDebug` (87 tasks, build successful). Strict lint initially identified missing icon and Android 12+ extraction rules; added a platform placeholder icon and explicit backup/transfer exclusions, keeping MC-018's persistence gate intact. Kotlin compilation runs in-process to avoid a separate compiler daemon cache. No emulator/device behavior is claimed.
- Local dependency gate passed with the checksum-verified official cargo-deny 0.20.2 binary: `cargo-deny --locked --config src/core/deny.toml check` reports advisories/bans/licenses/sources OK. It emits expected unused-license-allowance warnings for the dependency-free baseline. Initial cargo-deny 0.18.3 failed on current CVSS 4.0 advisory data; replaced the tool with 0.20.2 rather than ignoring advisories. The tool's prebuilt binary keeps the core on Rust 1.85.1.
- `actionlint` 1.7.7 passed against `.github/workflows/ci.yml` (shellcheck unavailable on this host). Gradle wrapper JAR SHA-256: `2db75c40782f5e8ba1fc278a5574bab070adccb2d21ca5a6e5ed840888448046`.
- Java's temporary directory and `user.home` are redirected into `.work/` as well as Android/Gradle preferences, preventing fallback analytics/settings writes outside the checkout. No global PATH, registry, Git settings or machine ACL changes were made.
- Publication blocker: automatic approval review rejected the ticket-branch push to `https://github.com/wickesjon/meshChat`, requiring explicit user authorization for uploading the source/CI payload to that destination. No push or PR creation occurred. Hosted Linux and Xcode 16.4 builds, independent/user review and squash merge are still pending; no passing CI or macOS evidence is claimed.

Reproduce from the repository root with Rust 1.85.1 installed: set `CARGO_HOME`, `RUSTUP_HOME`, `GRADLE_USER_HOME`, `ANDROID_USER_HOME` and temporary-directory environment variables to subdirectories of `.work/` before provisioning tools. Set Java's `-Djava.io.tmpdir` and `-Duser.home` to existing `.work/tmp` and `.work/java-home` directories through `JAVA_TOOL_OPTIONS`. Run the exact commands in `.github/workflows/ci.yml`. Android requires JDK 17.0.15+6 and a provisioned API 35 / build-tools 35.0.0 SDK; `bash src/android/gradlew -p src/android :app:assembleDebug :app:assembleRelease :app:lintDebug` builds it (use `gradlew.bat` on Windows). For iOS select Xcode 16.4 using `DEVELOPER_DIR` and use the workflow's `xcodebuild` commands with repository-local DerivedData. These builds do not establish BLE feasibility or release readiness.

## Review and merge

- Branch: `ticket/MC-002-build-layout-and-continuous-integration`.
- Review/PR: pending.
- Squash commit title: `MC-002: Build layout and continuous integration`.
- Completion becomes effective only when the reviewed squash commit lands on main.
