# MC-022 independent remediation retest

Assessment date: **2026-09-16 (America/Los_Angeles)**.

Assessor: **Codex automated AI assessment worker `/root/assess_mc022`**, separately assigned from the implementation author and routine Terra PR reviewers. This worker authored the original construction assessment and independently retested the approved remediation. This is automated construction-assessment evidence, not human or external-firm certification, a mathematical proof, or a claim that every release gate is complete. No remediation was implemented by this worker.

**Recommendation: the scoped remediation resolves F-01, F-02 and A-01 at `695fbc44e472e1f6a5664c9ec0cb3b8a0cc8f792`; no new blocking construction defect was found in this retest.** The original recommendation against freezing on account of those findings is lifted for this revision. A-02 remains an explicit native integration obligation, with no newly demonstrated remote bypass. Native compilation/SQLCipher execution, physical acceptance and the later MC-037 assessment remain separate evidence and gates. This report does not move the ticket or authorize treating unavailable evidence as passed.

## Inputs, scope and attribution

| Input | Identification |
|---|---|
| Exact production and committed-test candidate | `695fbc44e472e1f6a5664c9ec0cb3b8a0cc8f792` |
| Original assessed candidate | `ad87cf890459cd43e1e10991085440ebd6d48cb9` |
| Original production baseline | `835a7966f4edb2dead99f5768ec24b75059a337c` |
| New source snapshot | `C:/Users/wicke/Code/meshChat/.work/assessments/mc022/retest-source` |
| Disposable independent-probe copy | `C:/Users/wicke/Code/meshChat/.work/assessments/mc022/retest-probe-source` |
| Approved scope | Candidate `docs/testing/MC-022-assessment-remediation.md` |
| Original assessment | `report.md`, SHA-256 `d58bca2d19b8d7c6c45af08cbc1c99467100af9f93937c63375736b185638591` |
| Retest source inventory | `retest-inputs.json`, SHA-256 `5fbd973e92e72a1dfd6ecb3db5015d2841b712d09020ddc142400af555bb8c68` |
| Independent test modifications | `retest-probe-modifications.json` and `retest-probes.patch` |

The inventory records each of the 238 tracked source files' Git blob IDs and actual on-disk SHA-256 hashes. The supplied snapshot contains 236 CRLF transformations relative to LF Git objects. Every file matches its explicit candidate Git object after the documented line-ending normalization; it is not claimed to be raw byte-identical to Git. `retest-inputs.py` reproduces this verification and checks that the production diff consists of exactly five files: `friends.rs`, `ingress.rs`, `organizer.rs`, `storage.rs` and `text.rs`. Root manifests and lockfile are unchanged. No protocol encoding, primitive selection or dependency update is part of the remediation.

The original report, snapshots and probes were preserved. The new independent probes modify only four test files in the disposable retest copy; hashes before and after and a patch are retained. Its production source is unchanged. Build output, generated bindings and synthetic database files remain in ignored repository-local paths. The parent checkout, unrelated MC-023 work, branch, Git state, remote services and ticket status were not modified.

This is a focused retest and introduced-risk assessment, read together with the original full construction report. The attacker model is unchanged: arbitrary mesh traffic and malicious authenticated peers/staff, including replay, conflicting same-ID packets, cache poisoning attempts and forbidden text, against trusted local provider/transaction boundaries. Methods included reading the complete remediation diff and affected owner/transaction flows, independently constructed negative cases through production APIs, fresh execution of committed integration and storage checks, and reference-fixture reproduction. Existing helper code supplies links, synthetic identities and SQLite callbacks; the additional scenarios and assertions are assessor-authored.

## Finding dispositions and concrete evidence

### F-01 — Medium replay defect: resolved for the assessed revision

Candidate `src/core/src/storage.rs:560` chooses a canonical ledger direction for public signed subject domains 1 and 3, while `:564` reads both legacy directions. A matching digest in every existing row yields Replay; any contradictory digest yields Conflict, with the conflict update covering both directions at `:577`. These operations remain within the existing transaction. New public ledger records use direction 0; history retains its actual incoming/outgoing direction. DM lookup and identity still include direction (`:567`, `:579`).

Independent reproduction: `retest-probe-source/tests/integration/organizer/organizer.rs:22`, `retest_assessor_original_loopback_and_legacy_conflict`. An actual imported staff key signs and stores an outgoing post. Its returned packet is tested after deleting history, dropping owners and reopening the database, against (a) a new canonical ledger row, (b) only a legacy direction-1 row, (c) both legacy rows with the same digest and (d) contradictory legacy digests. Returned matching packets are Replay in the first three cases; contradictory legacy records are Conflict. A genuinely re-signed changed packet sharing the identity is Conflict. None creates replacement history, ledger counts remain stable, and conflict flags cover both legacy rows. Injected failure in the conflict update rolls back without partially changing either flag; retry then succeeds as Conflict.

