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

### Implementation plan and standing scope approval — 2026-09-17

All hard dependencies are complete on main. Branch starts from MC-042 squash `1861fb6bf20ecaff03deb5f9b79dfb9d79c78a11`. Integrate the actual shared native core, Keychain/SQLCipher adapters, CoreBluetooth driver and StoreKit policy behind a serialized SwiftUI feature owner. Preserve explicit confirmation for channel joins, friend pins/replacement, event roots and staff imports; plaintext channel and authenticated encrypted DM boundaries; generation-bound short-lived private operations; protected failure/reset behavior; iOS foreground catch-up and honest background/Beacon limits. Settings, themes, power and contribution panels use protected settings and existing components.

Necessary additional scope under standing repository-local approval: `.github/workflows/ci.yml` registers native UI/integration acceptance and production device/simulator builds; `tests/bench/ios/**` may extend existing driver checks for protected egress hooks while retaining their original coverage. Production and test entry points remain separate: synthetic wrapping/storage and deterministic radio inputs live only under `tests/integration/ios-ui/`, never as a production fallback. Mac/Xcode compilation and synthetic UI/integration checks are required; real-device flow/restoration and protection remain MC-027/044. Record gaps as blockers rather than reducing the v1 requirements.


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

### Additional shared-core integration scope

Inspection found that the native transport serves canonical SYNC requests but does not expose a requesting/received-history path to native feature owners. MC-035 explicitly requires foreground catch-up; merely changing its label would not satisfy that requirement. Necessary repository-local scope extension: `src/core/src/native_transport.rs`, its private native integration modules and registrations, plus `tests/integration/ios-ui/**` and shared native FFI regressions as needed, to expose the existing bounded SYNC session machinery without changing the frozen wire contract or trusting unverified content. Validate link-bound requesting, first-native-attempt deadline, received stored history/authentication, continuations, cancellation/restoration and ordinary transport budgets. This requirement remains open until the implementation and tests pass.
