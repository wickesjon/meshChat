---
id: "MC-022"
title: "Full-wire crypto review and freeze"
depends_on: ["MC-020","MC-021"]
kind: "gate"
branch: "ticket/MC-022-full-wire-crypto-review-and-freeze"
---

# MC-022 — Full-wire crypto review and freeze

## Objective

Run primitive reference vectors and negative cases against the selected construction and canonical encodings.

## Dependencies

`MC-020`, `MC-021` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `tests/vectors/crypto/**`, `tests/integration/**`, `docs/testing/**`, `docs/mesh-chat-design.md`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Run primitive reference vectors and negative cases against the selected construction and canonical encodings.
- Verify vectors through Kotlin and Swift bindings and an independent reference where available; distinguish shared-core parity from independent cryptographic review.
- Obtain independent crypto review of the construction and transcripts before declaring the full wire stable; record findings and disposition.

## Exit criteria

- [x] MC-019/020/021 provide real-verifier ingress invalid-first/valid-second and authenticated-replay evidence, including failure, eviction, concurrency and budget-available recovery; pending-state fixtures alone cannot satisfy this full-wire gate.
- [x] All required forgery, replay, binding, key lifecycle and credential-recovery checks pass in the implemented automated/native scope; separately owned physical acceptance and independent assessment are not claimed.
- [ ] Independent review findings affecting the protocol are resolved, with evidence linked.
- [ ] Every v1 wire/QR format is frozen and versioned; no undocumented field remains.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If independent review is unavailable, keep the full-wire freeze and public crypto release blocked.
- If review requires a layout change, update vectors/spec and reopen dependent integrations before release.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Started from main `835a7966f4edb2dead99f5768ec24b75059a337c`, after PR #25 completed MC-021. MC-020 and MC-021 hard dependencies are complete. Production code, root manifests/lockfile, native application code and workflow configuration are unchanged.

The [construction review packet](../../testing/MC-022-crypto-review-packet.md) inventories the versioned formats, executable checks, security limitations and questions for the independently required assessor. No external assessment has been supplied; no independent security approval or full-wire freeze is claimed. This is a real remaining gate, not deferred physical certification. The ticket must not be completed or merged as complete while that assessment and its findings remain outstanding.

### Implementation

- `tests/integration` is a standalone, test-only UniFFI fixture library. It reuses the existing admitted production friend/DM/organizer endpoint harnesses. Foreign callers supply the committed public binary inputs. Successful checks return family/count markers; assertion failures become a typed test error.
- Test harness reference functions are shared between Rust and native callers. SQLite test filenames include each module namespace to avoid collisions when the three suites share one process. The SQLite callback double remains plaintext synthetic test evidence, not SQLCipher/hardware evidence.
- Kotlin/Swift bindings are generated into ignored `.work/mc022`; they are never committed or packaged in the applications. Kotlin uses a standalone test project with pinned Kotlin 2.2.0, JNA 5.17.0 and JUnit 4.13.2. Its invocation is wired into the existing Android JVM test source directory; Swift is invoked by the existing Mac storage check after SQLCipher phases. No shipping FFI API was added.
- Both languages pass 8 friend, 17 DM and 8 organizer inputs and require typed failure after separately corrupting a signature/ciphertext in each family, followed by successful unchanged fixtures. This is same-core binding parity. Separately, Node/OpenSSL reference generators reproduce all committed public bytes and published RFC KATs.
- `check_references.py` verifies exact Node output, rejects any new/changed dependency package in the test-only lockfile apart from its own package, and records a revision/dirty-tree/hash inventory under `.work/mc022`. Production dependency versions and root lockfile remain unchanged.

### Local validation

Windows: Rust/cargo 1.85.1; Python 3.14.4; Node 24.15.0/OpenSSL 3.5.5; JDK 17.0.15+6; Kotlin 2.2.0; Gradle 8.13; cargo-deny 0.20.2.

