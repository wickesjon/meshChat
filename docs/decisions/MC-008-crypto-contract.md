# MC-008 — DM and trust contract

Evidence state: **specified**. The dependency experiment is host-compiled and locally checked; no application crypto implementation, native interoperability, device certification or independent construction review is claimed. This decision resolves MC-006's delegated encrypted-envelope, signature-domain and proof extension points. MC-020/021 implement them; MC-022 independently reviews and freezes the combined construction. The [approved dependency record](MC-008-dependency-scope-proposal.md) preserves the exact exception scope.

## 1. Construction and implementation boundary

Use RFC 9180 **Auth**, mode `02`, with DHKEM(X25519, HKDF-SHA256) `0020`, HKDF-SHA256 `0001` and ChaCha20Poly1305 `0003`. These identifiers are fixed by profile `01`; there is no suite negotiation, PSK, Base-mode fallback, export-only AEAD or streaming context. Use `hpke = "=0.14.1"`, defaults off, features `alloc,x25519,chacha,getrandom`. Optional PQ/NIST/AES features remain off. RFC 9180 defines the library construction; this document defines the meshChat embedding, including the complete wire input below. [RFC 9180 §§5–9](https://www.rfc-editor.org/rfc/rfc9180.html)

Each logical DM gets a fresh ephemeral KEM input and a fresh context, used for exactly one AEAD operation at sequence zero. Retransmission sends the original bytes; it does not reuse that context for a different message. There is no transmitted AEAD nonce, custom key schedule or hand-written two-DH encryption analogue. Sender setup must finish before constructing AAD because AAD includes `enc`; use `setup_sender_with_rng`, then one `seal`. Receiver uses Auth setup and one `open`. Never use the same context in both directions.

The selected library's convenience RNG API can panic on OS entropy failure. MC-020 must call the `_with_rng` API through the protected provider after a fallible OS read of exactly 32 fresh bytes. The pinned X25519 KEM requests one 32-byte fill for `DeriveKeyPair`. A one-use bounded adapter may supply only that buffer, recording consumption/error; return no encapsulation/ciphertext until the call completed with exactly that consumption and no adapter failure. Any unexpected RNG request aborts the operation and discards its outputs. Never substitute a zero/repeated seed on OS failure, unwrap an entropy error, or use a deterministic test RNG in production. Verify this API assumption on upgrade. [Pinned KEM source](https://github.com/rozbb/rust-hpke/blob/b83b0011030b55ac74f389c112db784274d4d667/src/kem.rs), [sender source](https://github.com/rozbb/rust-hpke/blob/b83b0011030b55ac74f389c112db784274d4d667/src/single_shot.rs)

MC-005 selects wrapped software curve keys, not extractable hardware keys. HPKE Auth takes a software private-key value; an opaque sign/agree handle cannot simply be passed to it. MC-017/020 must execute bounded Auth seal/open inside the provider-owned unlocked-key lifetime, returning public envelope bytes or authenticated plaintext/status. Keep the no-private-export application/UniFFI interface. The provider may instantiate HPKE key types internally from its unwrapped software material; it must not add an app-facing seed-export operation or assume a hardware curve key is exportable. Clear short-lived seeds, DH outputs, contexts and plaintext scratch on success/error/lock/reset as supported by the libraries; document unavoidable language/runtime copies. No alternate weaker wrapping policy is selected. Unavailable or locked providers refuse the operation.

The library's documented review history covers Cloudflare's internal review of 0.8, not a paid/current audit of 0.14.1 or this embedding. Selection is based on the standard, explicit Auth API, inspected source, pinned host compatibility and dependency checks. Independent review remains a gate, not a claim attached to the word HPKE.

## 2. Byte notation, identity and pin binding

All integers below are unsigned big-endian, except the X25519 public encoding defined by its primitive. `||` concatenates raw bytes, never hex text. `H` is the exact 26-byte MC-006 header with final payload length; `IH = H[0:3] || H[4:26]` is exactly 25 bytes. Only TTL at byte 3 is excluded. Preserve every reserved flag byte in authentication even when layout validation ignores high bits. Never reserialize normalized text, cosmetics or flags before checking a signature/tag.

For labels used below, `D(label) = ASCII("meshfest/" + label + "/v1") || 00`. `00` means one terminating zero byte. Only the literal labels enumerated here are valid. Length prefixes shown in transcripts are always present, with exact byte lengths; no optional separators or JSON encodings exist.

An identity generation consists of separate Ed25519 and X25519 keys. The confirmed friend QR remains exactly `01 || Ed[32] || X[32]`, 65 bytes, in MC-006's canonical URI grammar. Pin the **whole tuple**, its local generation and petname after explicit in-person confirmation; the nickname path is an untrusted display claim. The QR itself is not a signature or proof that the screen's holder controls either key. Ed possession is established by a valid message/link signature; X possession is established relative to the recipient by Auth opening. Neither operation changes the QR tuple. Ed and X keys are not converted into each other.

`sender_id = SHA256(sender_Ed)[0:8]`; the DM hint `sender_key_id = SHA256(sender_X)[0:8]`. These are lookup hints, never sufficient identity. Resolve candidates from the confirmed tuple, requiring both hints to agree. If more than one distinct pinned tuple matches both hints, do not trial-decrypt or choose by nickname/trust/arrival order: report local ambiguous identity and refuse DM acceptance for that hint pair until explicit pin management resolves it. A tag collision alone does not make a second candidate eligible. At most one HPKE open is attempted per packet.

Require 32-byte Ed/X public values. Ed verification uses pure Ed25519 (not Ed25519ph/ctx), strict canonical signature/public encodings, `S < L`, and rejects weak/small-order public keys and signature R points; use the pinned dalek strict verifier and negative vectors, not permissive/batch verification. X25519 static QR keys and transmitted `enc` must be canonical 32-byte little-endian u-coordinates (`u < 2^255-19`, high bit zero); reject noncanonical encodings before DH, and reject every all-zero DH result. These canonical wire restrictions are stricter than the primitive's permissive input decoding and are part of profile 01. On QR import, a provider agreement validates nonzero output before committing the tuple; provider/entropy failure leaves the pin uncommitted. [RFC 8032](https://www.rfc-editor.org/rfc/rfc8032.html), [RFC 7748](https://www.rfc-editor.org/rfc/rfc7748.html)

## 3. Encrypted CHAT and REACTION

Only MC-006 CHAT `01` and REACTION `06` with low flag bits `01` use this layout. Origins zero high flag bits. Version 1, length, known flag and TTL rules still apply before crypto. Encrypted packets cannot acquire a friend signature/organizer block or reinterpret a failed envelope as cleartext.

| Payload offset | Length | Value |
|---:|---:|---|
|0|1|Profile `01`|
|1|8|sender_key_id|
|9|32|HPKE encapsulated key `enc`|
|41|P+16|Ciphertext including the final 16-byte Poly1305 tag|

Let `E = payload[0:41]`. The allowed padded plaintext sizes P are **64, 144, 304**; the full payload lengths are **121, 201, 361**, and logical sizes **147, 227, 387**. Reject any other encrypted length/profile before allocation or DH. REACTION always uses P=64. CHAT plaintext is `timestamp:u32 || text_len:u16 || text[text_len] || zero_padding`, text length 1–280 bytes, valid UTF-8 under the ordinary CHAT text rules. Choose the smallest allowed P containing `6+text_len`, with all remaining bytes zero. Thresholds are 58 and 138 bytes; 139–280 use P=304. Receiver enforces minimal bucket, declared text length and every padding byte after authentication. A 280-byte final bucket could not hold 280 text bytes plus metadata; this explicitly resolves the previous underspecified 64/144/280 proposal without reducing text capacity.

REACTION plaintext is `timestamp:u32 || target_msg_id[8] || action:u8 || code:u8 || zero_padding[50]`. Action bit 0 removes; origins zero other bits and receivers ignore those bits for semantics while authenticating the raw bytes. Unknown palette codes use the existing generic rendering rule. A reaction can affect only an authenticated target in the same full-key conversation, and its actor must be the authenticated sender. A target ID ambiguous between different authenticated messages has no reaction effect; never attach it to a similarly numbered public or other-conversation message. Apply existing orphan/count limits. No acknowledgement/delivery receipt is introduced.

Let `S_Ed,S_X` be the sender's confirmed tuple and `R_Ed,R_X` the recipient's tuple, in those roles (not sorted). Exact inputs:

```text
info = D("dm-info") || 02 || 0020 || 0001 || 0003
       || S_Ed[32] || S_X[32] || R_Ed[32] || R_X[32]
aad  = D("dm-aad") || IH[25] || E[41]
```

HPKE Auth uses sender private X plus its matching public X at setup, recipient public X for seal, recipient private X and pinned sender public X for open. Both Ed tuples in `info` bind the QR association and role; the header sender_id must match S_Ed. Ed possession is not separately proved by a DM; the verified QR association and HPKE authentication give the friend attribution, subject to §7's compromise limits. Use final payload_len in IH. Profile, sender hint, enc, message type/ID, sender_id, channel tag, semantic and reserved flags, and length are all bound. TTL mutation preserves authentication but still must obey live/SYNC policy.

## 4. Recipient tag, admission and replay

Preserve the existing tag derivation exactly:

```text
pair_secret = X25519(own_X_private, pinned_peer_X_public)  # reject all-zero
epoch = floor(local_unix_seconds / 3600)
tag = HMAC-SHA256(pair_secret,
      ASCII("meshfest-dmtag-v1") || u64_be(epoch))[0:4]
```

This tag domain has **no terminating zero**. Both directions share the tag, but HPKE `info` fixes sender/recipient roles. Origins use their current epoch. A receiver permits its previous/current/next epochs, omitting negative/overflowing epochs; tags identify potential recipients, not trust. After resolving the unique tuple by both sender hints, compare only that tuple's up-to-three tags, constant-time for each comparison. Do not iterate through every friend upon tag misses or collisions. An active index may cache at most three tags per loaded friend (MC-007: 128 friends); derive lazily under work admission. A missing/unmatched/ambiguous pin cannot become verified through ciphertext supplied by the mesh.

Apply native/frame/byte/sender/crypto limits first. Refuse malformed input cheaply. A nonrecipient may relay/cache a structurally valid encrypted CHAT opaquely under ordinary limits, without claiming authentication; an intended recipient that rejects authentication cannot display or grant trust to it. Neither failed crypto nor an opaque relay-only/Bloom record may suppress later recipient authentication of different bytes with the same claimed msg_id. Short tags can collide and reveal traffic associations; they do not guarantee anonymity, recipient privacy or delivery. Old cached ciphertext outside the epoch window cannot be opened through an expanded tag search. The 48h history retention is not a promise of 48h network catch-up.

HPKE alone does not prevent replay. For successful DM authentication, require payload timestamp within `[now-172800, now+300]` seconds, using checked integer arithmetic; out-of-window ciphertext gets no new authenticated display/reaction. Commit accepted-message identity and replay tombstone **atomically before** UI/reaction effects. The key is `(local identity generation, full peer tuple, direction, logical type, msg_id)` with a hash of authenticated immutable bytes. An exact replay has no effect. A second authenticated variant under the same key is a conflict: retain the first accepted record, mark the conflict locally, and perform no second effect. Unknown/invalid packets do not create these tombstones. For reaction target lookup, a reused target ID with more than one authenticated meaning is ambiguous even if their direction/type keys differ.

MC-018 owns encrypted storage/schema caps, but must retain these tombstones until `now > timestamp+172800` even if the user prunes visible history, with bounded storage and explicit refusal of new authenticated effects when the ledger cannot commit. Do not evict live tombstones to grant fresh replay acceptance. Persist a local clock high-water mark; backwards jumps beyond 300s or an unavailable wall clock enter clock-uncertain state and refuse new time-dependent trust/DM effects until clock recovery. Do not advance this mark from peer timestamps. Restart must not reset replay protection or infer delivery. Local identity reset retires the generation and its old decryption keys; it cannot reaccept old-generation DMs as the new identity. These persistence behaviors are implementation gates, not accomplished by this decision.

## 5. Signature domains, credentials and replay

Let B be every raw logical payload byte before the final 64-byte message signature, including any length, key-inclusion, cosmetics, pin and credential fields. Header payload_len includes that final signature. Exact pure-Ed25519 signing inputs:

| Signature | Transcript |
|---|---|
|Friend CHAT or ANNOUNCE|`D("friend-sign") || IH || u16_be(len(B)) || B`|
|Organizer CHAT|`D("staff-sign") || IH || u16_be(len(B)) || B`|
|Root over staff credential|`D("credential") || u16_be(len(C)) || C`, C = every credential byte before root_signature|
|Event root self-signature|`D("event-root") || bundle[0:37]`|

Use separate domains as written; no prehash, domain omission, context variant or field normalization. Included friend keys must match both key_id and sender_id. For omitted keys, zero candidates remain bounded pending; multiple distinct full candidates remain ambiguous and unverified until an included-key packet resolves that packet's full key. Never guess a pinned person from a short-ID collision. A verified unknown key may gain an ordinary signed-content indicator, never a friend pin/petname. Anonymous Confessions messages do not use stable signatures.

Organizer verification resolves exactly one adopted full root for root_id, then an included valid credential or a unique cached credential for `(full_root, staff_key_id)`. Ambiguous roots/credentials give no authority; request/recover under MC-006/007 limits. Require credential root_id and staff hash binding, strict signatures, and `not_before <= now <= not_after <= root_expiry`. Root expiry must not precede now. Uncertain clocks grant no current staff badge or pin. No implicit expiry skew extends credential/root authority. A pin must also satisfy the existing signed pin bounds and current expiry. Staff-signed sender_id is a device claim bound by the staff key; it is not evidence of owning that Ed identity or a friend's pin.

For signed CHAT, the same 48h-past/300s-future timestamp window governs new verified display; stale content may remain historical/unverified under the design's drawer rules. Persist first accepted effects/tombstones keyed by the full signing identity, message type and msg_id, not only short sender_id. Organizer scope includes full root and staff keys. As with DMs, ledger overflow cannot silently allow replay effects. A repeated signature is historical evidence; a replay cannot refresh last-seen, pins or friend presence. Signed ANNOUNCE never establishes liveness by its timestamp. Credential/root replays do not extend their signed lifetime or create adoption; root/QR adoption still requires explicit confirmation.

## 6. Link proof and friend state transitions

Allocate transport type **03 LINK_PROOF**. It is whole-only, never logical/fragmented/SYNC/cache/flood traffic. Its body is exactly `version:01 || signer_role:u8 || signature[64]`, 66 bytes; role central=00, peripheral=01. Complete GATT value: 4-byte outer header + type + body = **71 bytes**, below admission floor146. Other unallocated transport types still reject.

The proof signer signs `D("link-proof") || signer_role:u8 || central_HELLO[54] || peripheral_HELLO[54]`, using the exact admitted HELLO bodies for this physical link and the Ed key claimed in its own HELLO. HELLO nonce generation, role/capacity/key validation and self-key refusal remain MC-006. Nonces must be fresh per admission, nonzero and distinct from other live local nonces. The receiver requires the opposite signer role and verifies against the remote HELLO's full Ed key. This binds both identities, physical roles, nonces and capacities; reflection, cross-link or changed-HELLO proof fails.

Attempt one proof per endpoint per admitted link within MC-007's ten-second post-HELLO deadline; reserve one local signature and one remote verification. At most one native retry of the same proof bytes; duplicate valid bytes need no second verification. Consume at most one distinct remote proof candidate per link, valid or invalid, without a work reset on duplicate traffic. A failure/timeout leaves only permitted unverified traffic and cannot evict a verified duplicate. A link can participate in MC-006's deterministic authenticated duplicate arbitration only after local proof transmission and successful remote verification. Disconnect, identity reset, changed HELLO/admission or provider invalidation clears this evidence.

The proof establishes a recent key response bound to a session, not distance or ongoing liveness. Freshness ends 60s after the **local HELLO nonce was generated**, or on disconnect/clock discontinuity, whichever first; suspend time counts and inability to measure it expires freshness. Subsequent signed ANNOUNCE, traffic, native retries or identical proofs do not refresh this deadline. No proof renewal opcode is introduced and no periodic reconnect is required. Long-lived links retain their authenticated session identity, but UI shows the age of the last fresh response rather than claiming the friend is currently nearby. A new naturally established link can produce a new proof. This implements the ticket's conservative freshness fallback without asserting unmeasured proximity. A real-time relay/wormhole remains possible; RSSI cannot repair that security limitation.

| Event/state | Required transition/effect |
|---|---|
|Unknown key or matching nickname/short ID|Unverified claim only; no pin, DM or association with an existing petname|
|User confirms a valid QR tuple|Pinned tuple; sending allowed when own provider/store are available; no reciprocal pin or fresh presence inferred|
|One-way QR pin|Sender can encrypt to that pin; receiver lacking sender's pin does not authenticate/display it as a friend; mutual pairing is needed for two-way DM usability|
|Signed pinned content|Verified historical content; fresh presence unchanged|
|Valid current link proof with pinned Ed|Recent authenticated session response; display last-response age, not a distance guarantee; X pin unchanged|
|Freshness expires/disconnect|Last authenticated observation only; keep confirmed pin and history; no current-presence claim|
|Different Ed or X observed over mesh|Untrusted new tuple; never silently change the old pin or assert it is the same person|
|User initiates replacement of a particular friend|Show old/new full tuple confirmation; block sends in that replacement flow; re-scan/confirm before committing a new generation; retain old history as old identity|
|Friend removed|Disable sends and new friend attribution; clear its live tag/key/proof state; do not silently merge future claims into archived history|
|Own provider locked/unavailable|No new private-key operation; no weaker provider/plaintext fallback; preserve encrypted artifacts|
|Own key loss/invalidation|Explicit recovery/reset required; no automatic regeneration; clear session trust|
|Explicit identity reset|Atomically rotate both own keys, clear friend pins and retire associated sessions/history per MC-017/018; peers require explicit re-pairing|

No network heuristic can reliably determine that a fresh key belongs to a friend who reinstalled. The old draft's automatic "Sarah's device changed" inference is withdrawn; that warning is appropriate in a user-selected replacement flow, not from nickname similarity or failed signatures. Receiving hostile claims alone cannot disable a valid existing pin or send permission.

## 7. Security limits, budgets and remaining gates

HPKE Auth authenticates against outsiders when the required private keys remain uncompromised; it is not a third-party-verifiable signature. A recipient can fabricate a transcript addressed to itself, and compromise of a recipient static key permits impersonating other senders **to that recipient** (KCI), as well as decrypting captured traffic addressed to that key. Sender compromise permits impersonating that sender. There is no forward secrecy, post-compromise recovery, replay protection supplied by HPKE, or metadata-private guarantee. A stable sender_id/hint, size bucket, time and traffic patterns remain observable; do not promise that the rotating tag hides the conversation from traffic analysis. These limits must be disclosed in DM security information. [RFC 9180 security considerations and errata](https://www.rfc-editor.org/errata_search.php?rfc=9180)

MC-007's work units count public-key scalar multiplications as DH work too, including X public-key derivation. For the pinned implementation and one unique candidate, reserve the following full upper costs before work; charge failures as attempted work. Hash/HKDF operations internal to a bounded crypto call are included in that call's unit; each separately derived epoch tag costs one unit.

| Operation on an already loaded/validated local identity | Work units |
|---|---:|
|DM send, no pair/tag cache|1 pair DH +1 current tag +2 ephemeral public derivations +2 Auth DH +1 seal = **7**|
|DM receive, no pair/tag cache|1 pair DH +3 epoch tags +1 recipient public derivation +2 Auth DH +1 open = **8**|
|Friend signature creation or verification|**1** each|
|Organizer message, uncached credential|**2** verifications; **1** when identical validated credential reusable, with current authority checks repeated|
|Link setup per endpoint|**1** local proof signature + **1** remote proof verification = **2**|

The pinned sender derives its ephemeral public key during key generation and again in encapsulation; do not count only the abstract two-DH algorithm. Loaded-key validation/generation, QR checks and cache warming must independently reserve actual work (node bucket, and arrival link when network-triggered); they cannot be hidden inside a supposedly free cache miss. The receive table already includes pair/tag cold work. Maintain at most two jobs and existing queue/memory limits. No unconditional all-friend tag sweep in the ingress path. Lock clears ephemeral/pair/tag caches; background precomputation is paced by the same node work bucket.

Maximum encrypted CHAT is387 logical bytes, fitting4 fragments at128-byte slices; encrypted REACTION is147 bytes, fitting2. Proof uses one71-byte frame each way, at most two attempts with the existing single retry. These stay within MC-007's worst-case1024-byte item, eight-unit item allowance, and initial eight-frame/eight-unit proof reservation per direction. No periodic proof traffic is added to its SYNC trace. Arithmetic is not measured crypto throughput or a passed simulator run. The [vector plan](../../tests/vectors/crypto/README.md) specifies how MC-020/022 replace sizing placeholders with executable byte-exact positive/negative cases.

MC-017/018 must demonstrate provider lifetimes, entropy failure, replay transactions, caps, clock recovery and reset with synthetic data. MC-019 proves link/friend state behavior; MC-020/021 execute library/reference interoperability, wire and hostile-input tests. MC-022 requires an independent construction/transcript assessment of the actual pinned revision, resolved dependency graph, QR binding, tag-key reuse, mutable TTL, replay, roles, provider boundary, memory handling and negative vectors, with all blocking findings remediated. Same-core Kotlin/Swift parity and Terra review are useful but cannot substitute for that assessment. MC-037 later independently assesses the integrated applications; MC-043/044 and radio/field gates remain physical evidence requirements.

Compatibility: base framing/header and clear payload bytes remain MC-006. Profile01 and transport03 fill deliberately blocked extension points; earlier clients may relay bounded opaque encrypted data but cannot decode this profile or establish its proof. Do not accept the historical AES-GCM/custom-nonce envelope, old incomplete signature transcript, or multiple interpretations as a compatibility fallback. A future construction/transcript change requires an explicit version/profile decision and new vectors; full-wire freeze is still MC-022.
