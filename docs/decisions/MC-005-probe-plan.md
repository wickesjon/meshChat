# MC-005 — Protected-key and encrypted-store probe

State: implemented feasibility harness, partial automated evidence, 2026-09-11. Physical acceptance, final provider selection and security review remain pending. The MC-004 gate replacement does not change these requirements.

## Candidate interface and protection model

MC-017 should use opaque key handles and explicit capabilities: generate, public key, sign, agree, delete/reset and protection metadata. Errors must distinguish unavailable/locked, invalidated/missing, invalid input and provider failure. A provider never promises extraction of a non-exportable private key. The spike exposes synthetic curve functions and native fixture controls, not a production identity provider. Final provider selection remains blocked on device evidence.

| Candidate | Implemented probe | Limits still to establish |
|---|---|---|
| Android platform-operated curves | AndroidKeyStore Ed25519/XDH generation, private-key exportability/hardware metadata, actual sign/agreement checked against Rust | Algorithm/provider availability and lock policy on API 29 and newer hardware; an unavailable result is not a pass |
| Android wrapped software curves | AndroidKeyStore AES-256-GCM wrapping key with unlocked-device requirement; authenticated versioned envelope in noBackupFilesDir; Rust curve operations after unwrap | Hardware support is measured, not assumed; native software interoperability depends on OS provider availability |
| iOS wrapped software curves | Secure Enclave P-256 ECIES wrapping with WhenUnlockedThisDeviceOnly access; CryptoKit/Rust Ed25519 and X25519 interoperability; complete file protection | Curve25519 is software, not Secure Enclave P-256; physical iPhone required for wrapping; no simulator/software wrapping fallback |

Each fresh fixture contains 96 random bytes: independent 32-byte signing seed, agreement seed and database passphrase. SQLCipher uses its normal key derivation; this is not a custom encryption format for production databases. No real identity or message can be imported. Software keys and database keys enter process memory; zeroization is best effort across Swift/Kotlin/UniFFI copies and native provider objects. At-rest protection does not protect against a compromised running process. Held database mode deliberately retains an open database across lock to measure that limitation.

Creation refuses retained files or an existing wrapping key. Reopen never creates a missing key/database. Key deletion retains ciphertext and requires explicit fixture reset. A partial creation also requires explicit reset; it is never recovered through plaintext or silent replacement. Reports contain only predefined operation names, statuses, provider/OS metadata, cipher version and lock state. Exception messages, private/shared bytes and database content are not reported. Reports are bounded and memory-only, with explicit clipboard copy.

## Primary platform inputs

