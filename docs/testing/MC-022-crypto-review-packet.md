# MC-022 construction review packet — candidate, not frozen

This packet scopes the required construction/transcript assessment. Automated tests and ordinary Terra PR review do not substitute for that work. A separately assigned automated assessor has now supplied an [attributable report](MC-022-assessment/report.md) and reproducible findings. This is a separate AI construction assessment, not an external human audit or certification. The user approved [scoped remediation](MC-022-assessment-remediation.md); full-wire freeze remains blocked until findings and retest are resolved.

## Revision and scope

The originally assessed production implementation is main `835a7966f4edb2dead99f5768ec24b75059a337c`: MC-019 friend authentication (PR #23), MC-020 authenticated DMs (PR #24), and MC-021 organizer authority (PR #25), assessed at test candidate `ad87cf890459cd43e1e10991085440ebd6d48cb9`. The initial MC-022 change was tests/documentation only. Its user-approved follow-up corrects acceptance/storage behavior in the scoped production files; the final retest must identify the actual remediation revision.

Normative inputs are the [design and corrections](../mesh-chat-design.md), [MC-006 wire/QR grammar](../decisions/MC-006-wire-contract.md), [MC-008 construction and transcripts](../decisions/MC-008-crypto-contract.md), and [selected dependency exceptions](../decisions/MC-008-dependency-scope-proposal.md). The [MC-016 base freeze](MC-016-base-freeze.md) covers base transport only. The selected construction has been implemented; the historical evidence labels in the decision documents describe their original decision-time state.

Run `python -B tests/integration/crypto/check_references.py` to reproduce all three Node/OpenSSL reference families and write `.work/mc022/reference-inputs.json`. This records the exact checkout, uncommitted changes, Node/OpenSSL versions and SHA-256 of production Rust sources, both relevant lockfiles, normative wire/crypto decisions and reference generators/fixtures. Attach that inventory and the exact reviewed commit to an external assessment. A dirty working tree must not be described as the pristine recorded commit. The test-only lockfile may add its own package but must preserve every dependency package version/source/checksum already present in the production lockfile.

## Wire and QR inventory

All formats below have canonical, versioned definitions. Their combined security approval is pending. Do not interpret this inventory as a declaration of full-wire stability.

| Format | Authoritative definition | Implementation / test ownership |
|---|---|---|
| Whole/fragment/control frames, HELLO and directional capacity | MC-006 §§1–3 | `framing`, `ingress`, `friends`; transport/ingress/friend suites |
| Version 1 logical header, type/flag/channel/TTL matrix | MC-006 §4 | `codec`; codec golden/fuzz suites |
| Clear CHAT, ANNOUNCE, REACTION, EVENT_INFO, CRED_REQ/OFFER | MC-006 §5; MC-008 §5 for authentication | `codec`, `friends`, `organizer`; exact reference vectors and invalid-byte cases |
| SYNC_REQ, pages and completion | MC-006 §6 | `sync`, `relay`; real encrypted three-node relay/SYNC case |
| Friend/event/staff provisioning URI/QR and HTTPS fallbacks | MC-006 §7 | `links`, `text`; confirmation and strict key-binding tests |
| DM profile 01 CHAT/REACTION and recipient tags | MC-008 §§1–4 | `dm`, `identity/dm`; RFC Auth KAT, independent reference and actual ingress tests |
| Friend/staff/root/credential signatures and LINK_PROOF | MC-008 §§5–6 | `friends`, `organizer`, `identity`; exact domains, length fields and replay tests |

No new profile, packet type, QR grammar, signature domain or shipping FFI operation is introduced by MC-022. The test facade is not application feature integration.

## Evidence layers and reproduction

Use the pinned Rust 1.85.1 toolchain and repository-local cache/output configuration from the build guides. The relevant native versions are Kotlin 2.2.0/JDK 17 and the CI-pinned Xcode 16.4/Swift 6 mode. All keys and messages used by these tests are synthetic/public fixtures.

1. **Primitive/reference agreement.** `node tests/vectors/crypto/dm_vectors.cjs` first reproduces the published RFC 9180 Auth mode 02, X25519/HKDF-SHA256/ChaCha20Poly1305 sequence-zero fixture, then emits the profile packets. `organizer_vectors.cjs` first reproduces RFC 8032 TEST 1. `friend_vectors.cjs` separately constructs and signs friend/HELLO/proof transcripts with OpenSSL. [Fixture provenance](../../tests/vectors/crypto/README.md) records the upstream fixture hash, seed recipes and exact reproduction commands. These reference scripts were written for this repository; primitive agreement is not an external review of the meshChat construction.
2. **Production integration.** `cargo test --workspace --all-features --locked` and the release equivalent exercise protected providers, actual admitted cryptographic verification and persistent acceptance. `cargo test --manifest-path tests/simulator/Cargo.toml --locked` runs deterministic real-core transport cases. Relevant MC-019/020/021 tickets record the original implementation evidence and review corrections.
3. **Kotlin/Swift binding parity.** `python -B tests/integration/crypto/run_native.py kotlin` and, on a Mac, the same command with `swift`, compile the test-only `tests/integration` library, generate bindings into `.work/mc022`, and pass the committed public fixture bytes across those bindings. Eight friend, seventeen DM and eight organizer inputs are checked. Each language then corrupts one packet from each family and requires a typed error before rerunning the positive inputs. Kotlin uses a standalone test project with the existing pinned Kotlin/JNA/JUnit versions; the Android app's existing JVM test task invokes it. The existing Mac storage runner invokes the Swift check after its native SQLCipher phases. Generated code is never committed or packaged in the apps.
4. **Separate construction assessment.** The [original report](MC-022-assessment/report.md) covers the original pinned construction with explicit provenance, coverage, findings and limitations. Remediation/retest is pending. Same-core native parity, generated test bindings and ordinary Terra reviews are not substitutes.

The native facade reuses the same endpoint harnesses as the Rust tests. Foreign callers supply the reference bytes; production owners perform real HELLO/admission/signature/HPKE/storage processing. Assertions are converted to a typed fixture error at this test-only boundary. The SQLite callback process uses plaintext synthetic databases and validates SQL transaction behavior, not native encryption. Actual SQLCipher/key-lifecycle checks remain separate. The database helper includes its module namespace in temporary filenames so the combined harness cannot collide between friend/DM/organizer instances. No production code or root lockfile changes are needed.

The facade requires the exact fixture-name set in each family, not only its count. Kotlin and Swift also replace a positive DM case with an unknown name and corrupt ciphertext at the same count; this must produce a typed fixture error. The Kotlin regression reproduced the previous false success before the key-set correction, demonstrating that dropping a required positive cannot silently preserve a green parity result.

## Required assessor questions and test map

| Area | Code / executable evidence | Review questions |
|---|---|---|
| Primitive selection, domains and roles | `identity/dm.rs`, `friends/proof.rs`, `organizer.rs`; `tests/integration/dm/provider.rs` | Auth mode only? Correct suite/domain/role ordering? One fresh KEM input and one AEAD operation per message? Fallible entropy and unexpected consumption refuse output? |
| Immutable-byte binding | friend/DM/organizer integration suites | Every non-TTL header byte and exact length/profile/body/enc/key/credential/pin field bound? No normalization of signed bytes? TTL exclusion compatible with live/SYNC admission? |
| Key and identity binding | friend strict-key/proof tests; DM hint/tag/provider tests | Canonical, nonweak public keys and nonzero DH at every boundary? Full Ed/X tuple pinned explicitly? Ambiguous truncated hints refuse instead of choosing/trial-decrypting? |
| Sender forgery and compromise | Node Auth reference, DM provider/integration negatives | Public-only attacker cannot forge a pinned sender? Recipient compromise permits expected KCI/fabrication? No claim of forward secrecy or non-repudiation? |
| Parser/padding/bounds | codec/links/fuzz suites; DM reference negatives | Unknown profiles, forbidden lengths, UTF-8, nonminimal/nonzero padding and malformed tags refuse effects without unbounded allocation/work? |
| Replay and durable effects | `storage.rs`, integrated friend/DM/organizer suites | Invalid-first/valid-second same-ID works; exact authenticated replay/conflicts cause no repeated effects/trust refresh? Transaction failures, full ledger, history deletion, pruning/reopen, clocks and reset preserve invariants? |
| Concurrent and deferred work | ingress/work queues and owner tests | Held jobs recheck live identity/pin/adoption/deadline? Failed or evicted work cannot poison later valid acceptance? Real budget exhaustion and refill exercised? |
| Friend presence | exact HELLO/LINK_PROOF reference and friend tests | Both roles/nonces/capacities bound? Reflection, replay, duplicate links and the 60-second deadline handled? ANNOUNCE cannot refresh proof freshness? |
| Organizer trust | root/staff/post fixtures and organizer suite | Adoption explicit? Full root/credential/staff binding and expiry enforced? Unsafe signed labels rejected? Pin expiry bounded? Staff authority never proves the separate device sender claim? |
| Credential recovery | late-joiner, cache flood, retry/deadline tests | Initially empty non-adopting relay can later supply flooded credentials? Opaque cache grants no badge? Ambiguous variants, 128-entry cache, 32/node recovery and retry limits remain bounded? |
| Provider lifecycle and erasure | identity, protected staff session, native storage/key checks | Lock/reset/invalidation refuse operations? No application private-export API? Unavoidable language/runtime copies correctly documented? Native feature owners recheck authority at display/egress? |

The complete test names are in `tests/integration/friends/friends.rs`, `dm/dm.rs`, `dm/provider.rs` and `organizer/organizer.rs`. Their invalid-first/valid-second, replay, eviction, concurrency and budget-recovery cases run actual production verifiers. Synthetic collision buckets exercise the actual resolver; they do not claim constructed 64-bit SHA-256 collisions. Deterministic simulator success does not measure radio delivery or hardware protection.

## Known limitations and remaining gates

- HPKE Auth does not provide forward secrecy, recipient-compromise impersonation resistance, or third-party non-repudiation. Recipient tags and sender hints are routing/admission aids; they are not identities or confidentiality guarantees for metadata.
- Public/semi-private channels remain plaintext. TTL is deliberately mutable; signatures bind the other immutable header/body fields. Relaying a structurally valid object does not grant display authority.
- Library zeroization cannot prove complete erasure of every FFI/runtime/database buffer. The wrapped software-key development contract and explicit user confirmation remain mandatory.
- Native feature wiring, physical key/storage certification (MC-043/044), real radio/lifecycle acceptance (MC-025/027), integrated assessment (MC-037) and release gates remain separately owned. This test work satisfies none of their physical or external-assessor requirements.

## Assessment return and disposition

The assessor's report should name the assessor and assessment date, exact production/test revisions and input inventory, reviewed scope/methods, findings with severity and reproducible triggers, and explicit limitations. Record each finding's status, remediation PR/revision and assessor retest/acceptance. Keep unresolved findings visible; do not rename a failed criterion as a pass. If the construction or layout changes, update normative decisions and vectors within approved scope and reopen dependent integration before release.

**Current assessment status: separate automated report supplied; remediation/retest pending.** No external human assessment is claimed. MC-022 remains in review, its findings/full-wire-freeze checkboxes remain unchecked, and no merge may claim completion until the applicable gates pass. The original report's historical open findings remain intact; subsequent dispositions are separate attributable records.
