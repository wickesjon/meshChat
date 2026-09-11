---
id: "MC-003"
title: "Sans-IO and UniFFI foundation"
depends_on: ["MC-002"]
kind: "foundation"
branch: "ticket/MC-003-sans-io-and-uniffi-foundation"
---

# MC-003 — Sans-IO and UniFFI foundation

## Objective

Define events for peer connection, disconnection, inbound bytes, link capacities, time and power changes; return bounded send commands and UI events.

## Dependencies

`MC-002` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/core/**`, `src/android/**`, `src/ios/**`, `tests/integration/ffi/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Define events for peer connection, disconnection, inbound bytes, link capacities, time and power changes; return bounded send commands and UI events.
- Inject monotonic time and randomness for reproducible tests; keep radio and platform key access behind explicit interfaces.
- Prove generated Kotlin and Swift bindings with a small round trip and defined FFI error behavior; inspect UniFFI panic handling before adding redundant wrappers.

## Exit criteria

- [ ] Both native skeletons call the same core through generated bindings.
- [ ] A deterministic event trace produces identical commands across repeated runs.
- [ ] Malformed input and disconnect sequences return defined errors without unwinding into native callers or leaking link state.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If callback-heavy bindings are fragile, use pull-based event/command batches with the same bounded semantics.
- If the selected panic strategy cannot recover, keep parser paths total and document the process-level failure policy; never claim catch_unwind catches aborts.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Not implemented. Record commands, versions, reproducible inputs and results here. For manual/hardware checks include device/OS, duration and report paths. No test or review is claimed yet.

## Review and merge

- Branch: `ticket/MC-003-sans-io-and-uniffi-foundation`.
- Review/PR: pending.
- Squash commit title: `MC-003: Sans-IO and UniFFI foundation`.
- Completion becomes effective only when the reviewed squash commit lands on main.