- [Android Keystore](https://developer.android.com/privacy-and-security/keystore): non-exportable operations and conditional hardware support. The probe reports actual KeyInfo metadata. [Android key generator source](https://android.googlesource.com/platform/frameworks/base/+/80a664262667cf14ee1ae52ab7c53abc26e17d1e/keystore/java/android/security/keystore2/AndroidKeyStoreKeyPairGeneratorSpi.java) informs curve-specific parameters; availability still requires runtime evidence.
- [Apple Secure Enclave guidance](https://developer.apple.com/documentation/security/protecting-keys-with-the-secure-enclave), [SecureEnclave.P256](https://developer.apple.com/documentation/cryptokit/secureenclave/p256), and [CryptoKit Keychain guidance](https://developer.apple.com/documentation/cryptokit/storing-cryptokit-keys-in-the-keychain) distinguish protected wrapping from software Curve25519. Minimum iOS remains 15.
- [RFC 8032 section 7.1](https://www.rfc-editor.org/rfc/rfc8032#section-7.1) and [RFC 7748 section 6.1](https://www.rfc-editor.org/rfc/rfc7748#section-6.1) supply exact Rust vectors. [RFC 8410](https://www.rfc-editor.org/rfc/rfc8410) defines the fixed public/private encodings used for Android interoperability; unexpected public encodings are rejected.

## Dependencies and builds

The user approved adding Cargo.lock and .github/workflows/ci.yml to MC-005 scope on 2026-09-11. The optional security-probe feature pins ed25519-dalek 2.2.0, x25519-dalek 2.0.1, zeroize 1.8.1 and getrandom 0.4.3. Default foundation builds do not enable the probe. Cargo.lock preserves existing versions. Generated probe bindings/artifacts are isolated in .work/security-ffi and .work/security-target, preventing the parent skeleton from accidentally linking probe exports.

Android uses SQLCipher 4.17.0 and AndroidX SQLite 2.5.2, with existing JNA 5.17.0. The upstream 4.17.0 AAR SHA-256 is `44fc40c33d1de597c8339072a71fa0ff20e12d01ab352d6abe4ad5df668ead94`. Its prebuilt library fails the required 16 KB RELRO check (the inspected 4.19.0 artifact also fails, so it was not adopted). [Android's alignment guidance](https://developer.android.com/guide/practices/page-sizes#check-relro) requires the RELRO end to align along with LOAD segments. The repository rebuilds only JNI from upstream revision `0725b962ffb60b00460b0e315bc632a543b399e7` and its pinned submodules, adding `common-page-size=16384` to the existing max-page-size flag with NDK 27.3.13750724. Java/classes and resources remain from the checksummed AAR. The generated AAR contains only arm64-v8a/x86_64. CI must validate the resulting APKs; rebuilding is not itself evidence of compatibility. This is a necessary native build implication of the spike, not a SQLCipher algorithm change.

iOS uses the [official SQLCipher Swift package](https://github.com/sqlcipher/SQLCipher.swift/tree/4.17.0), pinned at `c85425b80b8c9f0a1ceb4f72fa174e2b688181ba` (4.17.0). Its manifest pins the binary checksum `dd5a650346c1ba9933d6ba179f8844e03e4a075b3dd3a892796149864cd9ae57`. SQLITE_HAS_CODEC is enabled; the probe imports SQLCipher, keys through sqlite3_key and confirms a nonempty cipher_version before using the fixture. Unsigned simulator/device Debug/Release builds do not imply installation or Secure Enclave validation.

SQLCipher uses BSD-style licensing with attribution requirements; Android also contains upstream Android/SQLite/LibTomCrypt components. The original AAR metadata/resources are retained when replacing native libraries. [Android license](https://github.com/sqlcipher/sqlcipher-android/blob/v4.17.0/LICENSE) and [Apple license](https://github.com/sqlcipher/SQLCipher.swift/blob/4.17.0/LICENSE.md) are dependency inputs; release attribution/distribution review remains necessary before shipping. No commercial package, purchase or distribution is part of this spike.

## Evidence and required acceptance

Local Rust 1.85.1 all-feature Clippy and Debug/Release tests pass: four existing foundation tests plus three curve/vector/input tests. Cargo-deny 0.20.2 advisories, bans, licenses and sources pass (only existing unused-license warnings). Host Kotlin/Swift binding generation passes. Android Debug/Release Kotlin compilation and lint pass against upstream 4.17.0; the final rebuilt AAR and iOS builds await hosted CI. Workflow syntax validation passes. These are automated results, not security/device acceptance.

Follow the [bench runbook](../../tests/bench/security/README.md). Retain device/OS, exact app revision, wrapping policy, sanitized operation results and observed backup artifacts under docs/decisions. All rows below remain untested on actual phones:

| Scenario | Required evidence |
|---|---|
| Minimum OS curves | Android API 29 and iOS 15 core/native results and capability failures, plus current OS provider/hardware metadata |
| SQLCipher lifecycle | Create synthetic integer row, close/force-stop/relaunch, reopen/read; wrong key fails a real data/schema read; correct key still works |
| Backup | Cloud and supported device-transfer/restore attempt excludes folder, database, journals/WAL and wrapped secrets; inspect artifacts, not just manifest flags |
| Lock/reboot | Closed and intentionally held DB before/after lock, before first unlock and after reboot; record actual lock/protected-data state at execution and delayed/suspended probes |
| Key loss/reset | Delete wrapping key while retaining encrypted fixture, reopen refuses, explicit reset creates a new fixture; test OS invalidation separately from deletion |
| Uninstall/restore | Observe platform key/file persistence differences and mismatch handling without asserting identity continuity |

Device/Mac/signing inventory is unknown. The final protection model and all MC-005 exit criteria remain open. Secure persistence that cannot be demonstrated blocks persistent identities/DM release. No physical result or independent security assessment is inferred from this code or CI.
