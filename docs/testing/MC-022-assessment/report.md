# MC-022 construction and transcript assessment

Assessment date: **2026-09-16 (America/Los_Angeles)**.

Assessor: **Codex automated AI assessment worker `/root/assess_mc022`**, separately assigned from the implementation author and the routine Terra PR reviewers. This is an attributable, independently executed automated assessment of the supplied construction and source, not a human cryptographer's review, an external firm's audit, a formal proof, or certification. The worker received the author's review packet and existing tests; this was not a blind assessment. It made its own source checks and negative probes and implemented no remediation.

**Recommendation: do not freeze the full wire or close MC-022 at this revision.** One confirmed replay defect requires remediation and retest (F-01). A confirmed pin-policy conformance defect (F-02) and a reproduced clear-text validation gap with an unresolved core/native responsibility boundary (A-01) also need explicit disposition. Existing passing tests do not cover these conclusions away. Whether this type of automated assessor is acceptable for the repository's independently required construction gate is an explicit policy decision outside this report; this report does not automatically satisfy that gate or any external-assessor requirement.

## Assessed input and provenance

| Input | Revision / location |
|---|---|
| Candidate/test revision | `ad87cf890459cd43e1e10991085440ebd6d48cb9` |
| Production implementation | `835a7966f4edb2dead99f5768ec24b75059a337c` |
| Assessed snapshot | `C:/Users/wicke/Code/meshChat/.work/assessments/mc022/source` |
| Disposable probe copy | `C:/Users/wicke/Code/meshChat/.work/assessments/mc022/probe-source` |
| Original SHA-256 inventory | `input-inventory.json`, SHA-256 `3b2d68e55a88e9f8dc3aa23ad74598d3ce58e984773c6dcf16602087ab8593ac` |
| Revision/dependency verification | `provenance-results.json`, SHA-256 `bbcc2abe9460ffc4fef1740ef85e5e52d4b2abbedfeb67e877f2582a8f787606` |
| Probe modifications | `probe-modifications.json` and the three `probe-*.patch` files |

The inventory was taken before adding probes. All 233 tracked snapshot files were checked against the candidate's Git object IDs, with an important qualification: **232 supplied files have CRLF line endings where their Git blobs have LF**. After CRLF-to-LF normalization, all 233 match; their actual on-disk SHA-256 values are retained, so the snapshot is not mislabeled as raw byte-identical to the Git blobs. The candidate differs from the named production revision in no `src/`, root manifest, or root lockfile content. Verification used explicit revisions, never the unrelated working checkout's HEAD.

The original tracked snapshot files were not edited. Tests generated synthetic database artifacts inside the snapshot's ignored `.work/` folder. Only three test files in the disposable copy changed: `tests/integration/friends/friends.rs`, `tests/integration/organizer/organizer.rs`, and `tests/integration/dm/provider.rs`. All production files in that copy remain identical to the supplied source. No branch, tracked checkout file, Git state, remote comment, requirement, or ticket status was changed by this worker. Concurrent author work in the main checkout was excluded.

## Threat model and methods

The assessed attacker can control a mesh participant, originate arbitrary bounded or malformed frames, choose untrusted keys and labels, modify/replay/cache/reorder traffic, rotate claimed identifiers, and send invalid variants before valid copies. Pinned peers and adopted staff can also send malicious authenticated content. Compromise of sender or recipient keys is considered within the expressly limited HPKE guarantees. The local OS, provider callback implementation, explicit user confirmations, generation bookkeeping and SQLCipher engine are trusted boundaries; arbitrary hostile in-process native code is not excluded by the exported Rust/FFI APIs.

The assessment covered normative MC-006/MC-008 definitions, the MC-008 dependency exceptions, design corrections and relevant rendering/pin rules, MC-022's ticket and packet, actual Rust protocol owners, storage, identity, native provider/storage boundaries, and selected pinned upstream crypto implementation paths. Methods were manual transcript/state-transition analysis, resolved dependency and checksum inspection, independent negative probes through production APIs, existing real-verifier regression execution, and reproduction of the committed Node/OpenSSL reference families. Original integration helpers were reused for admitted links and synthetic SQLite; the new scenarios/assertions were assessor-authored. No private real-user material was used.

