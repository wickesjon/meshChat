---
id: "MC-041"
title: "Offline organizer key and credential tooling"
depends_on: ["MC-021"]
kind: "product"
branch: "ticket/MC-041-offline-organizer-key-and-credential-tooling"
---

# MC-041 — Offline organizer key and credential tooling

## Objective

Build offline root/key generation and staff credential issuance using the canonical core formats; require explicit input for validity and labels.

## Dependencies

`MC-021` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/organizer-tools/**`, `tests/integration/organizer-tools/**`, `docs/organizer/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

### Scope extension approved — 2026-09-17

Dependency MC-021 is complete on main. This branch starts from `a3ccac421f0ec3bfb0fbafabbc454bd09be8e7b2`, after reviewed MC-033 was squash merged. Implementation started after the scope approval below.

The offline tool will be a separate Rust workspace package using the existing pinned crypto dependencies and canonical core parsers/verification. Root and staff generation require explicit expiry/validity and labels; production randomness comes from the operating system. Public adoption data may be exported; private provisioning is an explicit local one-time operation, with no secret in command arguments, logs, tracked artifacts or relay state. Tests will generate synthetic inputs only, reject invalid/expired/mismatched chains, and rehearse two staff identities through a relay that does not adopt the root. Full mobile UI/protected staff lifecycle remains MC-030/035; this ticket still requires native import fixtures.

Approved narrow extension: `Cargo.toml` and `Cargo.lock` for workspace registration and the new package lock entry; `.github/workflows/ci.yml` and `src/core/build_bindings.py` for selecting the existing mobile core explicitly and registering the offline/native fixture checks; `src/core/src/security_probe.rs` for a test-only adapter to the canonical organizer import path if needed by Kotlin/Swift fixtures; and `src/android/security/build.gradle.kts` for registering those fixture sources under this ticket's existing test directory. The user approved this extension with “yes” on 2026-09-17 before implementation. No new production organizer API, wire format, trust rule, independent-security waiver or physical acceptance waiver is authorized.

- Build offline root/key generation and staff credential issuance using the canonical core formats; require explicit input for validity and labels.
- Produce public adoption QR data and controlled one-time staff provisioning display without committing private keys or QR images.
- Implement validation/dry-run output and documented rotation/expiry procedures; keep signing keys off relay beacons.

## Exit criteria

- [ ] Tool-generated public/staff bundles round-trip through core verification and mobile import fixtures.
- [ ] Expired, mismatched and malformed credentials are rejected; private material is absent from logs and tracked outputs.
- [ ] An offline rehearsal provisions two staff identities and verifies their updates through a non-trusting relay.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If safe QR display/export is unavailable, provide a controlled local workflow and block staff provisioning UI integration until verified.
- Do not use a shared permanent root private key on staff phones or beacons as a shortcut.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Implemented: a separate offline Rust issuer and local Python/Tk console, explicit UTC dates and labels, an authenticated encrypted root vault with a separately recorded random unlock key, public adoption data export and an in-memory 60-second staff QR display. Existing files are never overwritten; no root private key crosses to the console, no staff QR/seed is saved by the console, and no secret is supplied through command arguments or logged in errors. See [operator workflow, format, rotation, limitations and rehearsal](../../organizer/offline-console.md). Root/staff wire formats and production trust APIs are unchanged. The only shared API addition is a synthetic test-only adapter under the existing security-probe feature.

Initial validation: four Rust organizer tests pass, including encrypted root/import negatives, generation of disposable native fixtures, and two independently issued staff identities whose signed updates cross the actual bounded non-adopting relay scheduler and verify at an adopted receiver. Two executable pipe-protocol/export tests and the hidden Tk layout/masked-input/QR quiet-zone/clear check pass. Strict workspace clippy passed before the last fixture assertion/canonical-coordinate alignment. Final full Rust/security and Android/Swift native checks are in progress. No physical phone, hardware-backed storage, complete memory-erasure or screenshot-prevention result is claimed. The pinned matrix-only qrcode 0.14.1 dependency adds no image/file/network capability; existing crypto dependency versions are unchanged.

## Review and merge

- Branch: `ticket/MC-041-offline-organizer-key-and-credential-tooling`.
- Review/PR: [PR #35](https://github.com/wickesjon/meshChat/pull/35). Separate `gpt-5.6-terra` medium review of `9b6bb55902020e0b0d459cb13a8497ed93b7b6ae` found one blocker: issuance did not enforce the design's mandatory event-end plus 24-hour bound. The fix requires an explicit event end, authenticates it inside the local vault, bounds root and staff expiry, and adds exact-boundary/overflow tests. No other source findings were reported. Follow-up review and final evidence remain pending. The dependency gate also identified unversioned local core dependencies; these now use the exact workspace package version without changing any dependency version.
- Squash commit title: `MC-041: Offline organizer key and credential tooling`.
- Completion becomes effective only when the reviewed squash commit lands on main.
