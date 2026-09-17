---
id: "MC-033"
title: "Android phone Beacon Mode"
depends_on: ["MC-024","MC-018","MC-028"]
kind: "product"
branch: "ticket/MC-033-android-phone-beacon-mode"
---

# MC-033 — Android phone Beacon Mode

## Objective

Implement the beacon profile with bounded anti-abuse controls, extended cache, runtime-enforced connection limits and external-power infra hint.

## Dependencies

`MC-024`, `MC-018`, `MC-028` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/android/power/**`, `src/android/ui/**`, `tests/bench/beacon/**`, `docs/testing/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

### Scope extension approved — 2026-09-17

Dependencies are complete on main at `a232082c243c4ab0d7005c44b8f20b75eabcb221`. Implementation uses the dedicated branch created from that main revision. Inspection found that the existing shared power policy supports Beacon, but `NativeTransport` exposes only Auto/Normal/Saver and hardcodes auto-beacon off. Its Android radio adapter cannot yet select Beacon scanning or report its power transition. The existing bounded forward cache also needs integration with the native owner, and ANNOUNCE must reflect external power.

User-approved narrow scope extension: shared core Beacon integration in `src/core/src/native_transport.rs`, `src/core/src/native_channels.rs`, `src/core/src/power.rs`, `src/core/src/sync.rs` and a dedicated `src/core/src/native_beacon.rs` helper if needed; `src/core/Cargo.toml` test registration; Android radio wiring in `src/android/ble/src/main/java/org/meshchat/transport/**`; `src/android/app/build.gradle.kts` source/test registration and app activity wiring if required for the dim status screen; affected native-consumer regression tests under `tests/integration/**`; and `.github/workflows/ci.yml` for the Beacon checks. Original permitted paths remain available. No wire-format change, new protocol privilege, weaker limit, or physical-evidence waiver is requested. Only necessary Beacon changes within these additional paths would be made.

The user explicitly approved this extension with “yes” on 2026-09-17. Implementation started after approval; all other scope and acceptance boundaries remain unchanged. Verification will cover six logical hours with bounded scheduling/cache, transitions without resetting protected state or budget credits, power-only infra hints, runtime link ceilings, native builds, and the dim/hold-to-exit UI. MC-025 retains the actual six-hour endurance and coverage/battery measurements.

- Implement the beacon profile with bounded anti-abuse controls, extended cache, runtime-enforced connection limits and external-power infra hint.
- Build dim burn-in-safe status, hold-to-exit, auto-beacon charging behavior and battery auto-downgrade.
- Target retired Android phones for v1; provide the bridge/load measurement procedure for MC-025 without assuming battery offload or measured device capacity during implementation.

## Exit criteria

- [ ] Automated native/core integration exercises six logical hours of beacon scheduling/cache operation with bounded state; applicable native checks pass. MC-025 retains the six-hour powered physical-phone endurance test.
- [ ] Disconnecting power and reaching the downgrade threshold produce the approved behavior without lost core state.
- [ ] A reproducible sparse-gap, beacon/no-beacon comparison procedure and aggregate instrumentation are prepared for MC-025; physical coverage and phone battery/load results remain pending there.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If a device cannot support eight links, use its measured limit and document deployment capacity.
- ESP32, LoRa and backbone work remain follow-on scope; do not fork the protocol for this ticket.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Scheduling approved 2026-09-14 in [the validation policy](../../decisions/local-validation-policy.md#physical-acceptance-scheduling--approved-2026-09-14). Six-hour endurance, actual power removal/downgrade and sparse-gap/battery evidence transfer to MC-025. Synthetic time progression is not a physical endurance measurement.

Implemented: existing Beacon core policy, bounded native cache/SYNC serving, power-only infra hint, aggregate diagnostics, Android scanning/slot integration and protected preferences, dim moving status and hold-to-exit. See [MC-033 implementation and bench procedure](../../testing/MC-033-beacon-mode.md). Four native Beacon tests pass, including six logical hours; initial Android Debug/Release, lint and eighteen JVM tests pass. Final full Rust/security checks, Android UI restart/exit and Mac native checks are in progress. The old relay fixture now expects immediate auto-beacon charging entry; the channel fixture selects its labeled theme switch after a second Settings switch was added. No physical evidence or independent review is claimed yet.

## Review and merge

- Branch: `ticket/MC-033-android-phone-beacon-mode`.
- Review/PR: pending.
- Squash commit title: `MC-033: Android phone Beacon Mode`.
- Completion becomes effective only when the reviewed squash commit lands on main.


