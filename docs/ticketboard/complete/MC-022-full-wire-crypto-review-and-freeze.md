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

User-approved assessment remediation scope, 2026-09-16: `src/core/src/organizer.rs`, `src/core/src/storage.rs`, `src/core/src/friends.rs`, `src/core/src/ingress.rs`, `src/core/src/text.rs`, and clarification-only `docs/decisions/MC-008-crypto-contract.md`, solely for F-01/F-02/A-01 under the [approved remediation proposal](../../testing/MC-022-assessment-remediation.md). No wire/dependency change is authorized.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Run primitive reference vectors and negative cases against the selected construction and canonical encodings.
- Verify vectors through Kotlin and Swift bindings and an independent reference where available; distinguish shared-core parity from independent cryptographic review.
- Obtain independent crypto review of the construction and transcripts before declaring the full wire stable; record findings and disposition.

## Exit criteria

- [x] MC-019/020/021 provide real-verifier ingress invalid-first/valid-second and authenticated-replay evidence, including failure, eviction, concurrency and budget-available recovery; pending-state fixtures alone cannot satisfy this full-wire gate.
- [x] All required forgery, replay, binding, key lifecycle and credential-recovery checks pass in the implemented automated/native scope; separately owned physical acceptance and independent assessment are not claimed.
- [x] Independent review findings affecting the protocol are resolved, with evidence linked.
- [x] Every v1 wire/QR format is frozen and versioned; no undocumented field remains.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main. (Recorded before merge; PR #26's merge result establishes effective completion.)

## Potential fallbacks

- If independent review is unavailable, keep the full-wire freeze and public crypto release blocked.
- If review requires a layout change, update vectors/spec and reopen dependent integrations before release.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Started from main `835a7966f4edb2dead99f5768ec24b75059a337c`, after PR #25 completed MC-021. MC-020 and MC-021 hard dependencies are complete. The initial test-only revision left production unchanged; the approved assessment follow-up below changes only the scoped acceptance/storage owners. Root manifests/lockfile, native application code and workflow configuration remain unchanged.

The [construction review packet](../../testing/MC-022-crypto-review-packet.md) inventories formats, executable checks and security limitations. The separately assigned automated assessor has supplied [its report](../../testing/MC-022-assessment/report.md); no external human certification is claimed. Its findings are resolved by the separately archived retest below; completion/full-wire freeze becomes effective only on reviewed squash merge.

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
- Review/PR: [PR #26](https://github.com/wickesjon/meshChat/pull/26). Findings are resolved by the separate retest. Final metadata review and squash merge make completion effective.
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

### Separate assessment and approved remediation (2026-09-16)

The preceding record describes the pre-assessment revision. At the user's request, a fresh separate AI worker `/root/assess_mc022` assessed the complete construction/transcripts and integration at `ad87cf890459cd43e1e10991085440ebd6d48cb9`, not only the PR diff. The original report and supporting inventories/probes are archived in [MC-022-assessment](../../testing/MC-022-assessment/README.md). Its recommendation was to keep the gate blocked. The user approved the [additional remediation paths and behavior](../../testing/MC-022-assessment-remediation.md). MC-023's unfinished work is separately preserved on its branch at `559dce8`.

- F-01: public signed-message ledger reads now check both legacy directions and new tombstones use canonical direction zero. History direction and DM direction-qualified replay identity are preserved; no schema migration or deletion is used. Conflicting legacy variants refuse new effects transactionally.
- F-02: otherwise valid signed organizer text remains authenticated when its pin exceeds credential/root lifetime; only pin authority is suppressed. Outgoing construction still refuses invalid pins.
- A-01: shared raw-text validation is mandatory at friend/organizer authentication acceptance and unsigned clear ingress acceptance. Signed bytes remain unchanged. Native display normalization/badge/confusable handling remains required.
- A-02: native key-loading/QR work accounting and provider lock/reset invalidation remain explicit integration obligations, not completed device evidence.

New regressions first failed on the assessed revision for replay, pin handling and both signed text owners. Corrected targeted friend/organizer/ingress tests and 13 storage-policy tests pass, including legacy conflicting rows, rollback, history deletion/reopening, retained DM direction separation, eight forbidden-character cases in both text fields and preservation of valid multibyte/non-normalized signed originals. The old test expecting rejection of an entire valid post for excessive pin expiry is corrected to the normative text-preserved/pin-suppressed result. Android/Swift SQLCipher checks now include legacy public replay and DM direction separation; execution at the remediation revision and independent assessor retest remain pending.

### Remediation validation and peer review

Remediation source: `695fbc44e472e1f6a5664c9ec0cb3b8a0cc8f792`. The local tool versions above apply. The following checks pass on that source; earlier counts describe the historical candidate only:

- `cargo test --workspace --all-features --locked` and its `--release` equivalent: 151 tests each.
- `cargo test --manifest-path tests/integration/Cargo.toml --locked` in debug/release: 63 tests each; simulator manifest in debug/release: 40 tests each.
- Core all-feature release build, core/facade formatting and all-target clippy with `-D warnings`, 13 shared storage-policy checks, reference reproduction and unchanged dependency-package checks pass.
- Refreshed cargo-deny advisories, bans, licenses and sources pass; only existing unmatched license allowances warn.
- `python -B tests/integration/crypto/run_native.py kotlin`: both generated-binding tests pass, including corrupt-input refusal and valid-input recovery. The printed assertion panics are deliberately caught test-facade errors, not uncaught failures.

Logs are retained under `.work/assessments/mc022/` (`core-*-after.log`, `facade-*-after.log`, `sim-*-after.log`, `storage-after.log`, `deny-after.log`, and `kotlin-after.log`). A local Android instrumentation compilation attempt could not start its relevant compile because the Linux-built SQLCipher AAR was absent from the Windows cache. This is unavailable evidence, not a source failure or passing native test; the hosted native run supplies that check.

The separately assigned `gpt-5.6-terra` worker `/root/review_mc022_remediation`, medium reasoning, completed review of exact revision `695fbc44e472e1f6a5664c9ec0cb3b8a0cc8f792` and reported **no implementation blockers**. It checked public legacy replay compatibility, retained DM direction separation, pin suppression with signed-text retention, authenticated/unsigned raw-text acceptance boundaries, rollback/recovery regressions and the diff. It read the recorded core/storage logs; it did not claim independent reruns. Its remaining gates were native SQLCipher execution on this revision and the separate assessor retest. This returned review is recorded here as the repository workflow permits; automatic approval review rejected its attempt to post a GitHub COMMENT, so no posted review URL or formal GitHub approval is claimed. This peer review is separate from the construction assessment.

[Remediation CI run 35167822797](https://github.com/wickesjon/meshChat/actions/runs/35167822797) tests this exact source. Its Rust and ticketboard jobs pass. The [iOS job 105032764433](https://github.com/wickesjon/meshChat/actions/runs/35167822797/job/105032764433) passes all native builds, Swift FFI/curve checks, SQLCipher create/reopen/checks/key-loss/reset phases and the Swift public-vector facade under Xcode 16.4/Swift 6 mode. The log explicitly records `MC-022 Swift SQLCipher public replay and DM direction separation passed` and `Swift public crypto vectors and corruption/error parity passed`. Test wrapping and host execution do not certify device protection. The subsequent Android result and assessor retest are recorded below.

### Independent retest disposition

The separate assessor completed its [retest report](../../testing/MC-022-assessment/retest-report.md) on the exact remediation revision above. F-01, F-02 and A-01 are resolved, and it found no new blocking construction defect. It independently ran eight additional probes (including 156 signed forbidden-code-point nickname cases), 63 committed integration tests and 13 storage checks. It verified all 238 snapshot files against their explicit Git objects with documented CRLF normalization. The unchanged report and original bytes/probes/logs are retained with verified hashes in the [assessment evidence archive](../../testing/MC-022-assessment/README.md).

This separately executed automated construction/transcript assessment fulfills the user's requested assessor role, independently of routine Terra PR review. No human firm, formal proof, complete dependency audit or device certification is claimed. A-02 and native display/egress obligations remain explicit in the [freeze record](../../testing/MC-022-full-wire-freeze.md); physical MC-025/027/043/044, integrated MC-037 and release gates remain unchanged. The required native executions passed as recorded above and below.

### Final native result and conditional completion

The [Android job 105032764441](https://github.com/wickesjon/meshChat/actions/runs/35167822797/job/105032764441) also passed at `695fbc44e472e1f6a5664c9ec0cb3b8a0cc8f792`: application/BLE/security builds and lint, generated Kotlin vector tests, packaged native-library/alignment checks, and actual security/storage instrumentation on the isolated API 29 emulator (4096-byte pages). Its log records MC-005 create/reopen/key-loss success and MC-018 create/reopen/checks/key-loss/reset success. The checks phase executes the added legacy public replay/conflict and retained DM direction-separation assertions against SQLCipher. No device protection or radio evidence is inferred. All four jobs in run 35167822797 succeeded.

The [full-wire freeze record](../../testing/MC-022-full-wire-freeze.md) names the combined versioned grammar, unchanged compatibility rules, finding dispositions and remaining integration/device/release obligations. Final changes after the tested/reviewed production revision are documentation and attributable evidence only; no source, tests, dependencies, generated bindings or workflow changes invalidate those results. Ticketboard/default, its 12 regressions and whitespace checks pass after the status move. The completed folder is staged solely for the final reviewed squash commit; it becomes effective only when that commit lands on main. The merge checkbox remains unchecked in this pre-merge record. Terra's metadata review of `e5b7f59f9d09d27bc9dcbf63043958df101d458e` identified that premature checkbox and a stale pending-native sentence; both are corrected here. Final follow-up review is recorded in PR #26 before merge.
