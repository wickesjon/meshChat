---
id: "MC-043"
title: "Android physical key and storage verification"
depends_on: ["MC-017","MC-018"]
kind: "gate"
branch: "ticket/MC-043-android-physical-key-and-storage-verification"
---

# MC-043 — Android physical key and storage verification

## Objective

Complete the physical protected-key and encrypted-storage verification deferred from MC-005 by the user on 2026-09-11, using the production provider and persistence implementation before MC-034 Android beta.

## Dependencies

`MC-017`, `MC-018` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `tests/bench/security/**`, `docs/decisions/**`, `docs/security/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. Production remediation requires a separate scoped ticket; do not change provider protections to manufacture a pass. Read the [design](../../mesh-chat-design.md) and [active plan](../implementation-plan.md).

## Implementation details

- Run the [MC-005 procedure](../../../tests/bench/security/README.md) and equivalent production-adapter checks on named Android API 29 and current supported OS devices. Record model, OS, exact source/build, date, secure-lock setup and actual protection metadata.
- Verify curve operations, protected wrapping, encrypted fixture restart/wrong-key/correct-key reopening, ciphertext-preserving key-loss refusal and explicit reset. Distinguish manual key deletion from OS-triggered invalidation.
- Verify closed and held-database behavior across lock, reboot and before first unlock; record delayed/suspended execution honestly. Exercise backup, supported device transfer, uninstall/reinstall and same/other-device restore, inspecting database, sidecars and wrapped secrets without retaining private content.
- Confirm or revise the provisional provider/support matrix from actual results; retain sanitized evidence and retest after any scoped remediation.

## Exit criteria

- [ ] Minimum/current OS physical curve and wrapping capability results are recorded for the production implementation; unsupported capabilities are explicit and no emulator or unsigned build is called hardware evidence.
- [ ] Encrypted lifecycle and key-loss/reset tests pass; OS invalidation, lock/reboot/before-first-unlock behavior meets the documented protection policy, including held-open database limitations.
- [ ] Backup/device-transfer, uninstall/reinstall and restore checks demonstrate the required exclusions and safe mismatch refusal, including auxiliary files; no secret/message content is logged.
- [ ] The supported-device/protection decision is backed by results, failures are remediated/retested, and evidence permits the corresponding platform's sensitive-data use subject to other security/release gates.
- [ ] Relevant checks pass, required separate code/evidence review is recorded, and the ticket is squash merged to main.

## Potential fallbacks

- Missing devices, signing access or restore facilities blocks this gate; deferral and online/emulator evidence cannot satisfy it.
- If the development protection model fails, open scoped remediation or an explicit support decision. Never fall back to plaintext, silent identity replacement or undocumented weaker key protection. Independent security assessments remain separate requirements.

## Evidence

Deferred, not tested. This ticket is a mandatory later gate created under the user's 2026-09-11 scheduling decision. Device/signing inventory is unknown; no physical result is claimed. The early MC-005 emulator/source evidence is recorded in [the probe plan](../../decisions/MC-005-probe-plan.md).

## Review and merge

- Branch: `ticket/MC-043-android-physical-key-and-storage-verification`.
- Review/PR: pending.
- Squash commit title: `MC-043: Android physical key and storage verification`.
- Completion becomes effective only when the reviewed squash commit lands on main.
