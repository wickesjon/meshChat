# MeshChat ticketboard

[Active implementation plan](implementation-plan.md) · [Normative design](../mesh-chat-design.md) · [Repository rules](../../AGENTS.md)

## Workflow

`backlog/` → `inprogress/` → `inreview/` → `complete/`

The containing folder is authoritative. A ticket in complete on a branch is staged for merge; completion takes effect only on main after the squash merge. Do not start dependent implementation before that.

Each ticket contains JSON-compatible scalar/list values in a small front-matter block. `depends_on` is the authoritative dependency list. Required content sections are implementation details, exit criteria, potential fallbacks, scope, evidence, and review/merge information.

After edits run `python tests/ticketboard/validate.py --write`; then run `python tests/ticketboard/validate.py` and `git diff --check`. The validator uses only Python's standard library.

## Ticket index

<!-- INDEX:START -->
| Ticket | Task | State | Depends on |
|---|---|---|---|
| [MC-001](complete/MC-001-project-rules-and-executable-ticketboard.md) | Project rules and executable ticketboard | complete | None |
| [MC-002](complete/MC-002-build-layout-and-continuous-integration.md) | Build layout and continuous integration | complete | MC-001 |
| [MC-003](complete/MC-003-sans-io-and-uniffi-foundation.md) | Sans-IO and UniFFI foundation | complete | MC-002 |
| [MC-004](complete/MC-004-two-platform-ble-and-permission-feasibility.md) | Two-platform BLE and permission feasibility | complete | MC-003 |
| [MC-005](complete/MC-005-platform-key-and-encrypted-store-feasibility.md) | Platform key and encrypted-store feasibility | complete | MC-003 |
| [MC-006](complete/MC-006-canonical-wire-and-discovery-contract.md) | Canonical wire and discovery contract | complete | MC-004 |
| [MC-007](complete/MC-007-budgets-and-measurable-mesh-acceptance.md) | Budgets and measurable mesh acceptance | complete | MC-004, MC-006 |
| [MC-008](complete/MC-008-reviewed-dm-and-trust-contract.md) | Reviewed DM and trust contract | complete | MC-005 |
| [MC-009](complete/MC-009-logical-packet-codec-and-golden-vectors.md) | Logical packet codec and golden vectors | complete | MC-003, MC-006 |
| [MC-010](complete/MC-010-bounded-framing-and-reassembly.md) | Bounded framing and reassembly | complete | MC-009, MC-007 |
| [MC-011](complete/MC-011-channels-text-and-deep-link-parsing.md) | Channels, text and deep-link parsing | complete | MC-009 |
| [MC-012](complete/MC-012-deterministic-gatt-overlay-simulator.md) | Deterministic GATT-overlay simulator | complete | MC-003, MC-007, MC-009, MC-010 |
| [MC-013](complete/MC-013-ingress-budgets-authentication-states-and-dedup.md) | Ingress budgets, authentication states and dedup | complete | MC-010, MC-007, MC-012 |
| [MC-014](complete/MC-014-relay-scheduling-suppression-and-power-policy.md) | Relay scheduling, suppression and power policy | complete | MC-013, MC-012 |
| [MC-015](complete/MC-015-forward-cache-and-paginated-sync.md) | Forward cache and paginated SYNC | complete | MC-014, MC-007 |
| [MC-016](complete/MC-016-base-transport-freeze-gate.md) | Base transport freeze gate | complete | MC-011, MC-015 |
| [MC-017](complete/MC-017-identity-provider-and-key-lifecycle.md) | Identity provider and key lifecycle | complete | MC-005, MC-008, MC-003 |
| [MC-018](complete/MC-018-encrypted-persistence-and-retention.md) | Encrypted persistence and retention | complete | MC-017, MC-011 |
| [MC-019](complete/MC-019-verified-friends-and-fresh-presence.md) | Verified friends and fresh presence | complete | MC-017, MC-018, MC-011, MC-008 |
| [MC-020](complete/MC-020-authenticated-encrypted-dms-and-reactions.md) | Authenticated encrypted DMs and reactions | complete | MC-019, MC-008, MC-006, MC-016 |
| [MC-021](complete/MC-021-organizer-trust-credentials-and-signed-updates.md) | Organizer trust, credentials and signed updates | complete | MC-019, MC-006, MC-016 |
| [MC-022](complete/MC-022-full-wire-crypto-review-and-freeze.md) | Full-wire crypto review and freeze | complete | MC-020, MC-021 |
| [MC-023](complete/MC-023-android-gatt-transport.md) | Android GATT transport | complete | MC-004, MC-016, MC-019 |
| [MC-024](complete/MC-024-android-connection-and-power-integration.md) | Android connection and power integration | complete | MC-023, MC-014 |
| [MC-025](backlog/MC-025-android-bench-mesh-and-early-field-gate.md) | Android integrated bench mesh and field gate | backlog | MC-024, MC-015, MC-029, MC-030, MC-031, MC-032, MC-033, MC-042, MC-022 |
| [MC-026](complete/MC-026-ios-corebluetooth-driver.md) | iOS CoreBluetooth driver | complete | MC-004, MC-016, MC-024 |
| [MC-027](backlog/MC-027-cross-platform-radio-interoperability.md) | Cross-platform radio interoperability | backlog | MC-026, MC-023, MC-022, MC-035 |
| [MC-028](complete/MC-028-android-shell-and-channel-chat.md) | Android shell and channel chat | complete | MC-018, MC-023, MC-011, MC-024, MC-022 |
| [MC-029](complete/MC-029-android-friends-and-encrypted-messaging-ui.md) | Android friends and encrypted messaging UI | complete | MC-028, MC-020 |
| [MC-030](backlog/MC-030-android-organizer-ui-and-staff-provisioning.md) | Android organizer UI and staff provisioning | backlog | MC-028, MC-021, MC-041 |
| [MC-031](complete/MC-031-offline-sharing-and-installed-app-link-support.md) | Offline sharing and installed-app link support | complete | MC-011, MC-028 |
| [MC-032](inprogress/MC-032-supporter-entitlements-and-accessible-cosmetics.md) | Supporter entitlements and accessible cosmetics | inprogress | MC-028, MC-006 |
| [MC-033](backlog/MC-033-android-phone-beacon-mode.md) | Android phone Beacon Mode | backlog | MC-024, MC-018, MC-028 |
| [MC-034](backlog/MC-034-android-beta-integration-gate.md) | Android beta integration gate | backlog | MC-025, MC-029, MC-030, MC-031, MC-032, MC-033, MC-042, MC-022, MC-043 |
| [MC-035](backlog/MC-035-ios-feature-parity-and-lifecycle-ui.md) | iOS feature parity and lifecycle UI | backlog | MC-026, MC-022, MC-029, MC-030, MC-031, MC-032, MC-042 |
| [MC-036](backlog/MC-036-integrated-security-and-resource-regression-gate.md) | Integrated security and resource regression gate | backlog | MC-022, MC-027, MC-033 |
| [MC-037](backlog/MC-037-independent-security-assessment-and-remediation.md) | Independent security assessment and remediation | backlog | MC-034, MC-035, MC-036, MC-044 |
| [MC-038](backlog/MC-038-mixed-platform-scale-and-battery-field-validation.md) | Mixed-platform scale and battery field validation | backlog | MC-034, MC-035, MC-036 |
| [MC-039](backlog/MC-039-release-operations-store-readiness-and-user-docs.md) | Release operations, store readiness and user docs | backlog | MC-034, MC-035, MC-037, MC-038 |
| [MC-040](backlog/MC-040-v1-release-acceptance-gate.md) | v1 release acceptance gate | backlog | MC-039 |
| [MC-041](backlog/MC-041-offline-organizer-key-and-credential-tooling.md) | Offline organizer key and credential tooling | backlog | MC-021 |
| [MC-042](backlog/MC-042-honest-contribution-and-power-feedback.md) | Honest contribution and power feedback | backlog | MC-028, MC-024 |
| [MC-043](backlog/MC-043-android-physical-key-and-storage-verification.md) | Android physical key and storage verification | backlog | MC-017, MC-018, MC-029, MC-030, MC-031, MC-032, MC-033, MC-042 |
| [MC-044](backlog/MC-044-ios-physical-key-and-storage-verification.md) | iOS physical key and storage verification | backlog | MC-017, MC-018, MC-035 |
| [MC-045](complete/MC-045-defer-physical-acceptance.md) | Defer physical acceptance until integrated candidates | complete | MC-001, MC-007 |
| [MC-046](complete/MC-046-windows-android-build.md) | Windows Android core and app builds | complete | MC-003, MC-045 |
<!-- INDEX:END -->

## Status interpretation

- **Backlog:** specified, awaiting prerequisites or execution.
- **In progress:** active branch work with complete prerequisites.
- **In review:** implementation and validation evidence ready for review; merge remains pending.
- **Complete:** accepted work squash merged to main.

MC-001 contains this approved planning change. All other tickets initially remain backlog; none implies that application code, physical testing or security review has already been completed.
