---
id: "MC-023"
title: "Android GATT transport"
depends_on: ["MC-004","MC-016","MC-019"]
kind: "android"
branch: "ticket/MC-023-android-gatt-transport"
---

# MC-023 — Android GATT transport

## Objective

Implement scanner/advertiser, GATT server/client, CCCD setup, concrete UUIDs and runtime write/notify flow control.

## Dependencies

`MC-004`, `MC-016`, `MC-019` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/android/ble/**`, `tests/bench/android/**`, `docs/testing/**`, `src/core/src/native_transport.rs`, `src/core/src/lib.rs`, `src/core/Cargo.toml` (test registration only), `tests/integration/transport/**`, `tests/integration/ffi/**`. The additional core/binding paths are limited to the user-approved [native transport boundary](../../testing/MC-023-native-core-scope-proposal.md); no dependency or wire change is authorized.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Implement scanner/advertiser, GATT server/client, CCCD setup, concrete UUIDs and runtime write/notify flow control.
- Feed per-direction capacity and link lifecycle events to the core; preserve contiguous fragment transfer and disconnect cleanup.
- Integrate the foreground service and approved permission manifest for supported Android versions.

## Exit criteria

- [ ] Automated native-adapter tests exchange whole/fragmented packets in both directions and exercise runtime capacity/backpressure through controlled callbacks; applicable Android build/lint/tests pass. Two-device physical exchange and measured backpressure remain mandatory in MC-025.
- [ ] Notify subscription, busy/error callbacks, stalled writes, MTU changes and disconnects recover as specified.
- [ ] The native driver forwards untrusted bytes without duplicating core parsing.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If a device exposes a smaller capacity, use the agreed supported-capacity behavior rather than oversized sends.
- On stalled links cancel bounded work and reconnect via policy; avoid unbounded retry loops.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Scheduling approved 2026-09-14 in [the validation policy](../../decisions/local-validation-policy.md#physical-acceptance-scheduling--approved-2026-09-14). Physical driver acceptance transfers to MC-025; implementation evidence must distinguish controlled callbacks/emulation from physical radio results.

Started on the dedicated branch from main `835a7966f4edb2dead99f5768ec24b75059a337c`; all three dependencies are complete. No production implementation or test pass is claimed yet.

Integration scope approved 2026-09-16: generated Kotlin exports only the foundation byte-count/send boundary; the existing Rust HELLO/framing/ingress owners are not exposed to native callers. The user approved the [scoped core bridge and shared binding tests](../../testing/MC-023-native-core-scope-proposal.md), including the MC-019 dependency. Rust retains protocol parsing and admission; Kotlin owns native callbacks and flow control. Shared exported API changes require both Kotlin and Swift checks. MC-022's independent assessment remains a separate gate.

### Initial bridge work — incomplete ticket

`NativeTransport` now wraps the existing Friends, Ingress and Relay owners with generation-scoped connection admission, runtime directional capacity checks, fresh HELLO construction, short-lived provider proof operations, bounded send effects, completion tokens and disconnect/timeouts. Received logical effects explicitly remain unverified/opaque/pending; no display or delivery authority is inferred. The scheduler currently uses a conservative 146-byte encoding ceiling on every admitted link, while HELLO advertises measured native limits and ingress enforces the negotiated direction. This preserves the existing scheduler's pacing/credits and eight-fragment bound without claiming higher throughput. Android Normal mode is the current six-link boundary; later power integration remains MC-024.

Initial Windows evidence: Rust/cargo 1.85.1, Python 3.14.4, JDK 17.0.15+6, Kotlin 2.2.0/Gradle 8.13. Five new Rust integration tests pass (real HELLO/proof; bidirectional whole/fragmented traffic; capacity refusal; bounded connection admission; paced retry, timeout, stale-token and disconnect/clock cleanup). Workspace all-feature compilation and clippy with warnings denied pass. `python -B src/core/build_bindings.py host` generates Kotlin and Swift bindings successfully. The two `NativeTransportTest` Kotlin tests pass through the actual Android `:app:testDebugUnitTest` task, using a synthetic plaintext SQLite callback double. Initial SDK-path and nullable-Java warnings were corrected; no failed invocation is counted as a pass. Logs are under `.work/mc023/`.

This is unfinished implementation, not merge evidence for the full ticket. Production Android GATT/scanner/advertiser/service wiring and controlled callback coverage, Swift execution/Mac native checks, release/security validation and required Terra review remain outstanding. No physical-device result, ready PR or completion is claimed. The MC-022 assessor has separately returned findings; their remediation needs that ticket's own production-path scope decision.

## Review and merge

- Branch: `ticket/MC-023-android-gatt-transport`.
- Review/PR: pending.
- Squash commit title: `MC-023: Android GATT transport`.
- Completion becomes effective only when the reviewed squash commit lands on main.
