---
id: "MC-013"
title: "Ingress budgets, authentication states and dedup"
depends_on: ["MC-010","MC-007","MC-012"]
kind: "core"
branch: "ticket/MC-013-ingress-budgets-authentication-states-and-dedup"
---

# MC-013 — Ingress budgets, authentication states and dedup

## Objective

Charge bytes/frames before allocation and apply global/per-link expensive-work limits at each actual operation.

## Dependencies

`MC-010`, `MC-007`, `MC-012` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/core/**`, `tests/integration/ingress/**`, `tests/simulator/**`, `tests/fuzz/**`.

Also permitted by the 2026-09-15 user decision: coordinated acceptance-sequencing edits to MC-016/019/020/021/022 ticket files and `docs/ticketboard/implementation-plan.md`. Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Charge bytes/frames before allocation and apply global/per-link expensive-work limits at each actual operation.
- Separate bounded attempt tracking from accepted-message dedup so invalid copies cannot suppress later authenticated packets; preserve bounded rejection of repeated garbage.
- Implement per-sender/class and unknown-type limits, control-packet admission and reconnect-resistant abuse accounting within documented limits.

## Exit criteria

- [x] Malformed-first/structurally-valid-second and identical-replay tests pass for clear and opaque signed/encrypted fixtures at the admission/state-machine boundary. Signed/encrypted inputs remain pending/unverified and never enter trusted dedup or authenticated display before a production verifier succeeds. These are ingress/state tests, not cryptographic validation.
- [x] Flooding rotated identities cannot exceed the sum of the attacker's actual admitted link budgets; multi-link attacks are measured separately.
- [x] Over-budget frames never enter display, reassembly or crypto paths; memory and work remain within specified bounds.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If authentication is pending, use the explicit unverified/pending state without adding a trusted dedup entry.
- If budget pressure prevents verification, drop or defer within a bounded queue; never label unverified traffic authenticated.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Implementation prepared for review. Dependencies MC-010/007/012 are complete on main; branch starts from `c3ef8dba9b9bd0ce4400622d84c374ac2b51a92c`. No passing cryptographic fixture or completion is claimed.

### Approved acceptance sequencing — 2026-09-15

The user approved the scoped correction recorded on this branch: MC-013 tests admission, rates, rejection, dedup and explicit pending/unverified states; real ingress-plus-crypto invalid-first/valid-second and authenticated-replay gates are required in MC-019 (friends), MC-020 (DMs) and MC-021 (organizers), with mandatory closure in MC-022. MC-016 records the cryptographic portion as pending. No security test, independent assessment or release requirement is removed.

Reason: application crypto is not yet implemented, the existing crypto vectors are transcript/size fixtures only, and MC-020 depends on MC-016→015→014→013. Making MC-020 an MC-013 prerequisite creates a cycle (verified with the ticket validator in memory). The MC-008 HPKE integration and dependency exceptions retain their original owners. The user also approved coordinated planning edits to MC-016/019/020/021/022 and the active implementation plan; no other scope expands.

Implementation uses the ticket's pending-authentication fallback: opaque signed/encrypted data cannot create trusted dedup or authenticated display. Actual production verifiers and integrated crypto replay evidence remain with the named later tickets; no test verifier substitutes for their results.

### Implementation and integration contract

`src/core/src/ingress.rs` owns frame/byte admission, 16 node/two-per-link 512-byte staging slots, the actual reassembler, known/unknown logical class rates, 4096 sender records, eight accepted-source partitions, bounded rejection and pending queues, connection-attempt admission and work/session budgets. Input is borrowed and copied only after atomic node/link frame/byte charges and staging capacity checks; malformed/oversized attempts still face those budgets. Reported size mismatches fail closed and charge the larger available size. The rejection fast path does not enter reassembly or scan protocol tables.

All tables allocate their fixed capacity at construction. Reservations include keys, metadata, allocator capacity and control blocks; the real reassembler reports its own reservations. Reconnect and capacity changes invalidate the transfer generation, clear staging/fragments/pending state, and retain node/address credit and retired dedup partitions. A new link reclaims the oldest retired partition, never another live peer's partition. Accepted variants hash all immutable bytes (only TTL excluded); rejected variants are scoped to the arrival generation, while known fragments retain the reassembler's original group deadlines. Pending duplicate reception never creates accepted/trusted dedup or refreshes the 30-second deadline. Full pending queues evict oldest unverified data within per-link/node bounds. Sender-table overflow refuses new entries rather than evicting active senders.

This component emits `Unverified`, `Opaque`, `Pending`, `PendingDuplicate`, `Duplicate` or structural `Control`; no authenticated state, badge, UI/history effect or plaintext crypto fallback exists. Stored CHAT inside a SYNC transport object passes the same logical admission; empty markers do not complete a walk. Initial incoming cursor-zero SYNC requests charge serving admission; the future MC-015 consumer must validate all walk/cursor/sequence state. Locally requested sessions have separate buckets. Work permits reserve the full caller-declared known bundle and at most two concurrent operations, charge failures and grant no trust. A disconnect retains an in-flight work slot until its worker finishes, preventing reconnect from exceeding concurrency. MC-019/020/021 must bind actual verifier operations/results to these limits; their cryptographic tests remain required under the approved decision above.

Native drivers still must bound arrays before the FFI copy. The existing foundation's diagnostic `InboundObserved` is not a protocol/display path; MC-023/026 integrate native intake with this component. Mode-specific active-link selection/teardown remains MC-014/023/026: callers supply the platform ceiling and explicitly disconnect excess links before lowering it. No production platform or crypto integration is fabricated in this ticket.

The simulator's `--core-ingress` mode replaces fixture ingress buckets with this implementation, exposes actual admission/reservation counters and keeps the existing fixture relay/scheduler policy labeled as such. Legacy MC-012 mode remains available. Production relay/SYNC acceptance remains MC-014/015/016.

### Validation evidence

Host: Windows x86_64, Rust/cargo 1.85.1, Python 3.14.4. Commands use repository-local Cargo/Rustup/target/temp paths and locked dependencies; no dependency version or root lockfile changed.

- `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`; `cargo test --workspace --all-features --locked` and `--release`; `cargo build --workspace --all-features --locked --release`: pass locally. The new 14 ingress tests plus deterministic 600-second lifecycle/intake mutation smoke run are included automatically in these existing gates.
- The signed/encrypted ingress fixtures are structural `friend-included`, `organizer-included` and `encrypted-chat-121` from MC-009 vectors; no synthetic signature/ciphertext is declared cryptographically valid. Tests assert pending/unverified state, same-ID variant separation, exact replay, no trusted entry and no private operation.
- The 600-second established-link sender-rotation attack offers 100 values/second/link. One link: 60,000 offered, 614 admitted, sender peak 314. Eight links: 480,000 offered, 4,859 admitted, sender peak 2,459. These fit the declared link/node burst+refill bounds; the cases are reported separately. Admitted records rotate both message IDs and claimed sender IDs. Sender reservation stays 688,152 bytes on this host; accepted state reaches its fixed 4096 ceiling and staging stays bounded. No crypto operation runs.
- Tests also cover staging overflow, work concurrency and atomicity, explicit pending-queue pressure/expiry/eviction, session roles, all control/unknown classes, shared sender budgets, malformed-first/valid-second recovery, cross-link rejected-fragment poisoning, retired partition reclamation, backwards time, oversize/size mismatch, stale tokens, capacity change and reconnect-resistant node/address budgets.
- `cargo fmt --manifest-path tests/simulator/Cargo.toml -- --check`; corresponding locked build/clippy; `python -B tests/integration/simulator/test_simulator.py`: pass. The core-ingress scenario suite uses `python -B tests/simulator/runner.py --core target/release/meshchat-simulator-core.exe --output .work/mc013/ingress-suite --extra tests/simulator/scenarios/MC-012-driver-cases.json --core-ingress`. All 17 cases × three seeds × two policies with exact reruns pass (204 executions), exercising production ingress and retaining fixture relay/scheduler labeling; no production relay acceptance is claimed. Metrics include actual core admission and reservation counters.
- MC-007 definition/worksheet checks, ticketboard validator (46 tickets/127 dependencies), its 12 tests, and `git diff --check` pass. Cargo-deny 0.20.2 passes advisory/license/version/source checks with a freshly fetched RustSec database (only unused license-allowance warnings). Hosted native results remain pending and are recorded before merge. A required failed check still blocks completion.

Required Terra review, final check evidence and squash merge remain pending.

## Review and merge

- Branch: `ticket/MC-013-ingress-budgets-authentication-states-and-dedup`.
- Review/PR: pending.
- Squash commit title: `MC-013: Ingress budgets, authentication states and dedup`.
- Completion becomes effective only when the reviewed squash commit lands on main.
