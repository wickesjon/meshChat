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

Implementation prepared on the ticket branch; native and Rust build evidence is pending CI.

- Baseline verified against remote main: `8346be06e4bcce39e1779e0f0d3982cd508dc39b`; MC-001 is complete. The checkout was clean before branch creation.
- Added an unpublished, dependency-free Rust workspace; Rust 1.85.1, edition 2024, inherited `unsafe_code = "forbid"`, release overflow checks, and a committed lockfile. MC-003 owns the first core API and behavior tests; MC-002 does not introduce placeholder protocol functions or meaningless unit tests.
- Android uses AGP 8.9.2, Gradle 8.11.1 (distribution and wrapper SHA-256 verified), Kotlin 2.1.20, Temurin 17.0.15+6, API 35 / build tools 35.0.0, and minSdk 29. This matches [AGP's compatibility table](https://developer.android.com/build/releases/agp-8-9-0-release-notes). The minimal native Activity will be replaced by the Compose shell in MC-028. No radio permissions or sensitive persistence are introduced.
- iOS uses a checked-in Xcode project and shared scheme, Xcode 16.4, Swift 6 language mode and iOS 15 minimum. CI records compiler and SDK versions and builds Debug and Release for the simulator without signing. Bundle IDs are local skeleton identifiers, not registered store identifiers.
- CI runs on every PR and main push: board validation/tests, Rust format/strict Clippy/debug and release tests/build, cargo-deny 0.18.3 advisory/license/source/duplicate-version gates, Android debug and unsigned release assembly plus strict lint, and iOS builds. Action references are pinned to resolved commit hashes. SDK update suggestions alone are excluded from Android lint because this ticket intentionally pins a build baseline; other lint warnings remain errors.
- Dependency/license policy currently covers the Rust dependency graph (empty at baseline). Native skeletons use OS frameworks plus Kotlin's pinned standard library. No project license or distribution rights are assigned by this ticket; private workspace crates are excluded from third-party license classification.
- Build outputs, Gradle/Cargo/Rustup caches, temporary files and debug signing material are redirected to ignored repository paths. CI reads the runner's preinstalled Android SDK and disables Gradle SDK auto-downloads. No release signing material is created.
- Local checks: `python -B tests/ticketboard/validate.py` passed (42 tickets, 99 edges); `python -B -m unittest discover -s tests/ticketboard -v` passed (12 tests); `git diff --check` passed.
- Local Windows PATH has no Rust, Java, Gradle or Xcode. Native build checks will be evidenced by the Linux/macOS CI jobs; no physical-device, runtime, or independent review result is claimed.

Reproduce from the repository root with Rust 1.85.1 installed: set `CARGO_HOME`, `RUSTUP_HOME`, `GRADLE_USER_HOME`, `ANDROID_USER_HOME` and temporary-directory environment variables to subdirectories of `.work/` before provisioning tools. Run the exact commands in `.github/workflows/ci.yml`. Android requires JDK 17.0.15+6 and a provisioned API 35 / build-tools 35.0.0 SDK; `bash src/android/gradlew -p src/android :app:assembleDebug :app:assembleRelease :app:lintDebug` builds it. For iOS select Xcode 16.4 using `DEVELOPER_DIR` and use the workflow's `xcodebuild` commands with repository-local DerivedData. These builds do not establish BLE feasibility or release readiness.

## Review and merge

- Branch: `ticket/MC-002-build-layout-and-continuous-integration`.
- Review/PR: pending.
- Squash commit title: `MC-002: Build layout and continuous integration`.
- Completion becomes effective only when the reviewed squash commit lands on main.
