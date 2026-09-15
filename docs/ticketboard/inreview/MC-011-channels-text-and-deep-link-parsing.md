---
id: "MC-011"
title: "Channels, text and deep-link parsing"
depends_on: ["MC-009"]
kind: "core"
branch: "ticket/MC-011-channels-text-and-deep-link-parsing"
---

# MC-011 — Channels, text and deep-link parsing

## Objective

Implement canonical channel normalization, fixed word lists, public IDs and glyph mappings with committed vectors.

## Dependencies

`MC-009` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/core/**`, `tests/vectors/channels/**`, `tests/integration/links/**`, `tests/fuzz/**`, `Cargo.lock` (user-approved dependency integration only).

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Implement canonical channel normalization, fixed word lists, public IDs and glyph mappings with committed vectors.
- Parse channel, friend, event and staff links into inert typed proposals; validate host/path, encoding, sizes and bundle versions without changing trust/subscriptions.
- Separate wire validation from display sanitization; define byte-aware nickname/text handling including emoji and accessibility-relevant Unicode behavior.

## Exit criteria

- [x] All 8,000 triples have deterministic IDs; collisions are detected and resolved by an explicit decision rather than assumed absent.
- [x] Malformed URLs/bundles are rejected and every valid link requires a native confirmation before state mutation.
- [x] Boundary tests cover multi-byte lengths, confusables, bidi controls and normalization without altering signed bytes.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If a newer word or bundle version is unknown, return an update-needed state rather than joining a different channel.
- If sanitization would alter authenticated content, retain authenticated original bytes and render a safe separate representation.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Started from main `bee682ae62b04cc360cf44fbe878fa7404178bf5` after MC-010 squash; MC-009 is complete. The initial dependency-scope proposal preceded production work. A read-only standard-library enumeration of the design's 20×20×20 fixed triples found 8,000 unique SHA-256 prefix IDs, no private/private collisions and no collisions with the three public IDs. This is preliminary arithmetic evidence, not committed implementation/vector coverage.

### Approved root lockfile scope extension

The original permitted paths excluded root `Cargo.lock`, but proper SHA-256 channel hashing and the specified NFC/TR39 behavior need direct dependency declarations and a resolved lockfile update. Do not copy a hash/Unicode implementation to evade this boundary or replace full normalization/confusable handling with partial tables. Approved scope addition: **`Cargo.lock`**, only for MC-011's exact dependency integration. `src/core/Cargo.toml` is already permitted. No dependency/security-policy exception is proposed.

The approved direct dependencies are `sha2 = "=0.10.9"`, `unicode-normalization = "=0.1.24"`, `unicode-security = "=0.1.2"`, and normal-build availability of the already pinned `zeroize = "=1.8.1"` for private staff-provisioning proposal lifetime. SHA-256 and zeroize already exist in the locked graph. The [normalization crate](https://docs.rs/unicode-normalization/0.1.24/unicode_normalization/) supplies NFC and combining-mark tables; [unicode-security](https://docs.rs/unicode-security/0.1.2/unicode_security/) supplies the TR39 skeleton operation. Both chosen NFC/confusable tables report Unicode 16.0.0. An initial normalization 0.1.25 experiment reports Unicode 17.0.0; 0.1.24 is proposed to align those two tables explicitly, not silently mix data versions. A future table update requires reviewing expected text/security vectors.

An isolated repo-local experiment resolved only these five new registry packages (production manifest/lockfile untouched):

| Package | Version | Crates.io checksum |
|---|---|---|
| unicode-normalization |0.1.24|`5033c97c4262335cded6d6fc3e5c18ab755e1a3dc96376350f3d8e9f009ad956`|
| unicode-security |0.1.2|`2e4ddba1535dd35ed8b61c52166b7155d7f4e4b8847cec6f48e71dc66d8b5e50`|
| unicode-script |0.5.8|`383ad40bb927465ec0ce7720e033cb4ca06912855fc35db31b5755d0de75b1ee`|
| tinyvec |1.13.2|`4cf0ded5c4e56918d8f8a339e1bb67d038d3bc6d144ac407904015ba2e4cde9b`|
| tinyvec_macros |0.1.1|`1f3ccbac311fea05f86f61904b462b55fb3df8837a366dfc601a0161d0532f20`|

The disposable workspace used the current core with security-probe/bindgen features and those exact direct dependencies, seeded from current Cargo.lock. Rust/cargo 1.85.1 Windows MSVC compiled and ran public synthetic checks for composed-accent NFC, Cyrillic/Latin confusable skeleton equality and the literal General channel ID. `cargo-deny 0.20.2 --all-features --locked --config src/core/deny.toml check` passes advisories/bans/licenses/sources with fresh advisory data. No new duplicate-version allowances are needed. The probe is non-publishable like this private workspace; an initial missing-private-manifest setting caused only its own license gate to fail, then was corrected before the passing run. Sources and resolved checksums above are available in `.work/mc011-dependency/`; generated outputs are ignored.

Dependency changes are covered by the native-consumer row of the approved local-validation policy. This scope approval alone would not waive those checks. The current Windows environment has no Mac/Xcode for required iOS validation; hosted Actions provisioning is unavailable under the previously recorded account limitation. That remains a separate merge prerequisite to resolve through available native runners or an explicit validation-policy decision. No host probe is described as native evidence, and no hardware/independent-security gates are waived.

The user approved adding root `Cargo.lock` to MC-011 scope on 2026-09-11. The exact dependency integration above is authorized; native validation remains required. Implementation and review subsequently completed as recorded below; the ticket remains inreview and unmerged.

### Implementation and host validation

Implemented `channel.rs`, `links.rs` and `text.rs` with the approved lockfile changes. No old package version/checksum changed; exactly the five listed registry packages were added. NFC/TR39 tables remain Unicode 16.0.0 and the duplicate/advisory/license/source rules remain unchanged. `zeroize` is available in normal builds for bounded staff-proposal storage.

The [committed vector and integration guide](../../../tests/vectors/channels/README.md) records precise choices and caller obligations. All 8,003 canonical names/IDs/glyphs match independent Python SHA-256 vectors with no collisions. Every private channel round-trips both link forms. Fixed v1 private glyph indexing is unsigned big-endian channel ID modulo nine. Thirty additional independently encoded public URI fixtures and seven integration tests cover authority/routes, exact lengths, canonical binary/percent encoding, inert proposals, original display claims, staff seed storage and Unicode byte/security boundaries. `UnconfirmedProposal` has no state-changing operation; native confirmation and the relevant crypto/import gates remain mandatory for consumers. Original authenticated text stays untouched; a separate display copy handles NFC, forbidden characters, combining marks, badge filtering and the explicit native single-line/ellipsis requirement.

A 100,000-mutation link/text target uses public-only persisted seeds and checks parsing, original-byte preservation, NFC and bounded rendered output. Staff test seed/URL material is generated only at runtime in zeroizing storage and not logged or persisted. This is mutation smoke evidence rather than a coverage-guided or native rendering claim.

Local host checks pass with Rust/cargo 1.85.1 Windows MSVC and Python 3.14.4: formatting; all-target/all-feature locked clippy with `-D warnings`; all-feature locked debug and release tests (34 each); release build; locked offline resolution; exact approved lockfile addition/checksum comparison; channel/public-URI vector regeneration check; ticketboard write/default and 12 unit tests; `git diff --check`. Initial Cargo index access was unavailable in the sandbox; the approved already-cached graph resolved offline. cargo-deny 0.20.2 subsequently refreshed advisories with permitted network access and passes advisories/bans/licenses/sources. `python -B src/core/build_bindings.py host` also builds and generates both Kotlin and Swift binding artifacts successfully. That is host binding generation, not native compilation.

### Native dependency validation and remaining merge blocker

The Android/iOS native-consumer checks remain required because this change adds shared dependencies. Initially Windows Android compilation was unsupported. After MC-045/046 merged, main `5f29811aabc6a5ef90a56731054511030954d073` was integrated without conflicts as `63a2d975ddf452e1e2f7a71bcdb021fc2b034a8e`. MC-011 channel/text/link code and its approved dependencies are unchanged from the original reviewed implementation; the integration supplies the separately reviewed Windows tooling and scheduling documentation.

On that integrated MC-011 revision, both normal and `--security-probe` Rust Android builds pass for ARM64 and x86-64 with NDK r27d 27.3.13750724, Clang 18.0.4 and Rust/cargo 1.85.1. App debug/release builds, lint and a fresh JVM Kotlin/JNA regression pass with Temurin 17.0.15+6, Gradle 8.13, SDK 36 and Build Tools 35.0.0: `src/android/gradlew.bat -p src/android --no-daemon :app:assembleDebug :app:assembleRelease :app:lintDebug :app:testDebugUnitTest --rerun-tasks` executes all 97 tasks. The fresh FoundationTest report has one test and zero failures/errors/skips. Both completed APKs pass `tests/integration/ffi/check_android_apk.py` and `zipalign.exe -c -P 16 4`; both security-feature Rust libraries pass direct ELF machine/16 KB LOAD/RELRO checks. Logs are `.work/mc011-windows-android.log`, `.work/mc011-windows-security-rust.log` and `.work/mc011-windows-gradle.log`; commands and isolated paths follow the [Windows guide](../../testing/windows-android-build.md). Channel/URI vectors, three tooling regressions, 12 board tests, board validation and diff checks pass after integration.

This is Android cross-compilation/app packaging and host JVM FFI evidence, not emulator, physical-device or full SQLCipher/security-app lifecycle validation. The unchanged SQLCipher/emulator pipeline still needs Unix tooling. This environment still has no `xcodebuild`/`xcrun`, so required iOS validation is unavailable, not passed. No production native UI is wired by this internal module. MC-011 stays inreview and cannot squash merge until all applicable native evidence exists or the user explicitly changes the policy; MC-045's physical deferral did not waive native compilation. Independent security gates remain unchanged.

## Review and merge

- Branch: `ticket/MC-011-channels-text-and-deep-link-parsing`.
- PR: [#13](https://github.com/wickesjon/meshChat/pull/13), ready for review, unmerged.
- Separate `gpt-5.6-terra` / medium worker completed review of `fd1031fe698227a62435f554bac6e0db5e45e585` and posted [review 5185120908](https://github.com/wickesjon/meshChat/pull/13#pullrequestreview-5185120908): no actionable source findings. Completion notification was awaited without polling. This is an actual worker COMMENT review, not formal author self-approval or an independent security assessment.
- Reviewer reproduced formatting, clippy, debug/release tests (34 each), release build, independent vectors, ticketboard validation and 12 tests, diff check, and host binding generation. Cached offline cargo-deny checks passed; the reviewer's fresh advisory fetch was unavailable because GitHub could not be reached. The earlier fresh author run remains the recorded fresh-advisory evidence.
- [Hosted run 34671735191](https://github.com/wickesjon/meshChat/actions/runs/34671735191) for that revision completed with all four jobs marked failure and no job steps or logs returned. Android/iOS checks therefore provide no passing native evidence. The connector rejects the annotation endpoints, so the exact cause of this run is unverified; the previous account provisioning limitation is historical context only.
- Merge remains blocked by unavailable required native evidence, including iOS. The initial review above covers MC-011 source; subsequent main integration brings separately reviewed MC-045/046 changes, and follow-up review of the integrated revision/evidence is recorded on the PR.
- Squash commit title: `MC-011: Channels, text and deep-link parsing`.
- Completion becomes effective only when the reviewed squash commit lands on main.
