---
id: "MC-042"
title: "Honest contribution and power feedback"
depends_on: ["MC-028","MC-024"]
kind: "product"
branch: "ticket/MC-042-honest-contribution-and-power-feedback"
---

# MC-042 — Honest contribution and power feedback

## Objective

Implement local counters and power feedback with measured-versus-estimated labels and defined reset/retention behavior.

## Dependencies

`MC-028`, `MC-024` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/core/**`, `src/android/ui/**`, `src/ios/UI/**`, `tests/integration/stats/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

### Implementation plan and standing scope approval — 2026-09-17

Dependencies MC-028/024 are complete on main. This dedicated branch starts from `00d6c85e818b78c0fd7669f166747f0cb9f61976`, the reviewed MC-030 squash. No dependent code starts before closure.

Use the canonical native transport's bounded local aggregate counters. Distinguish incoming GATT values, reassembled logical packets (including repeated copies), scheduled outgoing frames, successful native frame completions, completed outgoing objects and relayed live CHAT copies per egress link. Native completion is never remote delivery. Failed/stale/duplicate callbacks must not count as successful completion, fragments/retries must not inflate completed objects, and reset must not alter scheduler credits or trust. Session-only retention is explicit: counters start with the protected transport owner, survive ordinary radio stop/start, and clear on explicit stats reset, process restart or protected owner replacement/reset. No persisted identity/content/peer/channel aggregate dimension or telemetry is added.

Fallback triggers: the transport has no authenticated people counts or disjoint-cluster topology, and the native power adapters supply device charge/power-policy inputs without app-specific energy attribution or a calibrated drain model. Use literal contribution progress from completed relayed chat copies; omit unique-person/bridge/ranking/night claims and app-drain numbers. Update normative §16 to these observable definitions. Show fresh device battery level separately from app consumption and label stale/unavailable readings. Preview a fixed snapshot before explicit image sharing, with user-selected aggregate categories; export contains only selected numeric counts and their qualification.

Android ships the stats/power panel and bounded temporary image export. iOS receives the matching compiled SwiftUI component and shared-core consumer tests; MC-035 owns integration with its full protected application shell, as with existing Supporter components.

Necessary additional paths under the user's standing repository-local scope approval: `src/android/app/build.gradle.kts` registers stats JVM/emulator tests; `src/android/app/src/main/res/xml/share_paths.xml` grants only the stats image cache subdirectory; `src/ios/MeshChat.xcodeproj/project.pbxproj` compiles the SwiftUI stats component; `.github/workflows/ci.yml` registers its native consumers/emulator acceptance; `docs/mesh-chat-design.md` corrects §16's unsupported contribution examples and defines retention/measurement labels. Validation: core counter/reset/fragment/retry regressions, Kotlin/Swift consumer parity, Android native/lint/package/emulator export and power UI checks, Mac native compilation, board checks, separate Terra review and squash merge. No wire, trust, resource-budget or physical/security gate change is authorized or needed.


- Implement local counters and power feedback with measured-versus-estimated labels and defined reset/retention behavior.
- Map contribution titles/share cards from design §16 only to locally observable evidence; replace unverifiable unique-person or bridge claims with accurate counters.
- Keep stats private by default and share/export user-initiated; exclude raw identities, message text and channel words from aggregate diagnostics.

## Exit criteria

- [ ] Counter tests distinguish packets, logical messages and egress sends and cannot imply successful delivery.
- [ ] Battery estimates are labeled and unsupported OS attribution is never presented as measurement.
- [ ] Share cards contain only user-approved local aggregates; no background telemetry or leaderboard is introduced.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If bridge/person-count inference cannot be supported, use literal relay/message counters and record the reduced wording.
- If reliable battery attribution is unavailable, show a labeled estimate or omit the number rather than fabricating measurements.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Implemented: bounded shared native counters and explicit session reset; fresh device-level power feedback without fabricated app drain; Android live panel plus selected-category frozen image preview and bounded read-only export; matching SwiftUI component; core/Kotlin/Swift/emulator regressions. Three targeted Rust tests and strict workspace clippy pass. Normal both-ABI Android bindings and production/test Kotlin compilation pass. Full Rust/Android/Mac checks and Terra review are pending; no physical or final acceptance is claimed.

## Review and merge

- Branch: `ticket/MC-042-honest-contribution-and-power-feedback`.
- Review/PR: [PR #37](https://github.com/wickesjon/meshChat/pull/37).
- Squash commit title: `MC-042: Honest contribution and power feedback`.
- Completion becomes effective only when the reviewed squash commit lands on main.

Additional scope under standing approval: `tests/integration/ffi/main.swift` calls the new stats consumer trace from `tests/integration/stats/StatsChecks.swift`, reusing its existing synthetic SQL callback harness. Native Swift/Android checks validate the new exported record and reset/share methods.

### Initial review and validation corrections

Separate Terra medium source review PASS at `b871cc3448e90ae5f5d15126d15f906e23f4e8e9`, with no actionable findings. Full Rust debug/release suites pass 205 tests each, plus release build/storage policy/dependency gates. Initial Android stats emulator acceptance passes panel/selection/reset/image bounds; the exported image was inspected. Android lint requires the already available Kotlin bitmap factory, now used. The initial Mac Swift consumer trace passes, but its app compile correctly rejects iOS-16-only Transferable/ShareLink against the supported iOS15 deployment target. The component now uses NavigationView and UIActivityViewController without raising the supported minimum or removing export. A new Mac build and follow-up source review are required. Screenshot capture now waits for native dialog animations to settle before visual inspection; no product behavior changes in that test correction.
