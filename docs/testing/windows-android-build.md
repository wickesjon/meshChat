# Android core and app builds from Windows

MC-046 adds Windows host support to the Android path in `src/core/build_bindings.py`. MC-028 replaces the skeleton with the protected channel application. This guide covers Rust libraries/bindings and local app build, lint and JVM FFI checks. It does not claim emulator execution, physical-device evidence or iOS validation.

## Prerequisites and isolation

- Rust/cargo 1.85.1 and rustup on the current process PATH. This checkout has its toolchain under `.work/rustup/toolchains/1.85.1-x86_64-pc-windows-msvc/bin` and rustup under `.work/tools`.
- NDK **27.3.13750724 (r27d)**, installed through Android Studio's SDK Manager. `ANDROID_HOME` must name its SDK root, not the NDK directory. The user's installed SDK was verified at `C:\Users\wicke\AppData\Local\Android\Sdk`; the compiler is read without changing that installation.
- Temurin JDK **17.0.15+6**, Gradle **8.13** via the checked-in wrapper, SDK platform **36** and Windows Build Tools **35.0.0**, matching the CI workflow. Existing checkout copies are under `.work/tools` and `.work/android-sdk`. Provision missing SDK components into that repository-local SDK before Gradle; do not silently download/update an external SDK. `android.builder.sdkDownload=false` remains in effect.
- Keep writable Rust/Gradle/Android/Java/temp caches and generated files inside this repository. The commands below change only the current PowerShell process environment, not machine settings. Existing ignored `.work` tools are local prerequisites, not committed project files.

Android's [NDK build-system guide](https://developer.android.com/ndk/guides/other_build_systems) describes the `windows-x86_64` host directory and direct Clang target selection. Windows uses `clang.exe` with `--target=aarch64-linux-android29` or `--target=x86_64-linux-android29`, avoiding `.cmd` argument forwarding. Cargo's linker path is a single environment value, including any spaces. Linux/macOS retain their existing target-specific compiler wrappers. Both ABIs retain 16 KB maximum/common page-size link flags.

## Build the Rust Android libraries

Run from the repository root in PowerShell. Substitute your installed SDK root if different. Ensure the listed repo-local tools have been provisioned first.

```powershell
$meshRoot = (Get-Location).Path
$env:CARGO_HOME = "$meshRoot/.work/cargo"
$env:RUSTUP_HOME = "$meshRoot/.work/rustup"
$env:CARGO_TARGET_DIR = "$meshRoot/target"
$env:TEMP = "$meshRoot/.work/tmp"
$env:TMP = $env:TEMP
$env:PATH = "$meshRoot/.work/tools;$meshRoot/.work/rustup/toolchains/1.85.1-x86_64-pc-windows-msvc/bin;$env:PATH"
$env:ANDROID_HOME = 'C:\Users\wicke\AppData\Local\Android\Sdk'
New-Item -ItemType Directory -Force "$meshRoot/.work/tmp" | Out-Null
rustc --version
cargo --version
python -B src/core/build_bindings.py android
if ($LASTEXITCODE -ne 0) { throw 'Android Rust build failed' }
python -B src/core/build_bindings.py android --security-probe
if ($LASTEXITCODE -ne 0) { throw 'Security-feature Rust build failed' }
```

The script installs the pinned toolchain's Android targets through rustup, builds the host binding generator, and writes normal Kotlin/Swift and Android libraries to `.work/ffi`. Security-feature artifacts go to `.work/security-ffi` and `.work/security-target`. Building that Rust feature does not rebuild SQLCipher or test its Android lifecycle.

Do not set overriding `RUSTFLAGS` or `CARGO_ENCODED_RUSTFLAGS` when running these commands: Cargo gives those precedence over the target-specific flags, which would remove the selected Android target/page-size requirements. APK checks below independently verify the shipped ELF architectures and alignment.

## Build and check the app