Primitive review compared the embedding to [RFC 9180](https://www.rfc-editor.org/rfc/rfc9180.html), and key-encoding assumptions to [RFC 7748](https://www.rfc-editor.org/rfc/rfc7748.html) and [RFC 8032](https://www.rfc-editor.org/rfc/rfc8032.html). This is source-level construction analysis, not independent reimplementation of each primitive or mathematical proof.

## Confirmed findings

### F-01 — Medium: a returned organizer post is accepted a second time under another direction

**Status: open; blocks the no-repeat-effects criterion and this assessment's freeze recommendation.**

Evidence in original source:

- `src/core/src/organizer.rs:846`: the local signing path persists the post with direction `1` before returning it.
- `src/core/src/organizer.rs:669`: the receiving path persists the same authenticated bytes with direction `0`.
- `src/core/src/storage.rs:292` and `:557`: direction is part of the ledger primary key and lookup for all subject domains, including organizer posts.

A staff author signs an ordinary valid post, including a valid pin if desired. A relay returns its unchanged bytes to the author's admitted ingress. The signature and credential verify, but the outgoing ledger record does not match the incoming key. `Organizer::complete` returns `AcceptResult::Accepted` again and commits a second history row and second ledger row. No forgery, compromised key, collision, clock change or malformed packet is required. A normal mesh loop or an attacker replaying the observed post is sufficient. Volatile outbound/relay suppression cannot supply durable protection after restart or eviction.

Reproduction: `probe-source/tests/integration/organizer/organizer.rs:19`, test `assessment_looped_back_own_post_is_accepted_twice`. Synthetic node 2 adopts root 9, imports staff 7, signs message 991 at time 200000, then receives its exact bytes. The probe observes **Accepted, history count 2, ledger count 2**, both on the existing owners and after dropping them, opening fresh owners and reopening the persistent database. This is a synthetic owner/store restart, not a phone restart. The probe intentionally asserts the defective behavior to make its reproduction deterministic; a green probe is not a passed replay requirement. Log: `probes.log`.

MC-008 §5 keys signed CHAT effects by full signing identity, type and message ID; its direction-qualified replay rule belongs to DMs in §4. Organizer public broadcasts have the same signing identity when they return to their author. The reproduced behavior violates the requirement that an exact authenticated replay have no second effect. Current UI integration has not been exercised, so the demonstrated impact is duplicate durable history and a second accepted-effects result, not an observed on-screen notification.

Remediation should make public signed-content replay identity independent of local incoming/outgoing presentation direction while retaining the necessary DM direction distinction. Account for existing stored records/migration and conflicting variants. Retest unchanged and conflicting returned posts, history deletion, process reopening, and pin-effect handling. No repair was made here.

### F-02 — Low: an out-of-policy pin discards otherwise valid signed organizer text

**Status: open; protocol conformance disposition required before asserting the implementation exactly implements the frozen contract.**

`src/core/src/organizer.rs:658` returns `Error::Authority` for a correctly signed post if its positive pin expiry exceeds the credential or root bound, before accepting its text. MC-006 §5 explicitly requires suppressing an out-of-policy pin even when the text signature verifies, rather than changing the valid signed text's status. The current receiver discards the entire verified effect; it neither commits the text nor returns its verified authority.

Reproduction: `probe-source/tests/integration/organizer/organizer.rs:45`, `assessment_overlong_pin_discards_valid_signed_text`. Adopted unexpired root, valid staff credential ending at 201000, valid signed post with pin expiry 201001. Result: `Error::Authority`, zero history rows. Log: `probes.log`.

An outsider cannot alter these signed pin bytes. The trigger needs an authorized/malfunctioning staff sender, so this is a low-severity availability/interoperability defect, not an authority escalation or cryptographic forgery. Preserve the authenticated text and suppress the invalid pin according to the current requirement, or obtain an explicit normative policy decision; do not silently declare the incompatible receiver behavior conformant.

## Reproduced boundary ambiguity and integration obligations

### A-01 — Medium potential impact: forbidden clear signed text receives verified-history/current-authority results

**Status: reproduced acceptance gap; exact core-versus-native enforcement responsibility needs disposition. No actual unsafe native rendering was demonstrated.**

`src/core/src/codec.rs:465` checks byte lengths and UTF-8, but deliberately does not apply `text::validate`. That separation is documented by MC-009 and is reasonable for a borrowed structural parser. However, neither `Friends::complete` (`friends.rs:481` onward) nor `Organizer::complete` (`organizer.rs:590` onward) applies the forbidden-character rule to CHAT text/nickname before returning verified results and persisting history. The same lack of validation exists along the clear ingress path. DMs do call `text::validate` in `dm.rs` after authentication, and organizer credential labels are separately validated.

Probes `assessment_forbidden_friend_text_is_committed_as_verified` (`probe-source/tests/integration/friends/friends.rs:17`) and `assessment_forbidden_organizer_text_gets_current_authority` (`probe-source/tests/integration/organizer/organizer.rs:55`) replace the nickname or first text byte with NUL and recompute a legitimate synthetic sender signature. The friend result reports new verified history and the existing pin; the organizer result is Accepted with current authority `(true, false)`. These exact strings fail `text::validate`. Logs: `probes.log`.

MC-006 §1 says text must pass design §10.2 sanitization; that section rejects C0/C1, specified bidi and zero-width characters. MC-009 says sanitization remains later owners' responsibility. The current native feature/UI owner does not yet integrate these results, so it is not established that a prohibited string will ever render. This is therefore not reported as a demonstrated badge spoof or signature bypass. Before freeze, make the mandatory rejection boundary explicit and executable: either validate original text without normalizing authenticated bytes in the owning acceptance path, or identify and test a compulsory native presentation gate and clarify which earlier effects are allowed. A mere expectation that every future consumer remembers to sanitize does not establish the invariant.

### A-02 — Work accounting at local/native boundaries remains conditional

**Status: integration obligation; no remotely exploitable over-budget crypto path was demonstrated.**

The network DM owner reserves 7/8 units, friend verification one, and organizer credential/message verification one/two; the actual selected X25519 HPKE implementation has the extra public-key derivations counted by MC-008. In contrast, `Friends::confirm` (`friends.rs:232–259`) directly calls `public_identity()` and `agree()` without an Ingress/work token, and provider load/import methods likewise are trusted local primitives. A caller can reserve local work externally, but that obligation is not mechanically enforced by these APIs. Native integration must charge key loading/validation/QR work and invalidate public friend/session evidence on provider loss, without relying on a later failed private operation. This is particularly relevant because `observation()` has no provider argument. Existing tests demonstrate clearing after a failed provider operation, not automatic native lock notifications. The construction review does not certify that unfinished native wiring meets this obligation.

## Coverage and security conclusions by area

“Reviewed/tested” below means the stated source paths and scenarios were examined; it does not mean exhaustive assurance or full application certification.

| Area | Assessment coverage and result | Limits/open matters |
|---|---|---|
| Suite and HPKE construction | Reviewed `identity/dm.rs`, `dm.rs`, pinned HPKE `setup.rs`, `kem.rs`, `kem/dhkem.rs`, `dhkex/x25519.rs`, and AEAD/key-schedule interfaces. Fixed Auth, X25519/HKDF-SHA256/ChaCha20Poly1305; one setup and one sequence-zero AEAD per message; no Base fallback or shared bidirectional context found. Published Auth KAT and reference bytes reproduce. | No proof of library correctness, timing behavior, or exhaustive upstream audit. |
| Exact transcripts/domains | Compared DM info/AAD, friend/staff/credential/root/proof domains and length prefixes with normative bytes. Exact header except TTL, flags, lengths, profile, hint, enc, key metadata, cosmetics and pin/credential fields are bound by the examined paths. Existing immutable-byte mutation suites passed. | Future schema changes need new review; mutable TTL is intentionally outside authentication. |
| QR/full identity binding | Strict tuple import, nonzero DH, pin revisions, explicit replacement/removal, ambiguous short-hint refusal, included-key binding reviewed. New probe held recipient X fixed while changing its Ed key: original opens, altered full tuple fails authentication. | QR display/source confirmation remains a human/native requirement; QR itself is not possession or distance proof. Synthetic collision tests are not generated SHA-256 collisions. |
| Tag reuse/collisions | Tag is the specified truncated HMAC of static pair DH with distinct tag-domain bytes and epoch. Only the tuple resolved by both sender hints gets its three epoch tags; no all-friend DH sweep or trial decryption. HPKE has separate labeled extraction and full role-ordered tuple info. No cross-protocol oracle yielding an HPKE secret or authentication bypass was found. | Static key reuse for tags is an additional application composition, not certified by a primitive KAT or a proof in this assessment. Four-byte tags collide and link both directions. Full sender metadata remains visible. |
| Compromise/forgery | Reviewed Auth roles and public-only/recipient-fabrication reference negatives. Existing outsider forgery cases failed; the reference intentionally reproduces recipient fabrication. | No forward secrecy, recipient-compromise resistance to impersonation, non-repudiation, post-compromise recovery or metadata privacy claimed. |
| Keys/entropy | Ed canonical-coordinate checks plus dalek strict verification and weak-key refusal reviewed. X static/enc canonical checks and all-zero DH rejection exist at appropriate boundaries. New low-order/noncanonical tests (0, 1, p−1, p, high-bit encoding) refuse static agreement. One-use entropy adapter, fallible OS failure, exact pinned RNG consumption and key invalidation tests pass. | No hardware entropy qualification. Native generation/provider is trusted; no caller-supplied generation/key relation can be considered safe against malicious in-process callers. |
| Erasure/provider lifetime | Reviewed Zeroizing seeds/plaintext, short-lived provider locks, explicit invalidation, HPKE/dalek drop behavior/features, and native finally/defer lifecycle. Receiver uses caller-owned zeroizing plaintext output. No private-export operation was added by this work. | Stack/register copies, Swift/JVM/FFI/SQL buffers and upstream temporary key copies are not proven erased. No heap forensic, crash-dump, fault-injection or side-channel experiment. |
| TTL/framing/parser | Source review of codec/framing/ingress and reference layout; structural suites passed. Canonical DM lengths/profile checked before DH; forbidden encodings reject; zero/minimal padding checked after opening. TTL-only relay rewriting leaves authenticated bytes stable; live zero TTL refuses; stored CHAT processing does not refresh TTL. | Attackers can reset mutable TTL and create traffic subject to local budgets/dedup. Radio behavior and sustained field throughput unmeasured. Clear-text acceptance ambiguity is A-01. |
| Replay/storage/clocks | Reviewed SQL BEGIN/COMMIT/rollback, first-record conflict handling, live tombstone caps, history pruning separation, generation-scoped database/reset and persisted high-water clock. Existing invalid-first/valid-second, failures, full ledger, reopening and concurrency tests pass. New expiry then ≤300-second rollback probe correctly refuses reacceptance. | F-01 breaks public organizer replay identity across direction. SQL callback/native engine durability assumed; synthetic SQLite is not SQLCipher crash/power-loss evidence. |
| Deferred jobs/bounds | Reviewed WorkPermit lifetime/deadlines, shared two-slot reservation, bounded pending/rejected/cache/orphan/recovery collections, frame/byte charging and friend/root revision rechecks. Existing eviction, expiry, recovery and flood cases pass. | No exhaustive interleaving proof, allocator peak measurement, or device CPU benchmark. A-02 covers unfinished local/native work admission. |
| Friend proof/presence | Role plus both full HELLO bodies are signed; opposite role, nonces, capacities, self-key refusal, one candidate, local-send evidence, duplicate arbitration and original-nonce freshness deadlines inspected. Existing reflection/cross-link/retry/discontinuity tests pass. ANNOUNCE does not establish fresh presence. | Wormholes remain possible. Suspend-inclusive monotonic clock and provider invalidation must be supplied by native integration. |
| Organizer adoption/credentials | Explicit full-root adoption, self-signature domain, revisions, root/staff hint binding, strict signatures, current-time validity and bounded label handling reviewed. Adopted root and credential are rechecked at completion/current authority. Staff signature does not establish ownership of header device Ed key. | Authority must be rechecked at UI/egress. Root self-signature does not certify event name or external organization identity. |
| Organizer recovery/cache/pins | Opaque offers grant no badge; invalid credentials verified before authority; ambiguous cached variants do not choose a winner. 128 credentials, 32 recovery entries, per-link limits/three attempts and absolute deadline reviewed/tested. Initially empty non-adopting relay recovery passes. Expired pins suppress correctly. | An attacker can force bounded recovery failure or ambiguity; no delivery guarantee. Out-of-policy pin handling is F-02. |
| Native integration | Read Android/Swift identity and encrypted-storage adapters: generation binding, reset journals, operation serialization, unlock checks, closure of DB/session, SQLCipher configuration. Packet/ticket native parity evidence considered as author evidence. | This worker did not rerun Kotlin/Swift, build apps, inspect device behavior, or certify unfinished shipping friend/DM/organizer FFI/UI integration. MC-025/027/043/044 and MC-037 remain separate. |

The accepted compromise limits above are consistent with RFC 9180's distinction between outsider authentication and recipient compromise, and its lack of application replay protection/forward secrecy. The selected embedding adds persistent replay and full tuple binding, but those additions require their own correct implementation. [RFC 9180 security considerations](https://www.rfc-editor.org/rfc/rfc9180.html#section-9).

## Dependency evidence

The active all-feature resolved graph was captured in `dependency-metadata.json` and `dependency-features.txt`. HPKE features are exactly `alloc,chacha,getrandom,hkdfsha2,x25519`; optional AES, NIST, PQ and SHAKE features were not active. Ed25519-dalek 2.2.0 has `alloc,std,zeroize`; both X25519-dalek 2.0.1 and 3.0.0 have `static_secrets,zeroize`; ChaCha20Poly1305 0.11.0, Poly1305 0.9.1, HMAC 0.13.0 and SHA2 0.11.0 have zeroization enabled. The nine duplicate-version pairs match the approved exception exactly, checked across all targets/features by the repository script. The facade lockfile adds only its own test package to production dependency tuples.

`audit_inputs.py` independently verified **127 cached crate archives** against the root lockfile SHA-256 values, with none missing. It also compared every packaged file in 14 critical extracted crypto crate trees byte-for-byte with those checksum-verified archives. This includes both dalek families, HPKE, SHA2 families, HMAC, HKDF, AEAD, Poly1305, zeroize and getrandom. HPKE's lockfile checksum is `a5324110b02044183df000f0bd7d2d7e61f1000631508627e77f63983b6fdffb`; published package source identifies revision `b83b0011030b55ac74f389c112db784274d4d667`. Dependency identity and feature selection are verified, not claimed as evidence that every dependency is vulnerability-free.

Cargo-deny 0.20.2 passed advisories, bans, licenses and sources using cached RustSec database `e2e640471715167f73e22eaf761f2e547adafeec` (commit date 2026-09-14T18:06:06+02:00), with unmatched license allowance warnings. **The database was not refreshed in this assessment**, so this is explicitly a check against that dated database. The author's earlier refreshed check is separate evidence. RFC 9180's main text was accessible, but both attempted RFC Editor errata endpoints returned tool errors; current errata enumeration was not completed. These limitations must not be restated as a current comprehensive vulnerability/errata clearance.

## Commands, versions and results

Environment: Windows host, Rust `1.85.1 (4eb161250 2025-03-15)`, Cargo `1.85.1 (d73d2caf9 2024-12-31)`, Python `3.14.4`, Node `24.15.0`, OpenSSL `3.5.5`, cargo-deny `0.20.2`. Repository-local Rust/Cargo homes and temp paths came from `.work/rust-env.ps1`; RUSTC/RUSTDOC were explicit executable paths; build output was `.work/assessments/mc022/target`. No native compiler/tool version is attributed to execution by this worker.

All commands below run from `C:/Users/wicke/Code/meshChat` after activating that environment. `B` abbreviates `.work/assessments/mc022` here only; substitute it with that actual path when reproducing.

| Command | Result / retained evidence |
|---|---|
| `cargo test --manifest-path B/source/tests/integration/Cargo.toml --locked --offline` | Original 57 friend/DM/organizer tests passed; `baseline-tests.log`. |
| `cargo test --manifest-path B/probe-source/tests/integration/Cargo.toml --locked --offline assessment_ -- --nocapture` | Five new scenarios executed; confirmed F-01, F-02 and A-01 behavior; clock rollback negative passed; `probes.log`. |
| `cargo test --manifest-path B/probe-source/Cargo.toml --lib --locked --offline` | Five provider tests passed, including two new key-binding/low-order probes and three original KAT/entropy tests; `provider-probes.log`. |
| `cargo test --manifest-path B/source/Cargo.toml --locked --offline --test codec --test links --test ingress` | 6 codec, 7 links and 14 ingress tests passed; `structural-tests.log`. |
| `python -B B/audit_inputs.py` | Candidate content/production comparison, all archive checksums, selected extracted trees, facade lock package comparison and all 3 Node/OpenSSL reference generators passed; `provenance.log`, `provenance-results.json`. |
| `cargo tree --manifest-path B/source/Cargo.toml --all-features --locked --offline -e features` | Captured in `dependency-features.txt`. |
| `cargo metadata --manifest-path B/source/Cargo.toml --all-features --locked --offline --format-version 1` | Captured in `dependency-metadata.json`. |
| `python -B B/source/src/core/check_dependency_pairs.py` | Exact nine approved all-target/all-feature duplicate pairs verified. An initial invocation supplied `--help`; this script ignores arguments and actually ran the verification. |
| `.work/tools/cargo-deny-0.20.2-x86_64-pc-windows-msvc/cargo-deny.exe --manifest-path B/source/Cargo.toml --all-features --frozen --config B/source/src/core/deny.toml check` | All four categories passed against the dated local DB; `deny.log`. Initial `cargo deny` was absent from PATH and subsequent obsolete flag placements failed before the correct direct invocation. These setup failures are not test passes. |

`audit_inputs.py` was used instead of the stock `check_references.py` because the latter would discover the unrelated parent checkout's HEAD when invoked in a no-.git snapshot. The first provenance attempt exposed the CRLF transformation; the final retained result records it. A second script setup assumption about `.cargo-checksum.json` was corrected to direct archive/extracted-file comparison. Neither correction modified source or relaxed the actual content/checksum checks.

The probes reproduce actual incorrect behavior and therefore use assertions for that behavior; their passing status must never be described as demonstrating that the corresponding security requirements pass. No release-mode, full simulator, whole-workspace, Kotlin/Swift or physical rerun is claimed by this assessor. The selected executed tests support the areas above; author-reported broader runs remain separately attributed evidence.

## Disposition and remaining gates

F-01 is unresolved and requires a scoped implementation change, relevant storage compatibility decision, regression evidence, and assessor retest at the remediation revision. F-02 requires correction or an explicit specification decision. A-01 requires a mandatory validated boundary and coverage or a documented, justified ownership resolution; it cannot be silently counted as secure rendering. A-02 must remain an explicit native integration obligation.

Already accepted limitations are not newly discovered flaws: HPKE KCI/recipient fabrication and no forward secrecy; short routing hints/tags and visible metadata; plaintext public channels; mutable TTL; wormhole-compatible session presence; bounded availability and recovery failures; software curves under the approved wrapping contract; and incomplete erasure of language/runtime copies. None is upgraded here into a stronger product guarantee.

Unverified matters include formal composition proof for tag-key reuse, exhaustive cryptographic library audit, side channels and memory erasure measurement, complete current errata/advisory enumeration, native end-to-end authority/render/egress wiring, hardware key/storage certification, actual Bluetooth/lifecycle behavior, power-loss durability on phones, and the later integrated independent assessment. These are explicitly outside the evidence obtained, not hidden beneath approval.

No wire-format replacement or weakened acceptance rule is authorized by this report. No ticket is marked complete. After remediation and retest, the repository owner must still determine whether the assessor role meets the intended MC-022 independence requirement and retain every separately required native, physical, integrated-assessment and release gate.