- `cargo test --workspace --all-features --locked`, debug and release: 144 tests each, including actual provider/ingress verification and persistent replay/transaction regressions.
- `cargo test --manifest-path tests/integration/Cargo.toml --locked`, debug and release: all 57 combined friend/DM/organizer cases pass. Test-facade release build passes.
- `cargo test --manifest-path tests/simulator/Cargo.toml --locked`, debug and release: 36 tests each pass.
- Core and test-facade clippy (`--all-targets`, core `--all-features`, `--locked`, `-D warnings`), both formatting checks, ticketboard/default + 12 validator tests and diff check pass.
- `python -B tests/integration/crypto/check_references.py`: all three reference fixtures reproduce exactly, and test dependency packages match the production lockfile. Public RFC Auth KAT and expected recipient-fabrication limitation reproduce; organizer RFC8032 KAT reproduces.
- Refreshed cargo-deny advisory/bans/license/source checks pass, with existing unmatched license allowance warnings only. Sandbox network refusal on the first advisory refresh was resolved with the authorized network-enabled check.
- Direct `python -B tests/integration/crypto/run_native.py kotlin`: passes generated Kotlin compilation, positive vectors and three typed corruption errors/recovery. Initial setup failures (bindgen Cargo lookup and network-unavailable plugin resolution) were corrected/retried. The outer Android task initially exposed three nullable platform API warnings; those are corrected. Final `:app:testDebugUnitTest --tests org.meshchat.ffi.WireVectorsTest` passes through the real Android JVM test task, including its nested generated-binding tests.

Logs and generated input inventory remain in `.work/mc022`. Corrected-source native Swift execution and Terra peer review passed as recorded below. Physical-device and independent assessment requirements remain unchanged.

## Review and merge

- Branch: `ticket/MC-022-full-wire-crypto-review-and-freeze`.
- Review/PR: [PR #26](https://github.com/wickesjon/meshChat/pull/26). External construction assessment remains unsupplied; do not merge as complete.
- Squash commit title: `MC-022: Full-wire crypto review and freeze`.
- Completion becomes effective only when the reviewed squash commit lands on main.

### Automated peer review correction

Terra medium reviewed source `4fb33c84157374a982b2f00f4b732730b9583341` and posted [its findings](https://github.com/wickesjon/meshChat/pull/26#issuecomment-5704376205). It found one P2: checking only fixture-map counts allowed an expected DM vector to be replaced by a renamed invalid input, silently omitting a positive case. The test facade now requires the exact expected key set for every family. A Kotlin regression reproduced the old false success by renaming `chat_2_1` and corrupting its ciphertext at unchanged count; Swift includes the same negative case. This corrects test coverage, not production crypto.

After the fix, all 57 combined harness tests pass in debug and release; release build, clippy, formatting, reference reproduction/lock-package checks, board validation and diff checks pass. Kotlin's two native tests (positive/corruption/recovery and same-count renamed input) pass through the actual Android test task with `--rerun-tasks` (22 tasks executed).

Terra follow-up reviewed exact source `60d4dcbf9b740b3101fc953f803e2c8f27ea8d16` and reported no remaining implementation blockers in [its posted follow-up](https://github.com/wickesjon/meshChat/pull/26#issuecomment-5704432110). It reran all 57 Rust facade tests, both generated Kotlin native tests, reference reproduction, board and diff checks. This is automated peer review, not the independently required construction assessment.

### Hosted native evidence and applicability

[CI run 35149668845](https://github.com/wickesjon/meshChat/actions/runs/35149668845) covers corrected source `60d4dcbf9b740b3101fc953f803e2c8f27ea8d16`. Ticketboard and Rust jobs passed. The [iOS job 104974718186](https://github.com/wickesjon/meshChat/actions/runs/35149668845/job/104974718186) completed successfully: device/simulator library builds, unsigned app/BLE/security builds, Swift FFI/CryptoKit checks, SQLCipher lifecycle phases and the added Swift vector facade. Its log explicitly records `Swift public crypto vectors and corruption/error parity passed` after all three corruption checks, the renamed-input refusal and final positive recovery. Xcode 16.4/Swift 6 mode was used; this is Mac/native evidence, not physical-device evidence.

The Android job passed Rust/Kotlin binding generation, app build/lint/JVM tests (including the new vector runner), packaged library/alignment checks and BLE probe build/lint. The remaining unchanged security-probe/emulator steps were still running when this record was written; no successful full Android job/run is claimed. Under the local-validation policy, those unchanged steps are supplemental for this test-only PR: no production Rust, native application/security adapter, native dependency/package or workflow configuration changed. The actual changed Android JVM hook and Mac storage/vector script have executed successfully. Any later relevant source failure must still be investigated; no runner/test failure is waived.

Final metadata only records these results and the blocker; it does not change source, test runners, libraries or lockfiles. Ticketboard/default validation, its 12 regressions and diff check are rerun, and the metadata receives Terra review. Independent construction/transcript assessment remains unsupplied, full-wire freeze remains blocked, and this ticket stays in `inreview/`. No squash merge or effective completion is authorized by automated success alone.
