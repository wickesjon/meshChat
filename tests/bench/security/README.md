# MC-005 synthetic security bench

Use test devices and synthetic data only. This is an acceptance harness, not a messaging app. Do not introduce real keys or messages. Keep generated output, caches and downloaded dependencies in repository-local `.work/`. See the [probe decision/evidence](../../../docs/decisions/MC-005-probe-plan.md).

## Build checks

- Core: `cargo test --workspace --all-features --locked` and the same with `--release`; `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`; `cargo fmt --all -- --check`; existing cargo-deny policy.
- Android (Linux, pinned NDK installed): `python3 -B src/core/build_bindings.py android --security-probe`, then `python3 -B src/android/security/build_sqlcipher.py`, then `bash src/android/gradlew -p src/android/security --no-daemon assembleDebug assembleRelease assembleDebugAndroidTest lintDebug`. Run `check_android_apk.py` on both APKs and Android build-tools `zipalign -c -P 16 4` on each. The source rebuild requires git, make and Tcl. Do not substitute the unaligned published AAR or suppress the alignment check.
- iOS (Xcode 16.4): `python3 -B src/core/build_bindings.py ios --security-probe`, then build `src/ios/Security/SecurityProbe.xcodeproj`, scheme `SecurityProbe`, Debug/Release for `iphonesimulator` and `iphoneos`. CI uses `CODE_SIGNING_ALLOWED=NO` and repository-local derived data/package/module caches. A physical run needs separately available development signing. The simulator cannot exercise the Secure Enclave wrapping policy.
- CI also compiles/runs `main.swift` with generated security bindings against CryptoKit on macOS. This tests curve interoperability through FFI, not iPhone storage or hardware behavior.

## Emulator functional evidence

CI starts an isolated Android API 29 emulator and runs `python3 -B tests/bench/security/run_android_emulator.py --serial emulator-5554`. The runner refuses non-emulator serials and checks the qemu property before installing the probe/test APKs. Each create/reopen/key-loss phase runs in a separate instrumentation process, with an explicit force-stop between phases. The key-loss phase checks that ciphertext stays unchanged, the wrapping key is not recreated, and only explicit reset permits a fresh fixture. Custom instrumentation uses the platform API and targets existing private bench operations without expanding the app's exported surface. Passing output must include each named phase; an adb command exiting successfully alone is insufficient.

`emulator.sh` checks startup before compilation, stops the VM while native builds run, and starts it again immediately before testing. Readiness requires boot completion and available package/activity services. It uses KVM only when the runner already permits it, otherwise software emulation; it does not change host permissions. Startup failure retains emulator diagnostics, and cleanup targets only the isolated emulator serial.

This can establish software/native-library and emulator Keystore behavior, including actual encrypted database reads. It does not establish physical key isolation, OEM backup/device-transfer behavior or iPhone Secure Enclave operation. Host CryptoKit checks remain distinct from iPhone acceptance. The user requested this emulator path on 2026-09-11; no physical gate was waived.

## Device procedure

Record the exact Git revision, build mode, device model/OS, page size on Android, secure lock configuration and test date. Run the minimum supported OS (Android 29 / iOS 15) and current OS. Reports contain elapsed time and actual observed lock/protected-data state; copy each report before process termination. Keep only sanitized reports, never raw database/envelope/key files, in the repository.

1. Reset this app's synthetic fixture, create a fresh protected fixture, then run reopen/curves/wrong-key. Record cipher version, each operation result and available protection metadata. A provider unavailable/failed line needs investigation or an explicit unsupported capability entry; the surrounding button completing does not make that capability pass.
2. Close any held DB, terminate the process, relaunch and run reopen. Verify the original synthetic row survives and wrong-key rejection is followed by successful correct-key reopen. Do not press Create after a restart.
3. On Android, run the platform curve probe separately. It generates temporary AndroidKeyStore curve keys, runs operations against Rust and deletes those aliases. Software-provider availability and non-exportable platform support are separate outcomes.
4. With a closed DB, schedule lock reads and immediately lock the screen. Repeat with a held DB. Record the state on each resulting line: execution delayed until unlock is not proof of refusal while locked. iOS can suspend the app before the timer fires. Repeat reboot/before-first-unlock scenarios through a suitable device test setup; an interactive app launched after unlock cannot establish pre-unlock behavior. The harness does not claim those scenarios automatically pass.
5. Close held DB, delete the wrapping key only, then run reopen. It must refuse and retain ciphertext; no replacement key may be created. Reset explicitly and create again. Separately exercise OS key invalidation (for example the documented device credential lifecycle on a dedicated test device) and record the exact trigger rather than treating manual deletion as equivalent.
6. Exercise supported backup and device-transfer restore workflows with the fixture present. Android uses `noBackupFilesDir`, `allowBackup=false`, legacy full-backup disable and explicit cloud/device-transfer exclusions; iOS excludes the complete-protection application-support folder and uses a device-only wrapping key. Inspect restored artifacts and attempt reopen. Configuration flags alone do not prove exclusion. Keep database sidecars and envelope in the exclusion check.
7. Uninstall/reinstall and restore on the same and another test device as available. Record whether files or key entries remain and how mismatches refuse access. An orphaned iOS key may require explicit Reset; never claim continuity after reset.

## Evidence states

Mark every scenario tested/pass, tested/fail, unavailable, or not run with its reason. Hardware status, actual backup exclusion and lock behavior remain separate from build/test success. A failed or missing criterion blocks MC-005 completion and merge. Separate code review is required; it does not stand in for the later independent security assessment.
