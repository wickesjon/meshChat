---
id: "MC-004"
title: "Two-platform BLE and permission feasibility"
depends_on: ["MC-003"]
kind: "spike"
branch: "ticket/MC-004-two-platform-ble-and-permission-feasibility"
---

# MC-004 — Two-platform BLE and permission feasibility

## Objective

Build temporary direct-link probes in both GATT roles using proposed service/characteristic UUIDs.

## Dependencies

`MC-003` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/android/ble/**`, `src/ios/BLE/**`, `tests/bench/**`, `docs/decisions/**`, `docs/mesh-chat-design.md`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Build temporary direct-link probes in both GATT roles using proposed service/characteristic UUIDs.
- Measure write and notify capacities separately, backpressure, low-MTU behavior, duplicate connections, foreground/background discovery, held-link survival and reconnects.
- Record devices, OS versions, durations, traces and the Android permission/RSSI decision; include suspended and force-quit states as separate outcomes.

## Exit criteria

- [ ] Recorded Android/Android, Android/iOS and iOS/iOS results distinguish discovery from established connections.
- [ ] Both directions have measured capacity and flow-control evidence, including failure/recovery behavior.
- [ ] A permission decision and supported deployment matrix are committed; unsupported cases are explicit.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If no Mac/iPhone or physical Android devices are available, mark hardware evidence blocked; simulation cannot replace the spike.
- If low-capacity links cannot fit the agreed maximum packet, propose a supported-link floor or a versioned fragmentation change before MC-006 closes.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Started from main `b3dc7fd05d45eb4fade01256449d34d54e1c7037`, where MC-003 is complete through [PR #5](https://github.com/wickesjon/meshChat/pull/5). The [physical measurement procedure and build guide](../../decisions/MC-004-ble-feasibility.md) and standalone probe source are prepared for review. Hardware evidence and the final permission/support decision remain blocked on device access; the ticket cannot merge yet.

Device inventory and Mac/Xcode/development-signing access are awaiting user input. The required Android/Android and iOS/iOS cases need two physical phones of each platform. No hardware unavailability is inferred from the unanswered question, but the physical gates cannot be marked passed without access and actual measurements. This ticket remains pending review/evidence; MC-006/007 remain blocked.

- Implemented temporary standalone native probe projects entirely under `src/android/ble/` and `src/ios/BLE/`. Both use identical test-only UUIDs, one central path and one selected peripheral subscriber, independent write/notify queues capped at eight values, payloads capped at 1024 synthetic bytes and a 4096-row in-memory trace. No wire decision, user-message traffic, persistent identity or cryptographic operation is introduced.
- Android runs through a connected-device foreground service with explicit permission controls. It serializes GATT operations, logs callback/API refusal and five-second callback timeouts, and invalidates scan/server generations on cleanup. The context retained by the session is Application-only. The scoped `MissingPermission` suppression covers the permission-checked callback helper; revocation races are caught and resources are closed. API 29–32 deprecated GATT calls are isolated behind version guards and local compiler suppressions. Other strict lint remains enabled.
- The experimental Android manifest requests location rather than asserting `neverForLocation`; it requests coarse and fine together and records grants. Approximate-only access is exercised on API 31+, while API 29/30 scanning requires fine permission. This is a probe configuration for evidence collection, not the final product/privacy ruling. Background-location and notification visibility controls are separate.
- iOS uses CoreBluetooth main-queue delegates, separate write/notify readiness callbacks, state-restoration identifiers and bounded traffic. It reports oversized values refused by the probe before an API call distinctly from enqueue acceptance and receiver observation. iOS cannot forcibly disconnect an inbound central; this limitation is explicit in its report. Timers may pause under suspension and are not a keepalive guarantee.
- Local `gradlew.bat -p src/android/ble --offline --no-daemon assembleDebug assembleRelease lintDebug` passed (87 tasks, 41 executed) with the pinned repository-local JDK/SDK/Gradle. Initial lint caught coarse/fine permission pairing, context typing and string resources; those were fixed without reducing the lint gate. No Android device execution is claimed.
- The iOS Info.plist and shared scheme parse as XML. Swift/Xcode compilation is not yet verified on this Windows host. A request to add `.github/workflows/ci.yml` to MC-004 for probe build checks is pending user approval; the file has not been modified for MC-004. MC-003's scope approval does not automatically apply. A documented Mac build is an alternative; existing CI currently checks the parent apps/core, not these standalone probes.
- Ticketboard validation and whitespace checks pass. Independent code review completed as recorded below; all physical exit criteria remain unchecked.

## Review and merge

- Branch: `ticket/MC-004-two-platform-ble-and-permission-feasibility`.
- Review/PR: [PR #6](https://github.com/wickesjon/meshChat/pull/6), published ready for review. A separate gpt-5.6-terra worker at medium effort reviewed `17e67720a725c1cc61d734cff31e5a43b910a4fb` against main `b3dc7fd05d45eb4fade01256449d34d54e1c7037` and returned no actionable findings. It reviewed source, scope and evidence without running builds or modifying files, and independently confirmed `git diff --check`. Its completion was awaited without polling. This review does not replace the pending iOS build or physical evidence, and no merge is authorized by missing gate results.
- Squash commit title: `MC-004: Two-platform BLE and permission feasibility`.
- Completion becomes effective only when the reviewed squash commit lands on main.
