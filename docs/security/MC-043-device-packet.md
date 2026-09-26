# MC-043 physical Android acceptance packet

Status: preparation only, 2026-09-26. No device scenario has been executed for this packet. [MC-043](../ticketboard/inprogress/MC-043-android-physical-key-and-storage-verification.md) remains an open physical gate. This preparation starts from main and does not depend on the unmerged MC-025 performance candidate.

## Prepare the records

From the repository root:

```powershell
python -B tests/bench/security/physical_packet.py init .work/mc043/preparation-2026-09-26/packet.json
python -B tests/bench/security/physical_packet.py check .work/mc043/preparation-2026-09-26/packet.json
```

Initialization creates a new file with 34 `not_run` records: 17 scenarios for each minimum/current device slot. It refuses to overwrite an existing packet. The tool has no device executor: it cannot install, reset, change credentials, invoke ADB or inspect a phone. Keep each later execution in a separate directory and preserve its original evidence.

Use `not_run` for an untouched scenario, `unavailable` with a concrete reason for a missing prerequisite, and `fail` or `pass` only after observing the actual outcome. Unavailable cases remain blockers. A successful structure check is not a test result. Even an all-pass packet returns `certified: false`; independent review of evidence and the ticket's acceptance criteria remains necessary.

## Inventory and execution prerequisites

The minimum slot requires a physical API 29 device. Identify the current supported API through a dated support decision when testing, then record `current_supported_api`, ISO `current_support_date` and `current_support_reference` (an existing repository-relative file, at most 512 characters); the current slot must match it. Neither the plugged-in phone nor tablet is presumed to satisfy these slots without inventory. Record model, full OS build, physical-device status, secure-lock configuration, source commit, both app and instrumentation APK SHA-256 hashes, and signing kind. A debug or probe build must not be represented as release-candidate certification. Each executed row needs date, elapsed seconds, an observation and existing repository-relative evidence paths.

Use the [MC-005 procedure](../../tests/bench/security/README.md), [probe plan](../decisions/MC-005-probe-plan.md), normative design and [validation policy](../decisions/local-validation-policy.md) to bind the execution to the production provider and persistence adapter. Old platform-curve experiments are supporting capability evidence only. Existing emulator runners deliberately reject physical devices; do not remove those guards to manufacture a physical run. Before testing, prepare and review a physical execution session using the exact candidate and its authorized fixture actions.

Use a dedicated synthetic fixture. Start with nondestructive checks. Before credential changes, key deletion, resets, restore or uninstall operations, identify the exact fixture/profile and obtain execution authorization if not already authorized for that session. Preparation authorization does not authorize these operations on existing personal profiles. Some checks need only one device; other-device restore and transfer need the corresponding second device and supported facilities.

## Scenario matrix

Run every row on both device slots. Evidence must describe the trigger, observed outcome, actual adapter/build and any limitation; a test name or source inspection alone is insufficient.

| Packet scenario | Procedure and evidence to retain |
| --- | --- |
| `production_curves` | Exercise production signing and agreement and verify outputs. Distinguish production wrapped-software operations from optional platform-curve probes; unsupported platform curves do not by themselves disprove the selected software provider. |
| `wrapping_metadata` | Read actual protection/security-level metadata for the wrapping key and test its enforced access conditions. Record hardware/software backing honestly rather than inferring it from device model. |
| `encrypted_create` | Create a synthetic encrypted fixture through the production adapter; inspect database, journal/WAL/SHM and wrapped-secret handling without retaining secret content. |
| `restart_wrong_correct` | Restart, attempt wrong-key reopen, then correct-key reopen. Demonstrate preserved ciphertext and identity, refusal with the wrong key, and successful recovery without silent replacement. |
| `lock_closed` | Close the store, lock the device and attempt production access. Record execution availability and actual refusal, then recovery after unlock. |
| `lock_inflight` | Lock during a controlled production operation; record timing, completion/refusal and subsequent access behavior against the protection policy. |
| `held_handle_limit` | Observe a deliberately held database handle separately from production per-operation closing. Document residual held-handle access limitations; do not claim lock erases already available plaintext. |
| `reboot_before_unlock` | Reboot and attempt the bound operation before first unlock, then after unlock. Suspended instrumentation or absent callbacks are unavailable evidence, not demonstrated refusal. |
| `key_loss` | Delete only the authorized fixture wrapping key. Show existing encrypted artifacts remain and production access refuses without recreating identity. Distinguish this manual deletion from OS invalidation. |
| `os_invalidation` | Trigger a supported OS invalidation condition on the authorized fixture. Record the actual trigger and platform response; if unavailable, document why and leave the gate open. |
| `explicit_reset` | Perform an explicitly authorized reset after key loss; demonstrate removal of stale artifacts/pins and handles, a fresh identity generation, and no implicit reset on earlier failures. |
| `cloud_backup` | Exercise the supported backup facility and inspect restored/excluded database, sidecars and wrapped-secret artifacts. Configuration declarations alone are insufficient. |
| `device_transfer` | Exercise supported device transfer; verify exclusions and safe refusal of mismatched material on the recipient using exact source and destination inventories. |
| `same_device_restore` | Restore the authorized fixture on its original device; verify required exclusions and safe handling of absent or mismatched wrapping material, including auxiliary files. |
| `other_device_restore` | Restore on the other identified device; verify no identity takeover or silent replacement and safe mismatch refusal. Link both device inventories. |
| `uninstall_reinstall` | Uninstall and reinstall the identified test fixture; inspect resulting key/artifact state and expected fresh provisioning or safe refusal. Never substitute a production personal profile. |
| `sidecars_logging` | Capture actual operation logs and inspect database sidecars, backup/restore remnants and errors from the above runs. Confirm no private key, provisioning QR or message content is emitted or retained in shareable evidence. |

Candidate-specific changes need supplemental checks when present. For example, the unmerged MC-025 raw-key envelope migration should receive legacy-to-v2 reopen, interruption and identity-preservation checks on that candidate. Link supplemental evidence to the relevant lifecycle row; this packet does not assert that change exists on main or add an implicit dependency on its PR.

## Evidence and review

Keep sanitized metadata and observations in ignored repository-local `.work`. Do not commit private keys, provisioning images, messages or database copies. Shareable reports should contain only sanitized outcomes, artifact hashes and protection metadata. Evidence paths are checked for existence and repository containment; their contents, signing suitability and truth are reviewed by a person or independent reviewer, not certified by the checker.

After execution, run the checker and retain its output with the packet. Review all failures, unavailable scenarios, cohort/build mismatches and limitations before proposing a support decision. Any production fix requires a separately scoped ticket and retest. Update normative support decisions from reviewed results; neither this guide nor a valid JSON packet waives minimum/current physical coverage, independent security assessments or release gates.

Host regression checks for this preparation:

```powershell
python -B -m unittest discover -s tests/bench/security -p test_physical_packet.py
python -B tests/ticketboard/validate.py
git diff --check
```
