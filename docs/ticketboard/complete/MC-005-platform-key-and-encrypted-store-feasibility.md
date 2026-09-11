---
id: "MC-005"
title: "Platform key and encrypted-store feasibility"
depends_on: ["MC-003"]
kind: "spike"
branch: "ticket/MC-005-platform-key-and-encrypted-store-feasibility"
---

# MC-005 — Platform key and encrypted-store feasibility

## Objective

Probe Ed25519/X25519 use from the shared core and each platform's key APIs on minimum supported OS versions.

## Dependencies

`MC-003` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/core/**`, `src/android/security/**`, `src/ios/Security/**`, `tests/bench/security/**`, `docs/decisions/**`, `Cargo.lock`, `.github/workflows/ci.yml`, `docs/mesh-chat-design.md`, `docs/ticketboard/implementation-plan.md`, MC-017/018/034/037 ticket files and new MC-043/044 ticket files (deferral planning only).

Scope approval: the user explicitly approved adding `Cargo.lock` and `.github/workflows/ci.yml` on 2026-09-11 for the prepared Rust crypto dependencies and standalone security-probe checks. The subsequent user instruction on 2026-09-11, “let's put off that verification until later,” authorizes moving physical verification to MC-043/044 and the associated design, plan and dependent-ticket edits. The user then explicitly approved publishing these revised criteria and dependencies, completing MC-005 under them, and continuing the review-and-squash-merge flow after the concrete changes were presented. Verification is deferred, not passed or removed; independent security assessments are unchanged.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Probe Ed25519/X25519 use from the shared core and each platform's key APIs on minimum supported OS versions.
- Compare platform-operated non-exportable keys with encrypted software key material protected by a platform wrapping key; document the threat model honestly.
- Establish automated SQLCipher/curve feasibility, specify physical backup/lock/lifecycle procedures and record dependency/build implications. Physical results are owned by MC-043/044 under the approved deferral.

## Exit criteria

- [x] An explicit key-provider contract and provisional wrapped-key development model are selected for both platforms, with physical certification deferred to MC-043/044.
- [x] Android emulator creation/restart/wrong-key/key-loss/reset passes; Rust vectors and iOS builds/host interoperability pass; physical backup/lock/uninstall procedures and limitations are recorded.
- [x] Every deferred physical scenario has an owning ticket and a mandatory downstream acceptance gate; sensitive-data use remains blocked until the relevant platform is verified.
- [x] Relevant checks and source review pass with evidence recorded. The final deferral revision must pass review/checks before the squash merge makes completion effective.

## Potential fallbacks

- If a platform cannot perform the selected curves in its protected hardware, use reviewed wrapped software keys with the weaker protection clearly recorded.
- If secure persistence cannot be demonstrated, block persistent identities/DM release instead of falling back to plaintext storage.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Started from main `9e3c75a17b759391fa53217d6002ade6336ff55c`, where MC-003 and MC-004 are complete. The [probe plan](../../decisions/MC-005-probe-plan.md) records implemented candidate protection models, dependency/build implications, automated evidence and required device scenarios. The [bench runbook](../../../tests/bench/security/README.md) provides concrete build and acceptance procedures.

Implemented: optional Rust Ed25519/X25519 probe exports and RFC vectors; isolated generated bindings; standalone Android/iOS wrapped-key and SQLCipher fixtures; native/core interoperability; explicit key deletion/reset and lock-read controls; backup exclusion configuration; sanitized bounded reports. These are feasibility tools, not production identity/storage providers.

Automated evidence: Rust 1.85.1 all-feature Clippy and Debug/Release tests pass (four foundation and three probe tests); cargo-deny policy passes; probe bindings generate. CI runs `34634940919` (`0741b26`) and `34636287902` (`12c30af`) passed native builds/lint, instrumentation compilation, rebuilt SQLCipher/Rust/JNA APK alignment, all four unsigned iOS security builds, Swift/CryptoKit interoperability and Rust/ticketboard gates. The original AAR alignment failure was resolved by the pinned native rebuild.

The user requested emulator testing. Three process-separated Android API 29 phases implement create/reopen/wrong-key, key-loss refusal/ciphertext retention and explicit reset. Run `34638458504` (`331dec9`) passed all native checks and booted the software emulator, but package service was unavailable during APK installation after compilation; no lifecycle phase ran. Startup now checks package/activity readiness, stops the VM during builds and restarts it immediately before testing. [CI run 34641115491](https://github.com/wickesjon/meshChat/actions/runs/34641115491) passed all four jobs at source/workflow revision `62f81e68e29e4218362f5176e58eea1ffb707d32`. Android API 29 (4096-byte pages) passed create, reopen and key-loss phases with explicit instrumentation success codes. This establishes the synthetic emulator lifecycle, wrong-key refusal/correct-key recovery and explicit reset, with key-loss ciphertext retention. The final evidence-only update leaves tested sources unchanged. These results remain emulator-only; the later approved deferral moves physical acceptance to MC-043/044.

Device/Mac/signing inventory remains unknown. On 2026-09-11 the user explicitly deferred that verification. MC-043 owns Android physical capability, storage lifecycle, backup/restore, lock/reboot, OS invalidation and uninstall/restore; MC-044 owns the same iOS scenarios. MC-034 requires MC-043; MC-037 requires MC-044 (and MC-034), so Android beta remains independent of iOS verification while integrated assessment/release requires both. The selected development contract is documented in the probe plan. Development may use synthetic data; no real sensitive data or verified-device support claim is authorized before the relevant platform gate passes.

This ticket is staged complete for the reviewed squash under the revised early feasibility criteria. Deferred verification remains unchecked in its own tickets. The original physical merge blocker is superseded by the user's scheduling decision, not by a fabricated test result.

## Review and merge

- Branch: `ticket/MC-005-platform-key-and-encrypted-store-feasibility`.
- Review/PR: [PR #7](https://github.com/wickesjon/meshChat/pull/7). Separate Terra medium review of `85047becbf428dcd4f860d6e89f12ea9019a841a` posted [review #5182256064](https://github.com/wickesjon/meshChat/pull/7#pullrequestreview-5182256064), no actionable code defects; merge withheld for missing physical evidence/final model. Completion was awaited without polling. Follow-up [review #5182328957](https://github.com/wickesjon/meshChat/pull/7#pullrequestreview-5182328957) flagged possible instrumentation APK inclusion in native checks. The actual `0741b26` CI check passed (the test APK is deeper than the original glob), and selectors are now explicit Debug/Release paths to remove ambiguity. Subsequent Terra reviews found no blocking code issue, including [review #5182864367](https://github.com/wickesjon/meshChat/pull/7#pullrequestreview-5182864367) of final source/workflow revision `62f81e68e29e4218362f5176e58eea1ffb707d32`. Review completion was awaited without polling. Evidence-only review [#5183039180](https://github.com/wickesjon/meshChat/pull/7#pullrequestreview-5183039180) verified the emulator logs at `02da25b`. These reviews predate the physical deferral; the revised acceptance/model/deferral diff requires a fresh Terra follow-up before merge.
- Squash commit title: `MC-005: Platform key and encrypted-store feasibility`.
- Completion becomes effective only when the reviewed squash commit lands on main.

CI reliability follow-up: documentation-only runs `34642974457` (`02da25b`) and `34645858133` (`e374c46`, first attempt) passed all native build/lint/alignment checks but failed on the second emulator boot with Android `DeadSystemException` during wake input, before any lifecycle test. One failed-job retry was requested before the matching earlier failure was discovered; it is superseded by the startup change. CI now prepares the AVD, compiles, then performs one clean disposable-AVD boot. Wake/unlock commands are bounded and part of two consecutive readiness samples within the existing boot deadline. Lifecycle assertions are unchanged and are not retried or bypassed. Fresh CI and Terra review are required for this helper/workflow change; the earlier passing emulator evidence remains at `62f81e6`.

AVD storage follow-up: run `34648153199` at `7c70a90` passed all build/lint/alignment checks but the emulator refused to create its default userdata partition (7372.80 MB required, 4369.25 MB available after compilation). CI now sets the disposable AVD `disk.dataPartition.size` to `2G`, using the [emulator hardware configuration property](https://android.googlesource.com/platform/prebuilts/android-emulator/+/master/linux-x86_64/lib/hardware-properties.ini). Only this generated repository-local AVD is changed; no host cleanup or test assertion changes are introduced. Runtime validation remains required.
