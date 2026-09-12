---
id: "MC-010"
title: "Bounded framing and reassembly"
depends_on: ["MC-009","MC-007"]
kind: "core"
branch: "ticket/MC-010-bounded-framing-and-reassembly"
---

# MC-010 — Bounded framing and reassembly

## Objective

Implement all four frame kinds, per-direction slicing, logical envelopes and transport-object envelopes.

## Dependencies

`MC-009`, `MC-007` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/core/**`, `tests/integration/transport/**`, `tests/fuzz/**`, `tests/vectors/base/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Implement all four frame kinds, per-direction slicing, logical envelopes and transport-object envelopes.
- Enforce group/count/byte/time limits before allocation; scope groups to connection generation and validate IDs and exact reconstructed lengths.
- Define conflicting duplicate fragments, timeout/eviction, disconnect cleanup and malformed-frame accounting.

## Exit criteria

- [x] Whole and fragmented vectors pass at minimum and larger supported capacities without oversized GATT values.
- [x] Cross-link/group collisions, out-of-order fragments, conflicting duplicates and reconnects cannot mix packets.
- [x] Exhaustion tests demonstrate bounded peak buffers and successful cleanup after expiry/disconnect.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If a link cannot carry a legal maximum-size packet within fragment bounds, return an explicit unsupported-capacity result.
- On resource exhaustion drop/evict according to the contract; never grow limits or accept truncated packets.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Started from main `2145d862b423f0fe8d810f7dc213d3cd15b0a66d`; MC-009 and MC-007 are complete. MC-008's whole-only LINK_PROOF extension is available. No scope expansion or fallback was required.

Implemented the internal `framing` module: all four outer frame kinds, bounded whole/bootstrap HELLO and LINK_PROOF, exact SYNC_ITEM data/marker grammar, per-direction streamed encoding and allocation-bounded reassembly. Validated fragments retain arrival partitions and reconstruct in index order; completion checks the logical message ID and ordinary codec or transport-body grammar. Direct controls cannot become embedded history. Count, total and slice constraints are checked before slot admission. A fixed 1,035-byte caller output is required before mutation; no per-value heap allocation occurs.

Groups are keyed by process instance, registered connection generation and logical message/group or transport kind/transfer. Registrations use monotonically fresh generations. Disconnect or changed capacity invalidates partial/rejected context; the owner must cancel outbound values and re-admit with a fresh generation. Exact duplicates do not extend the original 30-second deadline. Conflicting metadata/bytes and malformed completions reject through the original deadline; a first malformed complete known envelope anchors its own deadline. Truncated and unknown fragment keys allocate no tracking; accepted-message dedup is never modified. Incomplete overflow evicts oldest; rejected overflow replaces earliest expiry. `ingest_admitted` requires earlier frame/byte admission, not a bypass around MC-013. Session/HELLO/proof authentication and native queue integration remain their later gates.

Fixed pools include slot metadata, inline payload capacity and actual Vec capacity. Windows x86_64 measured reservations are logical 72,728 bytes, transport 36,632, rejected 16,408 and manager/link metadata 304: total 126,072. These byte reservations are constant current/peak and stay under MC-007 category caps. Tests also assert full occupied-slot bytes under per-link 12/8/8 KiB and reach 64/32/512 node groups without growing pools. All active group ownership clears on expiry and disconnect; bounded reusable pool allocations remain. Native queues, other future protocol components and general process/allocator overhead are separately accounted, not claimed here.

The [vector/evidence guide](../../../tests/vectors/base/README.md#mc-010-outer-framing-and-bounded-reassembly) includes 15 independent Python-assembled outer vectors plus the canonical alternate-partition reaction. Eleven transport tests cover C145 refusal/C146/C182/C512, all object forms, malformed envelopes/fields, cross-link/group/generation isolation, duplicate/conflict/sum errors, original expiry after eviction, capacity changes, monotonic time, exhaustion and recovery. The framing fuzz target runs 100,000 seeded mutations through stateless and stateful paths with eight links, fixed-memory assertions and final cleanup. This is reproducible mutation fuzzing, not coverage-guided or native evidence.

Local Rust/cargo 1.85.1 Windows MSVC formatting, clippy all-target/all-feature locked with `-D warnings`, debug/release all-feature locked tests (26 each), release build and unchanged-lockfile check pass. cargo-deny 0.20.2 advisory refresh and all four gates pass, with only preexisting unused-license-allowance warnings. Python 3.14.4 logical/frame vector checks, ticketboard write/default, 12 board tests and `git diff --check` pass. No dependencies, build scripts, UniFFI exports or native interfaces changed. This module is not yet wired to native ingress; local component checks apply under the approved policy, subject to separate Terra confirmation. No physical, authenticated-peer or independent security result is claimed.

## Review and merge

- Branch: `ticket/MC-010-bounded-framing-and-reassembly`.
- Review/PR: pending.
- Squash commit title: `MC-010: Bounded framing and reassembly`.
- Completion becomes effective only when the reviewed squash commit lands on main.
