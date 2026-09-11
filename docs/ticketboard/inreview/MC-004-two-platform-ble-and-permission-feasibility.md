---
id: "MC-004"
title: "Two-platform BLE and permission feasibility"
depends_on: ["MC-003"]
kind: "spike"
branch: "ticket/MC-004-two-platform-ble-and-permission-feasibility"
---

# MC-004 — Two-platform BLE and permission feasibility

## Objective

Establish documented BLE feasibility and permission policy using primary online evidence and compiled direct-link probes in both GATT roles.

## Dependencies

`MC-003` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/android/ble/**`, `src/ios/BLE/**`, `tests/bench/**`, `docs/decisions/**`, `docs/mesh-chat-design.md`, `.github/workflows/ci.yml`, `docs/ticketboard/implementation-plan.md`, `docs/ticketboard/backlog/MC-016-base-transport-freeze-gate.md`.

Scope approval: on 2026-09-11 the user explicitly approved adding `.github/workflows/ci.yml` for the prepared standalone probe build checks: Android Debug/Release and lint, plus unsigned iOS Debug/Release simulator/device compilation. Signing overrides remain on CI command lines only. This approval does not waive physical evidence or authorize other scope changes.

Acceptance/scope approval: on 2026-09-11 the user explicitly requested replacing the physical acceptance gate with online evidence. The [replacement decision](../../decisions/MC-004-online-feasibility.md) defines the new criteria and limitations. Necessary consistency updates include the normative design, active plan and MC-016 reference to MC-004 evidence. Later physical radio/release gates and MC-005 protected-storage gates remain unchanged. Earlier review blockers about absent MC-004 measurements are superseded by this approval, not recorded as passed tests.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Build temporary direct-link probes in both GATT roles using proposed service/characteristic UUIDs.
- Record cited platform write/notify capacity and flow-control contracts, published measurements with device/OS/setup and missing-method limitations, and explicit low-capacity refusal inputs for MC-006.
- Commit the Android permission/RSSI decision and a conservative development matrix separating discovery, held links, background, suspension and user termination. Keep unmeasured behavior explicit and retain the physical procedure for downstream gates.

## Exit criteria

- [x] Primary online sources and attributable measurements record setup/OS and limitations; no external result is represented as a meshChat test.
- [x] Both directions have documented capacity/flow-control contracts and explicit failure/recovery assumptions; a development matrix distinguishes discovery from held links and unsupported cases.
- [x] A permission/RSSI decision is committed; later physical validation obligations and protocol refusal requirements remain explicit.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- Missing physical devices do not block this approved online-evidence gate. They still block later physical gates; neither hosted compilation nor external measurements certify meshChat hardware behavior.
- If low-capacity links cannot fit the agreed maximum packet, propose a supported-link floor or a versioned fragmentation change before MC-006 closes.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Started from main `b3dc7fd05d45eb4fade01256449d34d54e1c7037`, where MC-003 is complete through [PR #5](https://github.com/wickesjon/meshChat/pull/5). The [online feasibility decision](../../decisions/MC-004-online-feasibility.md), [retained physical procedure/build guide](../../decisions/MC-004-ble-feasibility.md) and compiled standalone probes satisfy the revised content criteria. Review and checks of the acceptance replacement are pending.

No device execution occurred and device availability is unknown. Later physical cases require two phones per platform and Mac/signing access. MC-006/007 become dependency-ready only when this ticket is squash merged under the revised criteria. No dependency has been removed.

- Implemented temporary standalone native probe projects entirely under `src/android/ble/` and `src/ios/BLE/`. Both use identical test-only UUIDs, one central path and one selected peripheral subscriber, independent write/notify queues capped at eight values, payloads capped at 1024 synthetic bytes and a 4096-row in-memory trace. No wire decision, user-message traffic, persistent identity or cryptographic operation is introduced.
- Android runs through a connected-device foreground service with explicit permission controls. It serializes GATT operations, logs callback/API refusal and five-second callback timeouts, and invalidates scan/server generations on cleanup. The context retained by the session is Application-only. The scoped `MissingPermission` suppression covers the permission-checked callback helper; revocation races are caught and resources are closed. API 29–32 deprecated GATT calls are isolated behind version guards and local compiler suppressions. Other strict lint remains enabled.
- The experimental Android manifest requests location rather than asserting `neverForLocation`; it requests coarse and fine together and records grants. Approximate-only access is exercised on API 31+, while API 29/30 scanning requires fine permission. The replacement decision now selects the location-disclosed product path; the probe remains an experimental permission exerciser, not a production permission implementation. Background-location and notification visibility controls are separate.
- iOS uses CoreBluetooth main-queue delegates, separate write/notify readiness callbacks, state-restoration identifiers and bounded traffic. It reports oversized values refused by the probe before an API call distinctly from enqueue acceptance and receiver observation. iOS cannot forcibly disconnect an inbound central; this limitation is explicit in its report. Timers may pause under suspension and are not a keepalive guarantee.
- Local `gradlew.bat -p src/android/ble --offline --no-daemon assembleDebug assembleRelease lintDebug` passed (87 tasks, 41 executed) with the pinned repository-local JDK/SDK/Gradle. Initial lint caught coarse/fine permission pairing, context typing and string resources; those were fixed without reducing the lint gate. No Android device execution is claimed.
- Following the specific scope approval above, CI includes standalone Android Debug/Release assembly and strict lint, and iOS Debug/Release compilation for both simulator and device SDKs. It reuses the pinned toolchains, keeps temporary/build output under the repository, and disables signing only on the CI command line. The iOS job in [run 34627732316](https://github.com/wickesjon/meshChat/actions/runs/34627732316) passed all four probe builds at `8a148b1aa28ecdd95523b79607d7a0678c8f4b5d` with Xcode 16.4 (16F6) and Swift 6.1.2; project/Info.plist lint also passed. This closes the previously unverified iOS compilation check, but cannot establish physical BLE behavior. Follow-up review is recorded below.
- Ticketboard validation and whitespace checks pass. Prior independent code reviews completed as recorded below; the new evidence/acceptance change requires fresh review. Physical execution remains unverified.
- Hosted CI [run 34627732316](https://github.com/wickesjon/meshChat/actions/runs/34627732316) passed all four jobs (`ticketboard`, `rust`, `android`, `ios`) at reviewed revision `8a148b1aa28ecdd95523b79607d7a0678c8f4b5d`, including the new standalone Android assembly/lint step and all four iOS probe builds. The subsequent documentation-only commit records these results and the review; source and workflow are unchanged. Ticketboard validation and `git diff --check` passed for the evidence update.

## Review and merge

- Branch: `ticket/MC-004-two-platform-ble-and-permission-feasibility`.
- CI follow-up review: a separate gpt-5.6-terra worker at medium effort reviewed `8a148b1aa28ecdd95523b79607d7a0678c8f4b5d`, including the approved workflow delta from `571676894c9bc6ad9845f7b5ca76a6e3fe885bee`, and returned no actionable findings. It confirmed scope, base ancestry and `git diff --check` without builds, hosted CI, actionlint or hardware tests. Completion was awaited without polling. At that revision, physical gates remained unchanged; the later explicit replacement above supersedes the MC-004 gate only.
- Review/PR: [PR #6](https://github.com/wickesjon/meshChat/pull/6), published ready for review. A separate gpt-5.6-terra worker at medium effort reviewed `17e67720a725c1cc61d734cff31e5a43b910a4fb` against main `b3dc7fd05d45eb4fade01256449d34d54e1c7037` and returned no actionable findings. It reviewed source, scope and evidence without running builds or modifying files, and independently confirmed `git diff --check`. Its completion was awaited without polling. That review did not replace the then-pending iOS build or physical evidence. Compilation has since passed. The later explicit acceptance replacement above supersedes the absent-MC-004-hardware blocker; downstream physical gates remain.
- Review of `238763b6bd0277146f3cf95d28421ed3e5680c08`: a separate Terra medium worker [posted a COMMENT review](https://github.com/wickesjon/meshChat/pull/6#pullrequestreview-5181768700) with no code/documentation defects, withholding approval under the then-current physical criteria. Completion was awaited without polling. Fresh review must assess this approved criteria change.
- Squash commit title: `MC-004: Two-platform BLE and permission feasibility`.
- Completion becomes effective only when the reviewed squash commit lands on main.