The production app now requires two aligned native dependency AARs. While
`ANDROID_HOME` still points to the installed NDK, run
`python -B src/android/ui/build_graphics.py`. This builds the pinned matching
AndroidX graphics source on Windows and checks both ELF architectures and 16 KB
LOAD/RELRO alignment. Its output is
`.work/ui-graphics/graphics-path-1.0.1-aligned.aar`.

SQLCipher still needs the Linux build described below. To run local app checks,
obtain `security-sqlcipher/sqlcipher-4.17.0-aligned.aar` from this repository's
trusted `mc028-android-evidence` CI artifact, verify the downloaded ZIP against
the artifact's SHA-256 metadata, and place that one file under
`.work/security-sqlcipher/`. Record the source run, revision and artifact digest.
Do not rename an upstream unaligned AAR or substitute an API jar for packaging.
The package checks below remain mandatory even with a verified artifact. See
[Android channel validation](android-channel-ui.md) for provenance and limits.

Keep using the same repository-root PowerShell session. Gradle uses the existing repository-local SDK instead of the external SDK used only as compiler input above.

```powershell
$env:JAVA_HOME = "$meshRoot/.work/tools/jdk-17.0.15+6"
$env:GRADLE_USER_HOME = "$meshRoot/.work/gradle"
$env:ANDROID_HOME = "$meshRoot/.work/android-sdk"
$env:ANDROID_SDK_ROOT = $env:ANDROID_HOME
$env:ANDROID_USER_HOME = "$meshRoot/.work/android"
$env:JAVA_TOOL_OPTIONS = "-Duser.home=`"$meshRoot/.work/java-home`" -Djava.io.tmpdir=`"$meshRoot/.work/tmp`""
New-Item -ItemType Directory -Force "$meshRoot/.work/java-home", "$meshRoot/.work/android" | Out-Null
& ./src/android/gradlew.bat -p src/android --no-daemon :app:assembleDebug :app:assembleRelease :app:assembleDebugAndroidTest :app:lintDebug :app:testDebugUnitTest
if ($LASTEXITCODE -ne 0) { throw 'Android Gradle checks failed' }
& ./src/android/gradlew.bat -p src/android --no-daemon :app:testDebugUnitTest --rerun-tasks
if ($LASTEXITCODE -ne 0) { throw 'Fresh JVM FFI regression failed' }
python -B tests/integration/android-ui/check_apk.py src/android/app/build/outputs/apk/debug/app-debug.apk src/android/app/build/outputs/apk/release/app-release-unsigned.apk
if ($LASTEXITCODE -ne 0) { throw 'Android native packaging check failed' }
foreach ($meshApk in Get-ChildItem src/android/app/build/outputs/apk/*/*.apk) {
    & "$env:ANDROID_HOME/build-tools/35.0.0/zipalign.exe" -c -P 16 4 $meshApk.FullName
    if ($LASTEXITCODE -ne 0) { throw "APK zip alignment failed: $meshApk" }
}
python -B -m unittest discover -s tests/integration/build-tools -v
```

The JVM tests exercise the Windows host library through generated Kotlin/JNA, while packaging checks inspect Rust/JNA/SQLCipher/Compose graphics on ARM64/x86-64 in both APKs. Neither runs those libraries on an Android emulator/device. The release APK here is unsigned, not a distribution artifact.

## Remaining checks

`src/android/security/build_sqlcipher.py` rebuilds the pinned SQLCipher JNI with required page-size alignment using Unix `./configure`, `make`, Tcl and `ndk-build`. Installing the NDK alone does not port that pipeline to Windows. Full security app/APK/lifecycle CI needs a supported Linux runner or a separately reviewed portability change; do not substitute an unchecked upstream AAR or disable its alignment/security checks. Emulator startup in the current CI uses a shell script as well. These steps are not included in a passing core/app build claim.

iOS still requires the pinned Mac/Xcode environment. PR #13's shared-dependency changes require native-consumer checks on that PR's revision; Windows results from another branch do not automatically validate it. Physical radio/storage acceptance remains in the later MC-025/027/043/044 gates, and independent security assessments remain required.
