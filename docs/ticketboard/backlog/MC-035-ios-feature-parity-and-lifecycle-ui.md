---
id: "MC-035"
title: "iOS feature parity and lifecycle UI"
depends_on: ["MC-026","MC-022","MC-029","MC-030","MC-031","MC-032","MC-042"]
kind: "ios"
branch: "ticket/MC-035-ios-feature-parity-and-lifecycle-ui"
---

# MC-035 — iOS feature parity and lifecycle UI

## Objective

Implement SwiftUI parity for onboarding/channels, friends/DMs, event trust, sharing, themes and Supporter flows using the shared core.

## Dependencies

`MC-026`, `MC-022`, `MC-029`, `MC-030`, `MC-031`, `MC-032`, `MC-042` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/ios/UI/**`, `src/ios/**`, `tests/integration/ios-ui/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Implement SwiftUI parity for onboarding/channels, friends/DMs, event trust, sharing, themes and Supporter flows using the shared core.
- Integrate Keychain-backed storage/lifecycle behavior and explicit iOS background degradation/foreground SYNC states.
- Provide an explanatory disabled Beacon Mode entry and native accessibility/confirmation protections.

## Exit criteria

- [ ] Each shipping v1 flow passes native UI/integration tests with the production frozen core and synthetic inputs, plus applicable Mac/Xcode builds. Real-device flow/lifecycle acceptance remains mandatory in MC-027.
- [ ] Key reset, background restoration, offline purchase cache and foreground catch-up behave as specified.
- [ ] Native iOS screens preserve the same plaintext/encrypted and verified/unverified boundaries as Android.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If a platform capability differs, provide the approved equivalent behavior with honest UI wording.
- Do not silently remove a v1 feature; record a release-blocking gap or request a scope decision.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Scheduling approved 2026-09-14 in [the validation policy](../../decisions/local-validation-policy.md#physical-acceptance-scheduling--approved-2026-09-14). Implementation consumes the native driver and independently frozen core before physical interop. MC-027 now follows this ticket and owns every shipping flow on the real iOS device matrix; no native build waiver is implied.

Not implemented. Record commands, versions, reproducible inputs and results here. For manual/hardware checks include device/OS, duration and report paths. No test or review is claimed yet.

## Review and merge

- Branch: `ticket/MC-035-ios-feature-parity-and-lifecycle-ui`.
- Review/PR: pending.
- Squash commit title: `MC-035: iOS feature parity and lifecycle UI`.
- Completion becomes effective only when the reviewed squash commit lands on main.
