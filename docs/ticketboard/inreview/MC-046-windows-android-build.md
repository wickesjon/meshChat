---
id: "MC-046"
title: "Windows Android core and app builds"
depends_on: ["MC-003","MC-045"]
kind: "tooling"
branch: "ticket/MC-046-windows-android-build"
---

# MC-046 — Windows Android core and app builds

## Objective

Use the user's installed pinned Windows NDK to build the Android Rust libraries and validate the Android app with the existing Windows Gradle wrapper. Distinguish these checks from full security/emulator CI and iOS validation.

## Dependencies

`MC-003`, `MC-045` are complete on main. See the [ticket index](../README.md#ticket-index).

## Scope

User approved Windows Android tooling work on 2026-09-14. Permitted paths: `src/core/build_bindings.py` (Android target setup only), `tests/integration/build-tools/**`, `docs/testing/windows-android-build.md`, factual tooling-status updates to `docs/decisions/local-validation-policy.md` and `docs/ticketboard/implementation-plan.md`, this ticket and generated ticketboard files. Local build/cache/evidence files remain ignored inside the repository. Existing Gradle wrapper/build scripts are invoked, not changed. SQLCipher's separate Unix build path is assessed and documented; no dependency, protection, wire or iOS behavior change is authorized by this ticket.

## Implementation details

- Select the Windows NDK host tools, use the real Clang executable with explicit Android architecture/API target, preserve both ABIs and 16 KB page-size link flags, and fail clearly on missing SDK/NDK/compiler inputs.
- Preserve existing Linux/macOS Android command construction and all host/iOS behavior; add focused command-planning regression tests including paths containing spaces.
- Build normal and security-feature Rust Android artifacts. Run the existing Windows Gradle wrapper for app debug/release, lint and JVM integration tests; inspect native packaging/alignment. Keep mutable SDK/build/user caches in the repository and read installed compiler inputs without modifying the external SDK.
- Document reproducible setup and commands, actual native evidence and remaining SQLCipher/emulator/iOS limitations. No global environment changes, WSL installation or gate waiver.

## Exit criteria

- [x] Both Android ABIs build from Windows with pinned NDK 27.3.13750724 and unchanged page-size requirements, including the security-probe Rust feature.
- [x] App debug/release build, lint, JVM FFI tests and APK ABI/alignment checks pass using repository-local outputs/caches.
- [x] Command regression tests cover Windows paths/targets/errors and preserved Linux/macOS behavior; documentation identifies full security/emulator and iOS checks separately.
- [ ] Relevant local checks and separate Terra medium review pass; ticket completion takes effect only after squash merge.

## Potential fallbacks

- If required tools or checks remain unavailable, retain honest partial evidence and keep the affected gate blocked. Do not substitute an unverified SQLCipher AAR, drop an ABI, weaken linker flags or call compilation emulator/device validation.
- SQLCipher's configure/make/Tcl pipeline may require a separately scoped portability ticket or Linux runner; iOS requires Mac/Xcode. Neither is a silent exception to MC-011's required native evidence.

## Evidence

Started from main `f10d70b1d262d2f5b3392527fe5cc0eea4639cc0`. Read-only verification found NDK r27d revision 27.3.13750724 and executable Windows Clang 18.0.4 at the user-provided SDK location. Installation alone is not a build pass. No source changes from the still-open MC-011 PR are included in this branch.

Implemented Windows selection and direct Clang Android/API target arguments with the existing two 16 KB linker flags. Linux/macOS wrapper paths and flags are preserved; the host and iOS branches, Rust sources, manifests/lockfile, native app sources and SQLCipher build are unchanged. Added three focused Python regression tests covering both ABIs, paths with spaces, environment isolation, missing inputs/unsupported host and existing Linux/macOS command settings. The [Windows guide](../../testing/windows-android-build.md) records exact setup, commands and coverage limits; policy/plan changes only correct the tooling status.

Validation: Windows x64, Rust/cargo 1.85.1, Python 3.14.4, pinned NDK r27d/Clang 18.0.4, Temurin 17.0.15+6, Gradle 8.13, SDK 36 and Windows Build Tools 35.0.0. Both `python -B src/core/build_bindings.py android` and the same command with `--security-probe` pass, generating normal and security-feature ARM64/x86-64 libraries and host Kotlin/Swift bindings. Initial execution found rustup absent from PATH; adding the existing `.work/tools` to the process PATH resolved it. The external installed NDK was read/executed without modification; targets/cache/generated outputs stayed in the repository.

`src/android/gradlew.bat -p src/android --no-daemon :app:assembleDebug :app:assembleRelease :app:lintDebug :app:testDebugUnitTest` passes (97 tasks, 44 executed). Its JVM test was initially up-to-date; a separate `:app:testDebugUnitTest --rerun-tasks` passes with all 22 tasks executed, and the fresh FoundationTest report records one test, zero failures/errors/skips. The generated Kotlin/JNA regression executes the Windows host DLL, not an Android device.

`python -B tests/integration/ffi/check_android_apk.py` on both app APKs passes exact Rust/JNA library sets, ABI machines and 16 KB LOAD/RELRO alignment. Build Tools `zipalign.exe -c -P 16 4` passes both APKs. The same ELF validator also passes both security-feature Rust libraries directly; this is not SQLCipher APK/lifecycle evidence. Logs remain ignored in `.work/mc046-android-build.log`, `.work/mc046-security-rust-build.log`, `.work/mc046-gradle-app.log` and `.work/mc046-gradle-ffi.log`.

`python -B -m unittest discover -s tests/integration/build-tools -v` (3 tests), ticketboard write/default (46 tickets, 125 dependencies), 12 board unit tests and `git diff --check` pass. No lockfile change. Check applicability: only Android linker setup is changed in the shared script; all host/iOS commands and arguments are unchanged. Android execution and command-regression tests cover the affected branch. No new Rust implementation/dependency gate or unrelated Mac build is claimed necessary for this Windows-only tooling change; the separate reviewer must confirm that assessment. This does not waive MC-011's both-platform checks for its shared dependency changes.

Full security CI remains unavailable locally: the unchanged SQLCipher script requires Unix configure/make/Tcl and the current emulator orchestration uses shell tooling. No replacement AAR, reduced alignment or weaker security setting was used. Mac/Xcode, emulator runtime and physical results remain separate; no passing result here applies automatically to PR #13's different source revision.

## Review and merge

- Branch: `ticket/MC-046-windows-android-build`.
- Review/PR: pending.
- Squash commit title: `MC-046: Windows Android core and app builds`.
- Completion becomes effective only when the reviewed squash commit lands on main.
