---
id: "MC-045"
title: "Defer physical acceptance until integrated candidates"
depends_on: ["MC-001","MC-007"]
kind: "planning"
branch: "ticket/MC-045-defer-physical-acceptance"
---

# MC-045 — Defer physical acceptance until integrated candidates

## Objective

Apply the user's 2026-09-14 instruction to defer real-device verification until implementation is near completion, preserving every physical acceptance requirement before beta/release and distinguishing it from native build validation.

## Dependencies

`MC-001`, `MC-007` are complete on main. See the [ticket index](../README.md#ticket-index).

## Scope

User-authorized documentation changes only: `AGENTS.md`, `docs/decisions/local-validation-policy.md`, `docs/mesh-chat-design.md` §0.3, `docs/ticketboard/implementation-plan.md`, this ticket, generated ticket index/diagram, and backlog tickets MC-023–MC-027, MC-033–MC-035, MC-043–MC-044. No production source, build scripts, dependencies, workflow or account changes.

## Implementation details

- Separate implementation-time automated/native compilation gates from deferred physical acceptance. Keep unavailable required native builds blocking; do not reinterpret the physical deferral as an iOS toolchain waiver.
- Transfer Android driver/OEM and Beacon physical scenarios to MC-025 and iOS driver/UI scenarios to MC-027. Retain synthetic automated implementation coverage and named-device acceptance criteria.
- Remove MC-027 as an implementation prerequisite of MC-035, retaining its driver and crypto prerequisites explicitly. Schedule physical radio and key/storage gates after integrated feature implementations through authoritative dependencies.
- Preserve MC-034 beta and MC-037/038/040 release dependency closure, independent assessments, sensitive-data restrictions and all measurable thresholds.

## Exit criteria

- [x] Every transferred physical scenario has a named later owner, with beta/release blocked on that owner.
- [x] Feature implementation no longer depends on the deferred physical gates; the graph is acyclic and regenerated consistently.
- [x] Policy, design and plan agree on synthetic evidence, native builds, physical certification and release prerequisites; MC-011's separate native blocker is not waived.
- [ ] Documentation checks and separate Terra medium review pass; completion becomes effective only after squash merge.

## Potential fallbacks

- If integrated candidates are ready before devices, keep physical gates and their beta/release descendants blocked; retain implementation results without claiming device certification.
- If required automated/native checks cannot execute, record that distinct blocker. Deferral never converts a failed or unavailable check into a pass or permits weaker protection.

## Evidence

Started from main `bee682ae62b04cc360cf44fbe878fa7404178bf5`; PR #13 remains on its separate branch. User is installing Android Studio; installation completion and native execution are not claimed. Documentation inspection confirms the Android NDK build script currently maps Linux/macOS only and iOS requires Mac/Xcode. No physical test result is claimed by this scheduling change.

Implemented the policy/§0.3/plan updates and ten affected backlog ticket updates. MC-025 retains its stable filename/branch and now describes integrated-candidate acceptance. Physical Android driver/OEM/Beacon and iOS driver/UI scenarios have explicit checkboxes in MC-025/027. Those gates, and MC-043/044, remain unchecked; none is marked complete by this ticket. MC-034 remains a physical beta gate. Native builds and MC-022/037 independent assessments remain mandatory.

Validation on Python 3.14.4, Windows: `python -B tests/ticketboard/validate.py --write`, default validator, `python -B -m unittest discover -s tests/ticketboard -v` (12 tests), and `git diff --check` pass. The generated DAG has 45 tickets and 123 dependencies. An additional read-only traversal of the parsed authoritative graph verifies that MC-023/024/026/033/035 have no MC-025/027/043/044 ancestor; MC-025/043 follow all six Android feature tickets; MC-027/044 follow MC-035; MC-034 retains MC-025/043/022; MC-037 retains MC-027/044/022; and MC-040 retains all four physical owners plus MC-022/036/037/038. All 79 relative document links checked in the affected documents resolve. The local audit script is ignored at `.work/mc045-check.py`; the assertions above describe its checks without treating graph consistency as physical evidence.

Check applicability: this PR changes only documentation and ticket dependencies. It does not alter production code, packages, wire vectors, platform manifests, FFI or build infrastructure, so Rust/native execution is not required for this PR. This does not change check applicability for the separate MC-011 dependency PR.

## Review and merge

- Branch: `ticket/MC-045-defer-physical-acceptance`.
- Review/PR: pending.
- Squash commit title: `MC-045: Defer physical acceptance until integrated candidates`.
- Completion becomes effective only when the reviewed squash commit lands on main.