Additional independent isolation checks: `organizer/organizer.rs:85` accepts identical message IDs from two different authenticated staff keys, preserving separate histories. `dm/dm.rs:17` sends actual HPKE-authenticated messages with the same ID in opposite directions between two nodes: each receiver accepts its peer's message and retains distinct history directions. An authenticated reaction targeting that now-ambiguous ID causes no reaction effect. Fresh FFI storage checks exercise both public domain prefixes, legacy ambiguity and direction-preserving DM behavior.

Compatibility risk is explicit and accepted by the approved remedy: existing duplicate history or conflicting ledger rows are retained, not silently rewritten. A downgrade to an older vulnerable binary is not protected by this change. No schema migration was introduced. Subject/root/staff separation, type/ID binding and the two-row bound remain intact.

### F-02 — Low pin-policy conformance defect: resolved

Candidate `src/core/src/organizer.rs:670` retains verified post authority while assigning no pin expiry when the claimed expiry exceeds either credential or root authority. Original signed bytes are preserved. `current` at `:719` still applies present-time credential/root validity; outgoing signing continues to refuse an excessive pin claim.

Independent reproduction: `organizer/organizer.rs:67`, `retest_assessor_pin_suppression_survives_cached_credential_and_time`. A correctly signed post with `u32::MAX` pin expiry is accepted with valid organizer attribution but no pin authority. A subsequent post omitting the credential, using its authenticated cache entry and again exceeding its expiry, is also accepted with no pin. The first post never regains pin authority as time advances; organizer authority expires at the credential boundary. Exact raw history is retained. This confirms suppression rather than rejection or expiry clipping, including the cached-credential route.

### A-01 — Clear-text validation ambiguity/gap: resolved at the core acceptance boundary

Candidate `src/core/src/text.rs:54` centralizes existing text validation for clear CHAT nickname/body, ANNOUNCE nickname and EVENT_INFO name. Friend completion (`friends.rs:510`) and organizer completion (`organizer.rs:658`) validate after signature authentication and before cache, ledger, history or trust effects. Invalid signed variants are resolved as failed without poisoning acceptance of a later valid variant. Unsigned ingress validates before accepted deduplication (`ingress.rs:1161`). Outgoing organizer signing validates at `organizer.rs:811`. Parsing remains structural, and incoming signed bytes are not normalized or re-encoded. MC-008 now names this ownership boundary.

Independent reproduction: `friends/friends.rs:21` tests all 78 forbidden code points in the existing policy in both authenticated CHAT and ANNOUNCE nicknames (156 separately signed cases). All reject before signature-key cache or ledger effects. `friends/friends.rs:45` authenticates a forbidden NUL body from an unpinned unknown key and confirms it cannot warm the key cache; the omitted-key follow-up cannot exploit such a cache, and a valid included-key same-ID packet subsequently succeeds without friend attribution. `organizer/organizer.rs:96` rejects a signed forbidden-text pin post with no credential-cache, history or ledger effect, then accepts a valid same-ID variant with appropriate authority. `ingress/ingress.rs:6` rejects unsigned ANNOUNCE/EVENT_INFO NUL names before accepted deduplication and permits valid same-ID replacements. Unknown opaque content and encrypted CHAT are preserved for their intended later owners.

The exhaustive code-point probe covers the two nickname routes; it is not described as exhaustive over every field or Unicode string. Body NUL and organizer/unsigned field probes complement the committed Unicode and field-validation tests. Native rendering still owns safe display and affordances; rejection at the core boundary does not certify arbitrary native renderers or historical content already stored before the fix.

### A-02 — Provider and native work-budget boundary: unchanged integration obligation

The remediation does not change identity/provider loading or friend confirmation. The original concern remains: native owners must budget direct identity/QR/provider operations, honor authenticated acceptance outcomes, and clear trust on lock/invalidation instead of assuming every exported entry point reserves ingress work. No new remote bypass was demonstrated. This was an integration obligation, not a confirmed cryptographic break, and this report does not claim it tested or completed pending native feature wiring. It remains visible for that work and later assessment; it is not reclassified as an unapproved prerequisite to the scoped core remediation.

## Coverage and introduced-risk review

