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

- [x] Automated native/core integration exercises six logical hours of beacon scheduling/cache operation with bounded state; applicable native checks pass. MC-025 retains the six-hour powered physical-phone endurance test.
- [x] Disconnecting power and reaching the downgrade threshold produce the approved behavior without lost core state.
- [x] A reproducible sparse-gap, beacon/no-beacon comparison procedure and aggregate instrumentation are prepared for MC-025; physical coverage and phone battery/load results remain pending there.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If a device cannot support eight links, use its measured limit and document deployment capacity.
- ESP32, LoRa and backbone work remain follow-on scope; do not fork the protocol for this ticket.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Scheduling approved 2026-09-14 in [the validation policy](../../decisions/local-validation-policy.md#physical-acceptance-scheduling--approved-2026-09-14). Six-hour endurance, actual power removal/downgrade and sparse-gap/battery evidence transfer to MC-025. Synthetic time progression is not a physical endurance measurement.

Implemented and tested: existing Beacon core policy, bounded native cache/SYNC serving, power-only infra hint, aggregate diagnostics, Android scanning/slot integration and protected preferences, dim moving status and hold-to-exit. See [MC-033 implementation, commands, versions, APK hashes and bench procedure](../../testing/MC-033-beacon-mode.md). Rust formatting, strict clippy, 193 debug and 193 release tests (including four native Beacon tests), release build, fourteen storage-policy tests and cargo-deny pass on core revision `c47c028448d64e8d4b19df3d8b4aafdecac70f31`. Generated bindings and both Android ABIs pass. Android Debug/Release/test APK, lint and eighteen JVM tests pass after the final gesture fix; BLE builds/lint/twenty-six JVM tests pass on unchanged core/driver inputs. Both app APKs pass native-set, ELF and ZIP 16 KiB alignment checks.

The [full Mac native job](https://github.com/wickesjon/meshChat/actions/runs/35251216171/job/105303956941) passed on that core revision, including Swift FFI, Beacon, iOS driver and Supporter regressions, app/BLE Debug/Release and security-probe device/simulator builds and crypto checks. Subsequent Android UI/test/evidence changes do not alter any Mac/core build input. Hosted Android source download failed with HTTP 503 before its unchanged Compose graphics dependency could be provisioned; local pinned native dependencies and native/emulator checks supply applicable evidence under the approved validation policy. This is not a claim that the failed hosted job passed.

Final API29 x86_64 emulator checks pass at 320x640/density160: three channel phases and four Beacon phases, including actual 2.3-second Android touch, ordinary tap rejection, accessible exit, protected restart and brightness restoration. Earlier friend (three), sharing (two) and synthetic billing (three) phases pass; their source inputs are unchanged by the final isolated Beacon gesture fix. The runner now waits for the protected save to complete before force-stopping the app; a fixed one-second delay produced a false persistence failure. The pointer handler uses a stable key/latest callback so status recomposition does not restart a hold. The old relay fixture expects immediate auto-beacon charging entry; the channel fixture selects its labeled theme switch after a second Settings switch was added. No physical evidence is claimed. Independent review findings and follow-ups are recorded below.

## Review and merge

- Branch: `ticket/MC-033-android-phone-beacon-mode`.
- Review/PR: [PR #34](https://github.com/wickesjon/meshChat/pull/34). Separate `gpt-5.6-terra` medium review at `336fcd7189b16846aaff49d032dd0fc4d6fc6b30` found one P1: enabling/restoring Beacon preferences did not start an absent transport service. The fix starts the existing service through its unchanged permission/lock checks. No other actionable findings; final follow-up review and evidence remain pending. Native validation also found a missing accessible exit grouping and a Swift compatibility expectation for the existing SYNC-request diagnostic event; both are corrected. Follow-up at `bd99267b36b1859ab5cad6c44849034cb25b5044` found a P2: preserving the diagnostic event also observed direct SYNC requests in relay state. Revision `c47c028448d64e8d4b19df3d8b4aafdecac70f31` emits that event and returns before relay/digest/cache/activity effects. The worker confirmed both findings resolved with no additional source findings. Final metadata/evidence review remains pending.
- Squash commit title: `MC-033: Android phone Beacon Mode`.
- Completion becomes effective only when the reviewed squash commit lands on main.
- Final staged completion is pending the exact-head Terra review and squash. The combined review/merge checkbox remains unchecked until those events occur; the actual reviewed revision and result are recorded in PR #34 before merging, and main's squash history establishes completion. The separate gesture review found no behavioral issue; its initial missing-import concern was withdrawn after confirming the timeout is an `AwaitPointerEventScope` member and the final Android build passed.
