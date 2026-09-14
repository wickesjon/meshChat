---
id: "MC-027"
title: "Cross-platform radio interoperability"
depends_on: ["MC-026","MC-023","MC-022","MC-035"]
kind: "gate"
branch: "ticket/MC-027-cross-platform-radio-interoperability"
---

# MC-027 — Cross-platform radio interoperability

## Objective

Run the approved discovery matrix separately from established-link lifecycle tests.

## Dependencies

`MC-026`, `MC-023`, `MC-022`, `MC-035` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `tests/bench/interop/**`, `tests/bench/ios/**`, `tests/integration/ios-ui/**` (physical acceptance harness/evidence only), `docs/testing/**`, `tests/vectors/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Run the approved discovery matrix separately from established-link lifecycle tests.
- Exercise signed traffic, DMs, SYNC transport objects, reconnects and differing capacities across Android/iOS combinations.
- Record minimum OS and multiple device generations, including screen-off and cold-start cases.

## Exit criteria

- [ ] Deferred MC-026 physical both-role/frame/capacity tests and background, suspension, disconnect, restoration, foreground catch-up and discovery/force-quit cases pass with named device/OS, source/build, timing and limitations.
- [ ] Deferred MC-035 every shipping v1 flow passes on the supported minimum/current physical iOS matrix with the production frozen core, including key reset, background restoration, offline purchase cache, foreground catch-up, accessibility/confirmation and trust/privacy boundaries.
- [ ] All supported pairings pass packet, fragmentation, crypto and SYNC checks with recorded evidence.
- [ ] Unsupported pairings are explicit and surfaced consistently in product behavior.
- [ ] No native platform implements a divergent parser or alters authenticated bytes.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If a platform-specific case is unreliable, narrow supported behavior explicitly and keep release blocked until the product fallback is approved.
- Do not count the same-core language-binding test as a substitute for physical interop.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Scheduling approved 2026-09-14 in [the validation policy](../../decisions/local-validation-policy.md#physical-acceptance-scheduling--approved-2026-09-14). Runs on integrated Android/iOS candidates after MC-035; it no longer blocks iOS UI implementation. It remains a hard dependency of MC-036 and therefore MC-037/038 and release. MC-044 separately owns physical key/storage protection; synthetic data is used until that protection gate passes. No original pairing or lifecycle criterion is removed.

Not implemented. Record commands, versions, reproducible inputs and results here. For manual/hardware checks include device/OS, duration and report paths. No test or review is claimed yet.

## Review and merge

- Branch: `ticket/MC-027-cross-platform-radio-interoperability`.
- Review/PR: pending.
- Squash commit title: `MC-027: Cross-platform radio interoperability`.
- Completion becomes effective only when the reviewed squash commit lands on main.