| Area | Retest coverage and limits |
|---|---|
| Public replay identity, durable effects and transactions | Source analysis; independent real organizer round trip, reopen/history deletion, both legacy directions, digest ambiguity and rollback; FFI policy tests for domains 1 and 3. No native SQLCipher execution by this worker. |
| DM direction, identity and reaction ambiguity | Independent real bidirectional HPKE traffic sharing an ID and ambiguous authenticated reaction; original direction-specific storage logic preserved. |
| Organizer credentials, pin expiry and cache | Independent included/cached-credential excessive pins, time boundary, text rejection before cache, and same-ID different staff isolation. Root adoption/recovery cryptography is unchanged; original assessment and committed integration regressions apply. |
| Text and poisoning of IDs/caches | Exhaustive forbidden-code-point nicknames; targeted body, organizer and unsigned controls; valid replacement packets; opaque/encrypted preservation. No native renderer execution or historical-data cleanup. |
| Wire transcript, QR/key binding, key validation and HPKE limits | No changed wire, key or primitive implementation. Three independent Node/OpenSSL reference families reproduced byte-for-byte; full original construction analysis remains applicable, including consciously accepted KCI/forward-secrecy limits. Not a second independent primitive implementation. |
| Bounds, TTL, clocks and expiry | Added text scans are within existing bounded fields; public legacy lookup returns at most two key-constrained rows. Work reservation behavior and mutable TTL contract are unchanged. Existing clock, ledger-cap, expiry and rollback policy regressions passed. No performance benchmark, concurrency stress campaign or new side-channel analysis. |
| Dependencies, features and supply chain | Manifest/lockfile and crypto source unchanged. Original pinned-tree/checksum/feature assessment applies. No newly fetched advisory database or completed errata check is claimed in this retest; dated and unavailable evidence in the original report remains qualified. |
| Native/platform/physical integration | Updated Android/Swift SQLCipher probe source inspected. Worker ran host core/FFI with a SQLite callback double only. Native CI, encryption-at-rest behavior on native providers, devices and MC-037 are separately required evidence. |

No new confirmed flaw or unresolved specification ambiguity was identified in these changed paths. The intentional residuals are preserved historical data, lack of protection against vulnerable-binary downgrade, and the unchanged integration obligations above. The retest does not erase the original report's threat-model assumptions or unverified matters.

## Commands and results

Commands ran from `C:/Users/wicke/Code/meshChat`, with `. .work/rust-env.ps1`, `RUSTC`/`RUSTDOC` resolved to that toolchain, and `CARGO_TARGET_DIR=.work/assessments/mc022/target`. Cargo/rustup/cache/temp paths remained inside the repository. Relevant versions: Rust 1.85.1 (`4eb161250`, 2025-03-15), Cargo 1.85.1 (`d73d2caf9`, 2024-12-31), Python 3.14.4, Node 24.15.0 and its OpenSSL 3.5.5.

| Command / action | Actual result and evidence |
|---|---|
| `python -B .work/assessments/mc022/retest-inputs.py` | 238 files verified against the explicit candidate; exact five-file production diff; three reference families match. `retest-inputs.log` and JSON inventory. |
| `cargo test --manifest-path .work/assessments/mc022/retest-probe-source/Cargo.toml --locked --offline --test friends --test organizer --test ingress retest_assessor_ -- --nocapture` | Seven independently added test functions passed (2 friend, 4 organizer, 1 ingress), including 156 forbidden-code-point cases. `retest-probes.log`. |
| `cargo test --manifest-path .work/assessments/mc022/retest-probe-source/Cargo.toml --locked --offline --test dm retest_assessor_ -- --nocapture` | One independently added real DM test passed. `retest-dm-probe.log`. |
| `cargo test --manifest-path .work/assessments/mc022/retest-source/tests/integration/Cargo.toml --locked --offline` | All 63 committed facade integration tests passed; zero doc tests. `retest-baseline.log`. |
| `cargo build --manifest-path .work/assessments/mc022/retest-source/Cargo.toml --locked --offline -p meshchat-core --features bindgen,security-probe` | New candidate core DLL and binding generator built successfully. `retest-storage-build.log`. |
| `uniffi-bindgen.exe generate --library .../target/debug/meshchat_core.dll --language python --out-dir .../retest-source/.work/storage-python --no-format`, then copy this DLL beside the generated binding | Generated fresh bindings for this candidate. Paths represented by `...` are under this assessment directory. |
| `python -B .work/assessments/mc022/retest-source/tests/integration/storage/test_policy.py -v` | All 13 storage policy tests passed against the freshly built core through Python UniFFI and synthetic SQLite callbacks. `retest-storage-tests.log`. This is not SQLCipher/platform certification. |

Parent-reported release, lint, full-workspace and native CI results are not substituted for the executions above. Their applicability and actual outcomes must be recorded by their owners. No result in this report claims native or physical validation that this worker did not perform.

## Gate interpretation and artifact preservation

The separately assigned automated worker independently executed the construction assessment and this retest. These artifacts provide the requested independent automated assessment evidence. They do not introduce a requirement for a human assessor absent such a normative requirement, and do not automatically satisfy unrelated native, physical, release or later-assessment gates. Closure of the three remediated issues is tied to the exact candidate above; substantive later changes require proportionate review/retest.

`retest-output-inventory.json` records hashes of this report and the retained retest scripts, logs, source inventory and probe modification artifacts. The original report hash was rechecked unchanged. No ticket was marked complete and no source fix, merge or publication was performed by this worker.
