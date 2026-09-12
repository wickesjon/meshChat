# Detailed Design Document — Offline BLE Mesh Chat
**Working name: "Meshfest" (placeholder)**
Version 0.7 — Base wire, budgets and crypto contract specified; independent crypto freeze pending

---

## 0. Approved implementation review — 2026-09-11

This revision incorporates the approved review of the design and original implementation plan. The active build sequence is [docs/ticketboard/implementation-plan.md](ticketboard/implementation-plan.md); [AGENTS.md](../AGENTS.md) governs branches, squash merges and scope.

**Precedence:** this section and the corrected normative sections below supersede contradictory historical review summaries. A requirement marked unresolved is not an implementation choice delegated silently to a coder: its owning decision ticket must close before dependent implementation/freeze gates. Existing numerical estimates not backed by the new validation protocol are hypotheses, not passed acceptance gates.

### 0.1 Approved corrections

1. **One outer frame per GATT value.** A write or notification contains exactly one §2.1 frame, with no additional two-byte batching prefix. Scheduling may flush multiple separate values. Capacity is runtime and direction-specific, including notify capacity. Unsupported capacity has an explicit refusal/recovery path; MC-004 supplies documented capacity constraints; MC-006 chooses and proves a protocol admission floor with runtime refusal. No physical minimum is guaranteed by online evidence.
2. **Feasible budgets and metrics.** The normative [MC-007 budget contract](decisions/MC-007-budgets-and-acceptance.md) reconciles frame, byte, crypto, queue, forwarding and SYNC units. The prior 95-of-100-in-30-seconds gate is withdrawn. The replacement selects at most 8 items/8192 encoded bytes and requires a 120-second lossless ready-link test including framing, control and concurrent traffic. Its worksheet is arithmetic evidence, not a passed simulator/device gate. Subscription relevance cannot be assumed when no selection information is exchanged.
3. **TTL-reachable testing.** With sender TTL 7 and decrement-before-forwarding, an endpoint nine edges away in a ten-node chain is out of range for a single live flood. Test reachable paths and explicit out-of-range behavior. Count actual per-egress GATT sends, fragments and origin sends separately; replace the unsupported absolute 0.3 relays/node/message target with a measured reduction against an unsuppressed GATT-overlay baseline.
4. **Early rejection and accepted-message dedup.** Over-budget frames are dropped before reassembly/display/crypto. Unknown reserved bits do not bypass known-type checks. Maintain bounded rejected-attempt tracking separately from accepted/authenticated message identity so an invalid first copy cannot suppress a valid later copy with the same msg_id. Budget actual verification work, including embedded SYNC packets, and separate pending/unverified state from trusted acceptance.
5. **Connection bootstrap.** Advertisements contain only the service UUID. Identity-dependent duplicate-link arbitration therefore occurs after an established-link exchange, not before discovery. An asymmetric iOS-compatible connection can remain even when a nominal identity ordering would prefer an unavailable discovery direction.
6. **Secure lifecycle first.** MC-005/MC-017 define platform-operated versus wrapped software keys explicitly; do not assume Rust can extract non-exportable platform keys. MC-018 owns migrations, DM conversation identity, event trust, verification provenance, reactions and retention. Encrypted persistence and key loss/reset behavior are required before real sensitive data is stored.
7. **Honest authentication claims.** The [MC-008 crypto contract](decisions/MC-008-crypto-contract.md) selects RFC 9180 Auth with HPKE 0.14.1, exact immutable-header bindings, encodings, replay and session-proof rules. TTL alone is mutable transport metadata. Signed ANNOUNCE proves historical content, not current presence. Recipient-key compromise permits impersonation to that recipient; no forward secrecy, distance proof or metadata privacy is promised. Independent construction review remains MC-022. Do not infer identity continuity from nickname similarity.
8. **Evidence and sequencing.** MC-004 uses the explicitly approved online BLE feasibility gate below; the two-platform key-storage spike supplies a provisional development contract and automated evidence; deferred physical key/storage verification is owned by MC-043/044. Android beta depends explicitly on the full crypto gate. Phone Beacon Mode is v1; ESP32/backhaul are follow-on work. A design-stage fix is only specified until implemented, tested, and independently reviewed where required.

### 0.2 Decisions blocking implementation or freeze

| Owner | Required decision |
|---|---|
| MC-006 | Specified in the normative [canonical wire contract](decisions/MC-006-wire-contract.md): framing, capacity, discovery, reassembly, SYNC, QR, pin and cosmetic fields; crypto extension points remain owned by MC-008 |
| MC-007 | Specified in the normative [budget and acceptance contract](decisions/MC-007-budgets-and-acceptance.md): token units, memory ceilings, SYNC selected set, deadlines, deterministic scenarios and physical measurement thresholds |
| MC-008 | Specified in the [DM and trust contract](decisions/MC-008-crypto-contract.md): selected library/suite, exact encrypted envelope and transcripts, replay, session proof, key/pin transitions and independent-review requirements |
| MC-018 | Persistent versus transient data model, trust provenance, encrypted DB/key lifecycle, migration/reset and pruning |

The [MC-006 contract](decisions/MC-006-wire-contract.md) is normative for base framing, field offsets, type/flag validation and discovery. The [MC-008 contract](decisions/MC-008-crypto-contract.md) resolves its delegated encrypted-envelope, signature-domain and fresh-proof extensions. Both decisions must be resolved before MC-020, and the final combined layouts must pass MC-022 before full-wire freeze. UI implementation must use the specified organizer pin and cosmetic bytes; a signature coverage requirement alone is not a passed cryptographic review.

### 0.3 Validation evidence and external prerequisites

Use four separate evidence states: **specified**, **implemented**, **tested**, and **independently reviewed**. Historical statements such as “all resolved” mean specified at design stage only. They do not close implementation tickets.

On 2026-09-11 the user explicitly replaced MC-004 physical acceptance with primary online documentation, attributable published measurements, a conservative development matrix, a permission/RSSI decision, compiled probes and independent agent review. See [MC-004 online feasibility](decisions/MC-004-online-feasibility.md). MC-016 consumes that replacement evidence; MC-025/MC-027 and MC-038 still require physical radio/field evidence. External results are not meshChat test results.

On 2026-09-11 the user subsequently deferred MC-005 physical key/storage verification. [MC-005's development contract](decisions/MC-005-probe-plan.md) selects wrapped software curves protected by the platform wrapping policy, with synthetic automated evidence sufficient for the early spike. MC-043 owns Android physical verification before MC-034 beta acceptance; MC-044 owns iOS physical verification before MC-037 integrated assessment. These dependencies carry verification into release. MC-017/018 can implement and test with synthetic data before those gates, but real sensitive-data use and verified-device claims remain blocked for an unverified platform. No plaintext storage, silent key replacement, iOS software wrapping fallback or independent-security-review waiver is introduced.

MC-004 records documented Android/iOS behavior and permission decisions; MC-005 records protected-key and encrypted-store feasibility; MC-016 gates the base transport; MC-022 gates crypto/full-wire review; MC-036–MC-040 gate integrated security, field evidence and release. Missing devices, macOS tooling, independent review, store/domain access or publication authorization remain explicit blockers. Do not fabricate evidence or weaken security as a fallback.

Reference for the crypto decision: [RFC 9180](https://www.rfc-editor.org/rfc/rfc9180.html). Platform background assumptions must be checked against [Apple's Core Bluetooth background documentation](https://developer.apple.com/library/archive/documentation/NetworkingInternetWeb/Conceptual/CoreBluetooth_concepts/CoreBluetoothBackgroundProcessingForIOSApps/PerformingTasksWhileYourAppIsInTheBackground.html) and the actual supported device matrix.

---

## 1. System Overview

Meshfest is a serverless, text-only chat app for Android and iOS. Every phone running the app is simultaneously:

1. **A broadcaster** — advertising its presence and transmitting user messages over Bluetooth Low Energy (BLE)
2. **A relay** — rebroadcasting messages it receives so they hop across the crowd
3. **A cache** — holding recent messages to replay to newly discovered peers (store-and-forward)

There is no server, no account system, no internet dependency at any point in normal operation. The only internet-touching feature is the app-store fallback page for shared channel links.

```
 [Phone A] ~~BLE~~ [Phone B] ~~BLE~~ [Phone C] ~~BLE~~ [Phone D]
     |                                            |
     +--- A's message reaches D via B and C ------+
          even though A and D are 150m apart
```

### 1.1 Design principles

- **Relay everything, display selectively.** Devices relay all valid packets regardless of channel subscription. Channel filtering happens only at the display layer. This maximizes mesh coverage — a phone subscribed only to #General still carries private-channel traffic for others.
- **Cooperative enforcement.** With no server, rules (rate limits, TTL, packet size) are enforced by every honest node refusing to relay violating traffic. A hacked client can misbehave locally but cannot recruit the mesh to amplify it.
- **Fail quiet, degrade gracefully.** No delivery guarantees are promised. UI language: "sent to the mesh," never "delivered."
- **Protocol core is shared, radios are native.** One Rust core implements everything radio-independent; thin Swift/Kotlin drivers own the BLE stack per platform.

### 1.2 Component map

| Layer | Contents | Implementation |
|---|---|---|
| UI | Chat screens, channel picker, settings | Jetpack Compose (Android), SwiftUI (iOS) |
| App logic | Subscriptions, nickname, message history | Native, thin |
| **Protocol core** | Packet codec, dedup, TTL, relay policy, rate limiter, channel derivation, store-and-forward queue | **Rust (shared)** via UniFFI bindings |
| BLE driver | Advertise, scan, GATT server/client, connections, MTU | Kotlin (Android) / Swift (iOS) |
| OS radio | BLE controller | Platform |

---

## 2. Wire Protocol

### 2.1 Design constraints & two-layer grammar

The normative [MC-006 wire contract](decisions/MC-006-wire-contract.md) defines every base field offset, exact length, reserved-field rule and refusal case. All integers are big-endian; a GATT value is exactly one outer frame, without batching or trailing bytes.

| Offset | Size | Field |
|---|---|---|
| 0 | 1 | `frame_kind`: `0x00` whole logical, `0x01` logical fragment, `0x02` whole transport, `0x03` transport fragment |
| 1 | 1 | `frame_flags`: zero; reject nonzero values |
| 2 | 2 | `frame_len`: exactly the remaining body bytes |

Compute capacity independently per direction: `C = min(local native TX limit, peer HELLO RX limit, 512)`. Query write and notify limits separately. Both directions must admit **146 bytes**, giving `146 − 4 − 14 = 128` logical slice bytes and `8 × 128 = 1024`. A smaller or unknown capacity refuses ordinary traffic explicitly. No online or arithmetic result certifies a physical device. The prior 182-byte value remains only an example, with 164-byte logical slices.

Logical packets are 26–1,024 bytes, with at most eight logical fragments. Transport object bodies have an absolute 1,200-byte ceiling and at most sixteen fragments, with tighter known-type limits. Whole transport is `object_type:u8 || body`; fragmented transport repeats the type in its 12-byte envelope, and `total_len` excludes that type byte. SYNC_ITEM is transport type `0x01`; HELLO is `0x02`, exactly 54 body bytes and whole-only. MC-008 allocates whole-only transport type `0x03` LINK_PROOF with a 66-byte body; other transport types reject. Transport objects never become mesh traffic themselves.

The [contract's envelopes and reassembly rules](decisions/MC-006-wire-contract.md#2-outer-frames-and-reassembly) define logical 14-byte and transport 12-byte envelopes, duplicate conflicts and bounded allocation. Admission/capacity changes clear stale transfers. A version is a single integer `0x01`, never a nibble split. The [size worksheet](decisions/MC-006-wire-contract.md#8-compatibility-size-proof-and-evidence) accounts for whole/fragment overhead and the new pin, cosmetic and request-session fields.

### 2.2 Packet header (26 bytes fixed)

| Offset | Size | Field | Notes |
|---|---|---|---|
| 0 | 1 | `version` | Protocol version (single integer), currently `0x01`. Compatibility per §2.8 — not a blind drop. |
| 1 | 1 | `type` | `0x01` CHAT, `0x02` ANNOUNCE (§2.5), `0x03` SYNC_REQ (§2.6), `0x04` **reserved** (was SYNC_BATCH; SYNC responses are transport objects now, §2.1/§2.6 — finding R5.1), `0x05` EVENT_INFO (§17.2), `0x06` REACTION (§2.7), `0x07` CRED_REQ (§17.1), `0x08` CRED_OFFER (§17.1) |
| 2 | 1 | `flags` | bit0: `encrypted` (payload is a DM authenticated-encryption envelope, §7.5) · bit1: `signed` · bit2: `sig_type` (0 = friend signature block §7.4; 1 = organizer credential chain §17.1) · bits 3–7 reserved. **Fragmentation is signaled by the outer `frame_kind` (§2.1), not a flag here.** |
| 3 | 1 | `ttl` | Flood origin 7, received flood values above 7 clamp to 7, decrement before relay; live 0 rejects. Direct controls require 1 and never relay. Eligible stored CHAT at 0 is local-only (§2.6). |
| 4 | 8 | `msg_id` | Random 64-bit ID generated by sender. Dedup key. |
| 12 | 8 | `sender_id` | First 8 bytes of SHA-256(device Ed25519 public key) — self-certifying fingerprint (see §7.1). |
| 20 | 4 | `channel_id` | Channel hash (§4.2), except direct controls use 0 and encrypted tags remain MC-008. EVENT_INFO/CRED_OFFER and organizer CHAT use #Event Updates. |
| 24 | 2 | `payload_len` | Byte length of payload. |

The [type/flag matrix](decisions/MC-006-wire-contract.md#4-logical-envelope-and-type-matrix) is mandatory: masked low-three-bit values are CHAT `{0,1,2,6}`, ANNOUNCE `{0,2}`, REACTION `{0,1}`, and SYNC_REQ/EVENT_INFO/CRED_REQ/CRED_OFFER `{0}`. Reject other known combinations. High reserved bits never skip these checks. ANNOUNCE, SYNC_REQ and CRED_REQ are direct controls with TTL 1 and channel 0; they cannot be forwarded or embedded in history.

### 2.3 CHAT payload

| Field | Size | Notes |
|---|---|---|
| `timestamp` | 4 | Unix seconds, display hint; never logical ordering |
| `avatar` | 1 | Packed animal/theme; unknown animal renders generic paw |
| `nick_len` + `nickname` | 1 + N | N = 1–20 UTF-8 bytes |
| `cosmetic_flags` + `rgb` | 1 + 3 | Bit0 custom color, bit1 supporter hint; remaining bits sent zero/ignored. Absent custom color sends zero RGB and receivers ignore RGB. |
| `text_len` + `text` | 2 + T | T = 1–280 UTF-8 bytes |
| Friend tail, flags `0x02` | 73 or 105 | Friend signature block immediately after text (§7.4) |
| Organizer tail, flags `0x06` | 5 + block | `pin_state:u8 || pin_expiry:u32` then organizer signature block (§17.1) |

Unsigned CHAT has no tail. Pin state 0 requires expiry 0; state 1 requires positive expiry; other states reject. Only verified, adopted and unexpired staff/root authority can produce a displayed pin, and pin expiry cannot exceed either credential or root expiry. Pin, cosmetics, length fields and signature metadata must be authenticated as specified by MC-006/MC-008. Anonymous Confessions uses neutral avatar/zero cosmetics and no stable profile association. A supporter hint never proves entitlement or identity.

Maximum unsigned CHAT is **338 bytes**; friend-signed with included key **443**; organizer-signed with included credential and pin fields **556**. Exact offsets and signature coverage are in the [clear-payload contract](decisions/MC-006-wire-contract.md#5-clear-payloads-cosmetics-and-signatures). Encrypted CHAT uses only the MC-008-selected envelope, not this clear layout.

### 2.4 Fragmentation (transport layer, only when needed)

Fragmentation is per-link: reassemble before mesh validation, then re-fragment independently for each egress capacity. The outer kind is the stable discriminator. The [canonical envelope tables](decisions/MC-006-wire-contract.md#2-outer-frames-and-reassembly) are authoritative.

Logical groups use `(arrival link, frag_msg_id, group)`; transport groups use `(arrival link, object_type, transfer_id)`. Validate count, index, total length, type and nonempty slice bounds before allocation. Matching metadata is required. Identical duplicates have no effect; conflicting duplicates abort the group rather than overwriting bytes. The sum cannot exceed the declared total and must equal it at completion; verify the inner logical msg_id matches the envelope.

Logical limits are eight groups per link/64 globally; transport limits four/32. Overflow evicts the oldest incomplete group. Deadline is 30 seconds from the first admitted fragment, never extended by duplicates. Aborted/evicted groups retain that deadline in rejected tracking; a rejected first fragment with a complete known envelope instead anchors expiry at observation plus 30 seconds. Repeats cannot extend rejected expiry; expiry permits a new budgeted attempt. Truncated envelopes and unknown transport types allocate no group state. The contract defines scoped rejection, independent of whole packets and accepted-message dedup; an invalid claimed msg_id cannot suppress a later valid packet. MC-007 fixes aggregate bytes, rejected-attempt ceilings and overflow policy before implementation. Disconnect frees link state. Encoders prefer whole form when it fits; decoders also accept valid nonempty alternative partitions.

### 2.5 ANNOUNCE packet (type `0x02`)

Sent every 30s to each connected peer over the GATT link (never in BLE advertisements — §2.5.1). Complete payload layout (finding R5.5 — previously prose only, which made signed ANNOUNCE unimplementable):

| Field | Size | Notes |
|---|---|---|
| `timestamp` | 4B | Unix seconds, sender's clock. Present so the friend-signature transcript (§7.4) has a timestamp to bind, and to bound ANNOUNCE replay. |
| `avatar` | 1B | Packed animal + color (§10.6) |
| `peer_count` | 1B | Peers this node currently sees — a coarse density hint for §3.4c, **not** a claim about which peers are mutual |
| `status` | 1B | bits 0–1 battery tier (0 critical <15%, 1 low <40%, 2 normal, 3 charging/full); bit 2 `infra`; bits 3–7 sent zero/ignored. Supporter lives only in the cosmetic field. |
| `nick_len` | 1B | Nickname length in bytes, 1–20 |
| `nickname` | `nick_len` B | UTF-8, sanitized (§10.2) |
| `cosmetic_flags` + `rgb` | 4B | Same fixed field as CHAT; covered by any friend signature |
| `digest_len` | 2B | Length of `digest`, 0 or 256 |
| `digest` | `digest_len` B | Recent-message Bloom (below); 0-length permitted when the node has nothing recent |
| *(optional)* friend signature block | 0 or var | Present iff `flags.signed` with `sig_type=0` (§7.4); located after `digest` via `digest_len` |

**Recent-digest Bloom:** at most 200 msg_ids held from the last 60 seconds; evict oldest at capacity. `m = 2048` bits (256 bytes), `k = 6`, same double-hash/bit order as §2.6 with salt `meshfest-digest-v1`. Expire a received digest 60 seconds after reception. Advisory only: false positives can skip useful relays and neither flooding nor a fixed-filter SYNC retry guarantees repair. MC-007 defines reproducible omission cases; never treat a digest match as authentication or delivery proof.

**Signed ANNOUNCE:** includes the full signer key on every signed ANNOUNCE and follows the complete MC-006 coverage requirement, with exact domains/encoding in MC-008 §5. It authenticates historical content, not fresh direct presence without the MC-008 proof. ANNOUNCE uses TTL 1/channel 0 and is never relayed or stored for SYNC. Maximum unsigned/signed sizes are 316/421 bytes.

### 2.5.1 Advertisement vs. connection data

BLE advertisements carry **only the service UUID** for discovery. All ANNOUNCE metadata travels over an established GATT connection, because (a) advertisement space is ~31 bytes and cannot hold a 256-byte digest, and (b) iOS strips almost all advertisement payload in the background (§8.4). Discovery is "see the service UUID → connect → subscribe and exchange HELLO → admit directional capacity → exchange ANNOUNCE over GATT." This is a platform-portability requirement, not an optimization.

### 2.5.2 EVENT_INFO packet (type `0x05`)

Broadcast by beacons (and relayed normally) so arriving phones learn an event supports verified updates. **Never a basis for trust** — adoption is QR-only (§17.2). Layout:

| Field | Size | Notes |
|---|---|---|
| `event_root_id` | 8B | SHA-256(root pubkey)[:8] |
| `name_len` | 1B | 1–32 |
| `name` | `name_len` B | UTF-8 event name, sanitized (§10.2) |

Unsigned in v1 (signing it would add no trust, since the root pubkey itself must come from a QR). Rate-limited as an ordinary packet; dedup by `msg_id` prevents circulation.

### 2.6 SYNC (paginated store-and-forward)

When two nodes connect, each offers the other recent history it may be missing.

**SYNC_REQ** (direct logical packet, type `0x03`, exactly 546 bytes including header). Payload:

| Field | Size | Notes |
|---|---|---|
| `session_id` | 2B | Requester-owned, not reused within this link direction; responses echo it |
| `item_count` | 2B | How many msg_ids the requester's filter represents |
| `cursor` | 4B | **Pagination cursor (finding R5.2):** `0x00000000` starts a new walk; otherwise the `next_cursor` returned by the previous SYNC_ITEM, requesting continuation |
| `bloom` | 512B | Filter over msg_ids the requester holds (see below) |

**Exact Bloom definition:** `m = 4096` bits, `k = 6`. Bit positions by **double hashing** over SHA-256: `h = SHA-256(bloom_salt ‖ msg_id)`, `h1 = u64_be(h[0..8])`, `h2 = u64_be(h[8..16])`, bit `i` (i = 0..5) = `(h1 + i·h2) mod 4096`. `bloom_salt = "meshfest-bloom-v1"`. Big-endian; bit 0 is the MSB of byte 0.

**SYNC_ITEM — a transport object, not a logical packet (findings R5.1 / R4.5).** SYNC responses use `frame_kind = 0x02/0x03` with `obj_type = 0x01` (§2.1), so they have a real wire representation with their own fragmentation, transfer id, and size ceiling — they are never relayed and never enter the mesh pipeline. Object body:

| Field | Size | Notes |
|---|---|---|
| `session_id` | 2B | Echoes the active request in this link direction |
| `seq` | 2B | Item sequence within the session |
| `flags` | 1B | Data 0; page-end/more 3; complete 5; budget-end/more/complete 7. Other values reject; see marker rules below. |
| `next_cursor` | 4B | Nonzero only for flags 3; opaque token bound to this link/session/filter/snapshot |
| `blob_len` | 2B | Data 26–1024; terminal/page marker 0 |
| `blob` | `blob_len` B | Exact stored eligible CHAT bytes, or no bytes for a marker |

- **Ordering:** the responder walks its forward-cache **newest-first** (a late joiner wants recent context first, and a truncated walk should yield the most useful messages), skipping msg_ids the requester's Bloom claims present. The responder binds opaque cursors to the snapshot position and original request; a cursor is not a trusted raw cache offset. New inserts belong to a later walk; evicted snapshot entries may be skipped.
- **Dedup identity:** the **embedded `msg_id`** controls dedup — the requester feeds the extracted logical packet into its normal §3.1 pipeline as if received live. `session_id`/`seq` are transport-only and never touch the seen-cache.
- **TTL:** the embedded packet's TTL is used **as-stored**, then decremented once if the requester relays it onward. SYNC never refreshes TTL (no laundering of expired traffic); stored TTL 0 ⇒ display/store locally, do not relay.

**Explicit completion:** every page ends with one zero-blob marker: flags 3/nonzero cursor means continue this admitted session; flags 5/cursor 0 means complete; flags 7/cursor 0 means budget complete with more available for a later new session. Empty responses are markers at sequence 0, never empty logical packets. Sequence numbers increase across data and markers without wrapping; process all prior sequences before accepting a marker. Identical duplicates are ignored, conflicts abort, and missing sequences remain bounded pending until timeout. Only one walk per link direction is active; continuations repeat session_id, item_count and Bloom. Stale, reused or altered-filter cursors reject. The [SYNC contract](decisions/MC-006-wire-contract.md#6-sync-request-pages-and-completion) defines exact request/response bindings.

**Budgets and acceptance — MC-007.** Serve at most 8 CHAT items and 8192 encoded logical bytes per session, at most 4 items per page, newest-first across all channels. Framing and markers count additionally against frame/byte/forwarding limits. Session deadlines are 120 seconds; gap deadlines 30 seconds, neither reset by duplicates/continuations. One walk per link direction and at most 2 serving plus 2 requesting walks per node. New sessions consume link capacity 1/refill 1 per 60 s and node capacity 2/refill 1 per 30 s; continuations still pay frame/byte/work costs. Node forwarding credit includes every served SYNC fragment and marker.

The [budget contract and worksheet](decisions/MC-007-budgets-and-acceptance.md#sync-workload-and-feasibility) specify the simultaneous-direction workload: eight 1024-byte sizing envelopes, signed ANNOUNCE and reactions on a 146-byte ready link. The target is every selected non-Bloom-matching item plus ordered completion within 120 s; it is not whole-cache replication or a guarantee for a congested/lossy link. Actual signed and encrypted fixtures are required when their codec/crypto tickets implement them.

Bloom false positives are deterministic for the same set and salt. Count omissions separately. Different live traffic or cache contents may recover an omission, but no recovery guarantee follows from retrying the same filter. Only eligible held CHAT IDs enter the requester's filter, at most 5000; reject greater item_count. Subscription relevance is unknown to the responder.

### 2.7 REACTION packet

Lets users react to a message with an emoji from a fixed palette. Total packet: header + **10-byte payload** = 36 bytes — the cheapest traffic in the protocol.

| Field | Size | Notes |
|---|---|---|
| `target_msg_id` | 8B | The message being reacted to |
| `action` | 1B | bit0: `remove` (1 = retract), bits 1–7 reserved |
| `code` | 1B | Index into the versioned reaction palette |

- **Palette, not free emoji:** a fixed, versioned list (same append-only governance as the channel word lists, §4.4) — v1 ships 8: 🔥 ❤️ 😂 👍 🎉 😮 🫠 🔊. One byte on the wire, no Unicode parsing of attacker-controlled emoji, consistent rendering across platforms. Unknown codes render as a generic "+1" (forward compatibility).
- **Semantics:** one active reaction per `sender_id` per target message; a new code replaces the previous one, `remove` clears it. Receivers aggregate locally by counting distinct sender_ids per code. Counts saturate in the UI at "30+".
- **Relay economics:** reactions dedup, relay, and suppress exactly like CHAT (§3), with two extra dampers for reaction storms on popular messages: relayed REACTIONs sit **below relayed CHAT** in the queue priority (§3.3), and each node relays **at most 30 reactions per target message** — beyond that the count is saturated anyway, so further copies buy nothing.
- **Ordering:** a reaction may arrive before its target (flood ordering is best-effort). Hold orphan reactions for 2 minutes awaiting the target, then drop.
- **DMs:** a reaction to a DM is the same 10-byte payload wrapped in the DM's authenticated-encryption envelope (§7.5) — reactions to encrypted messages never leak the target relationship in plaintext.
- **Identity:** reactions always use the stable `sender_id` (§7.1). Reacting is not anonymous even in #Confessions — only *posting* there is — because per-message throwaway IDs on reactions would break the one-reaction-per-user rule.

### 2.8 Version & compatibility policy

"Drop unknown versions" would partition a crowd running mixed app versions during a rollout — the common case at a festival where people update at different times. `version` is a **single integer** (§2.1) — "unknown version" means a different integer entirely; forward-compatible evolution within version 1 is carried by unknown *type* values and reserved *flag* bits, not a version sub-field.

- **Same version, unknown type — flood with envelope-only validation.** Types 09–ff are flood-only: origins use TTL 7; receivers reject live TTL 0, clamp values above 7 to 7, then decrement before forwarding only a positive result. Validate the fixed 26-byte envelope (`version` matches; exact `payload_len` ≤ 998; frame/fragment structure well-formed) and ordinary capacity/budget limits. Do not run type-specific structural validation, signature checks or decryption; preserve opaque flags, channel and payload. Unknown packets go to TTL/dedup/relay but never history or UI and confer no authentication. Future direct-link controls need an explicit versioned decision, not this opaque-forwarding rule.
- **Rate classification for unknown types:** an unknown `type` is charged to a dedicated **conservative unknown-type bucket per link** (capacity 5, refill 1/5s), *separate from* and stricter than known-type buckets, so a future or malicious type cannot bypass rate control by being unrecognized. It also draws from the per-link aggregate ingress bucket (§6.5) like everything else.
- **Opaque payload size cap:** an unknown-type payload above 998 bytes is rejected; the entire logical packet is at most 1024. Reserved types 0x00/0x04 reject rather than using opaque forwarding.
- **Reserved flag bits** do not bypass known-type validation, authentication or rate classification. Senders MUST set them 0; receivers apply the MC-006 type/flag matrix. Unsupported combinations of known semantic flags are rejected. Opaque forwarding applies only to unknown types, within the conservative unknown-type budget.
- **Different `version` integer ⇒ do not relay, do not interpret.** A version bump is reserved for changes that cannot be additive; such packets circulate only among same-version peers. All planned v1 evolution (reaction codes, avatars, glyphs, word-list appends, new *types*) is additive and stays version `0x01`.
- **Fragments** inherit the version of their enclosed logical packet.

This keeps a mixed-version crowd on one connected mesh while bounding what an older node will blindly carry.

---

## 3. Mesh Layer

### 3.1 Ingress pipeline

Ordered so that **cheap checks and rate accounting happen before expensive work** (finding R5.3 — the previous ordering ran signature/AEAD verification at step 2 but charged the rate limiter at step 5, letting a connected attacker force Ed25519/AEAD operations at raw GATT throughput despite the advertised ingress cap):

```
FRAME LAYER (per arriving GATT value: write or notification; bounded staging)
F1. enforce bounded native staging; CHARGE per-link AND node ingress
    BYTES AND FRAMES (§6.5), including malformed/oversized attempts.
    Over budget -> drop before parsing or allocating group state.
F2. parse the 4-byte outer frame header (§2.1); malformed -> drop. Charges apply
    whatever the frame contains, including rejected syntax.
    *** All later failures, including invalid signatures and failed AEAD, have
        already consumed this budget — that is the point. ***
F3. dispatch by frame_kind:
      0x02/0x03 → transport object (SYNC_ITEM, §2.6): reassemble per-link, consume
                  locally, NEVER relay, then stop — no mesh pipeline
      0x01      → logical fragment: reassemble per §2.4 (key: link+frag_msg_id+frag_group);
                  incomplete → wait. Reassembly buffers are bounded (§2.4) and
                  allocation happens only within the already-charged budget
      0x00      → whole logical packet
F4. reserve no trust at the frame boundary. Each actual Ed25519 / AEAD operation
    in the logical pipeline, including a packet extracted from SYNC_ITEM,
    must acquire per-link and global work allowance before execution.
    MC-007 defines operation units, ceilings and pending limits. No allowance -> bounded deferral or drop;
    encrypted bytes are never displayed as plaintext/unverified message text.

LOGICAL LAYER (on a complete logical packet P)
L1. if P.version unknown → §2.8 version policy (not a blind drop)
L2. envelope validation: header field ranges, payload_len consistency,
    ttl in the context-specific range (MC-006; stored TTL 0 is local-only),
    clamp >7, declared lengths (text_len, cred_len) internally
    consistent → else drop, penalize link
L3. reject exact accepted duplicates cheaply; consult bounded rejected-attempt
    tracking separately. An invalid variant must not suppress a valid variant.
L4. reserve bounded pending state, not an accepted/authenticated msg_id entry
L5. per-sender rate limit (§6.4). Violation → drop: neither displayed nor relayed
L6. type-specific structural validation (UTF-8, text ≤ 280B, palette codes, …)
L7. CRYPTO (only now, and only within the F4 allowance):
      signed → verify signature/credential chain (§7.4/§17.1); invalid → mark
               unverified, do not badge; still relay per §17.3 content-neutrality
      encrypted → attempt AEAD only if the recipient tag matches one of our
               friends (§7.5); failure → drop silently, penalize link
L8. commit the appropriate accepted/trust state only after required validation;
    subscribed plaintext or successfully authenticated addressed DM -> UI + store
L9. cache only explicitly eligible packet types under bounded retention (§3.5)
L10. if P.ttl > 1 and relay_decision(egress_peer) (§3.4):
       P.ttl -= 1; enqueue per eligible egress peer (re-framed for that link)
```

Two invariants: exact accepted duplicates are rejected cheaply before repeated crypto, while invalid variants cannot poison accepted identity; resource admission precedes allocation and every expensive operation. Rejected/pending and trusted states remain separate (MC-013).

### 3.2 Seen-message cache

MC-007 bounds accepted exact variants at 512 entries per arrival-link partition, 4096 node-wide/384 KiB allocated, aged 15 minutes from first local acceptance. Identify the immutable content variant as well as claimed msg_id; mutable TTL does not create a new variant. MC-013 implements trust-state-aware dedup so a rejected/unverified variant cannot suppress a later valid one. Overflow evicts oldest within that source partition, not another peer's entries. A replacement link cannot evict an unrelated live partition; node caps remain mandatory.

Rejected fragment identities and other rejected variants have separate 30-second bounded tables. A two-generation, 8 KiB coarse Bloom may advise relay suppression only: positives cannot suppress display eligibility, trust verification or valid variants. This probabilistic cache is not proof of acceptance. Node/link budgets bound churn even if claimed identities rotate.

### 3.3 Transmission scheduling

Relays wait 80–400 ms Normal, 40–150 ms tier 3, 300–700 ms tier 0, 10–40 ms Beacon before becoming eligible. Own traffic skips hold-off. The MC-007 scheduler uses bounded frame-cost deficit round robin across controls, own traffic, forwarded CHAT/control/unknown, forwarded REACTION and served SYNC/ANNOUNCE, with quanta 8/16/8/4/8. SYNC and ANNOUNCE alternate within their class when both are ready. At object boundaries, each active class receives bounded service; global contention rotates links. Platform backpressure or missing budget credit prevents a latency guarantee.

Pace each link at no more than one GATT frame/second without catch-up bursts. Every frame pays aggregate egress limits and forwarded/SYNC frames also pay the mode forwarding bucket. Finish an object's fragments contiguously in index order before choosing another. MC-007 specifies 32 objects/48 KiB per link and 128/192 KiB node-wide, overflow order and 30-second waiting expiry. Preserve admitted own/in-flight work; new own admission fails explicitly when no bounded space remains. Never equate queue admission with delivery.

This replaces the unmeasured 200 ms multi-value flush proposal. Faster pacing requires a reviewed compatible budget change and physical validation. One frame per GATT value and runtime directional capacities remain unchanged.

### 3.4 Relay minimization (storm control + battery)

Naive flooding makes every node retransmit every message — in a dense crowd that is hundreds of redundant transmissions per message, and redundant TX is the single largest battery cost in the system. The mechanisms below aim to reduce per-node relay volume in dense conditions; the reduction must be measured against the MC-007 baseline while preserving delivery in sparse ones and, critically, **on bridge nodes** (finding 2). They apply per candidate egress peer, in this order, to each packet eligible for forwarding at L10 of §3.1.

**Key correction (finding 2):** Meshfest is a point-to-point GATT overlay, not a shared broadcast medium. Hearing two neighbors relay a `msg_id` does **not** prove the peers on the *other side of a bridge* received it. So suppression is **per-egress-peer and driven by explicit per-link knowledge**, never a single global cancel:

**(a) Per-egress duplicate suppression.** During the hold-off (§3.3), track, *per egress peer E*, evidence that E already has this `msg_id`: (i) E is the peer we received it from; (ii) E's recent-digest (§2.5) contains the msg_id; (iii) we directly observed E relay or send this msg_id. If any holds, skip relaying to E. Crucially, a duplicate arriving from peer X on one side of the node is evidence about X, **not** about a different egress peer Y — so a bridge node that received a packet from cluster A still relays it toward cluster B, because it has no per-egress evidence that B's peers have it. This is what protects cut vertices.

**(b) Global cancel only when every egress peer is covered.** The whole queued relay is cancelled only if *every* current egress peer independently satisfies (a) — i.e. the node has positive per-link evidence that all its neighbors already hold the message. In a dense cluster this happens readily (everyone hears everyone), reproducing the battery win; on a bridge it essentially never happens, preserving the one path.

**(c) No extra probabilistic thinning in v1.** Coverage evidence already cancels an egress under (a). When evidence is absent, forward if budgets allow, regardless of connected-peer count or another peer's advisory density. The earlier residual probability table adds no distinct safe behavior after coverage suppression. MC-007 compares actual per-egress GATT costs to an unsuppressed baseline and reports the measured reduction, including zero; it does not impose the withdrawn 0.3-relays/node/message target.

**(d) Battery-tier load shifting.** Tier 3 uses 40–150 ms hold-off and tier 0 uses 300–700 ms; the mode also selects the finite forwarding budget. Under the declared low-load gate a Saver bridge must preserve reachable delivery. At budget exhaustion even a bridge can shed forwards; no unconditional coverage or energy benefit is promised.

**First-hop rule:** fresh packets (`ttl == 7`) remain eligible for all uncovered egress peers, subject to the same admission, queue and power budgets. No TTL bypass grants unlimited work.

**What we deliberately do not do:** full relay-node election (connected-dominating-set). Needs topology consensus that churns badly at crowd scale. The per-egress stateless mechanisms above get most of the win with only pairwise digest exchange. **Required simulator case (finding 2):** two dense clusters joined by one or two GATT bridge nodes, verifying the bridge relays and delivery crosses — this scenario gates the suppression thresholds before they're fixed.

### 3.5 Store-and-forward

`forward_cache` holds eligible CHAT for 15 minutes from first local arrival, bounded by 500 records, 256 KiB encoded and 320 KiB allocated. Re-access/replay cannot refresh that age. Phone Beacon uses 60 minutes, 5000 records, 5 MiB encoded/6 MiB allocated. Enforce all caps with LRU eviction; unsigned/pending/verified states remain distinct and encrypted content never downgrades to plaintext. Internal monotonic cache sequence numbers support newest-first snapshots; SYNC exposes only opaque bound cursors. New admitted sessions offer bounded recent context under §2.6; no instant full-history promise is made.

### 3.6 What is deliberately NOT in the mesh layer

No ACKs, no routing tables, no read receipts, no message ordering guarantees. **Ordering rule (reconciling §2.3):** the sender `timestamp` is *never* trusted for logical ordering or dedup (clocks drift and are attacker-controllable). Messages display in **local arrival order**. The only use of `timestamp` is a cosmetic one: a message that arrives late relative to its neighbors and whose sender-stamped time is > 2 min behind current gets a subtle "delayed" tint and shows its stamped time — it is *not* re-sorted into the past, it stays where it arrived. So arrival order is the source of truth; the timestamp is a display hint, consistent with §2.3.

---

## 4. Channel System

### 4.1 Model

A channel is nothing but a name. There is no registry, no creation step, no owner. Two devices that derive the same `channel_id` are "in" the same channel. Devices maintain a local **subscription list**; the three public channels are pre-subscribed on first launch and cannot be deleted (only muted). Private-channel subscriptions are capped locally (free: ~5; Supporter tier: ~30, §19.1) — a purely client-side limit that never affects the wire protocol or who may join a channel.

### 4.2 Channel ID derivation

```
channel_id = first 4 bytes of SHA-256( "meshfest-v1|" + normalized_name )
```

Normalization: Unicode NFC → trim → lowercase → collapse internal whitespace. Public channels use their display name ("#general", "#confessions", "#event updates"). Private channels use the joined word triple: `"melodic|techno|valley"` (pipe-separated, order matters — the three dropdowns are positional).

4 bytes = ~4.3 billion values; accidental collisions across a festival's worth of channels are negligible, and a collision's worst case is two groups seeing each other's messages — annoying, not dangerous.

The `"meshfest-v1|"` domain prefix means a future protocol revision can re-key every channel cleanly.

### 4.3 Public channel behavior

- **#General** — default landing channel, rate-limited (§6)
- **#Confessions** — identical mechanics, plus the only channel with an anonymous composer toggle. Anonymous posts are sent under a freshly generated single-use `sender_id` unlinkable to the device's stable ID (§7), with nickname displayed as "anon" + 4 hex chars of that single-use ID. Because the ID is genuinely single-use, there is no `sender_id` continuity for a sniffer to exploit *across* anonymous posts — but individual messages are still plaintext on the air (§18-B1), which the composer copy states plainly. Rate limiting for anonymous posts falls back to the per-link cap (§6.5) since there is no persistent sender to bucket.
- **#Event Updates** — rate-limited more strictly (2 msgs/min/sender) to keep signal high. At events that adopt an organizer key (§17), posting is restricted to verified event staff; elsewhere it behaves as an ordinary channel

### 4.4 Private channels — the three-dropdown flow

Three positional dropdowns forming **Descriptor · Genre · Location**, up to 20 words per slot (all ≤ 9 chars, unambiguous spellings, no word appearing in more than one slot). Proposed lists — 20 per slot, giving **8,000 combinations**:

| Slot 1 (Descriptor) | Slot 2 (Genre) | Slot 3 (Location) |
|---|---|---|
| melodic | techno | valley |
| hard | riddim | cave |
| deep | dubstep | beach |
| dark | house | forest |
| dirty | dnb | desert |
| dreamy | trance | rooftop |
| hypnotic | hardstyle | bunker |
| euphoric | garage | lagoon |
| groovy | psytrance | pier |
| filthy | jungle | warehouse |
| minimal | breaks | canyon |
| heavy | hardcore | oasis |
| bouncy | electro | meadow |
| spacey | disco | glacier |
| gritty | ambient | volcano |
| smooth | trap | swamp |
| wonky | footwork | grotto |
| lush | gabber | summit |
| uplifting | bass | dunes |
| acid | downtempo | tundra |

The combos read like set descriptions — `melodic · techno · valley` → channel name `melodic|techno|valley` → hashed per §4.2, displayed as **"melodic-techno-valley"**; `filthy-riddim-swamp` and `euphoric-trance-summit` are equally valid citizens.

**List governance:** the slot lists are versioned constants in the protocol core (`channel.rs`). Words may be *appended* in future versions but never removed or reordered — removal would strand existing channels and break old share links. Link validation (§5.3) already handles the forward-compatibility case where an old client receives a link using newer words.

**UI note:** at 20 items, wheel-style pickers (iOS) / scrollable dropdowns with haptic detents (Android) remain comfortable; alphabetical order within each slot for findability, with a "randomize" dice button that picks a fresh combo for users creating a new channel.

**Security honesty (surfaced in UI copy):** 8,000 combinations is casual privacy — it keeps out people who weren't told the words, not anyone determined. A modified client can subscribe to all 8,000 channels trivially. The join screen says: *"Semi-private: anyone who knows (or guesses) your three words can read this channel."* The `flags.encrypted` bit is reserved so v2 can derive an AES-256-GCM key from the same three words via HKDF with zero protocol break, if you later want actual confidentiality.

Users can join multiple private channels; each is a normal subscription-list entry with the word triple stored locally.

---

## 5. Shareable Channel Links

### 5.1 Link format

```
https://meshfest.app/j/melodic-techno-valley
meshfest://j/melodic-techno-valley        (custom scheme, offline-capable)
```

The "Share channel" button produces the HTTPS form (works everywhere: SMS, WhatsApp, printed QR on a totem) and the OS share sheet.

### 5.2 Resolution paths

| Situation | What happens |
|---|---|
| App installed, link tapped | Universal Link (iOS) / App Link (Android) opens the app directly → confirmation sheet: "Join private channel melodic-techno-valley?" → subscribed. **No internet required** — the words are in the URL path itself. |
| App not installed, online | meshfest.app/j/* serves a page showing the three words in large type + store buttons. After install, the user re-taps the link or picks the words manually. (Deferred deep-linking SDKs exist but add third-party dependencies; showing the words is the zero-dependency fallback.) |
| App not installed, offline | Link is dead — nothing we can do. Mitigation: the share sheet also offers **"Share as QR"** rendering `meshfest://j/...`; a phone with the app installed can scan it fully offline via an in-app scanner. |

### 5.3 Validation

The [canonical URI/QR grammar](decisions/MC-006-wire-contract.md#7-qr-and-link-grammar) governs exact routes, bundle lengths, unpadded base64url with zero pad bits, single-pass percent decoding, text bounds and rejection of extra URI components. Friend bundles are 65 bytes, event bundles 101, and staff provisioning carries a 114–130-byte credential plus a separate 32-byte Ed25519 seed. Staff private provisioning has no HTTPS route; all trust-changing actions retain explicit confirmation.

On open, the app validates all three words against the known lists (case-insensitive). Unknown words → error state ("This link uses words from a newer version — update the app"), never a silent hash of arbitrary strings. This keeps the channel space confined to the legitimate combo space and makes links tamper-evident.

---

## 6. Rate Limiting

The [MC-007 budget contract](decisions/MC-007-budgets-and-acceptance.md) is normative. Token buckets specify burst capacity and refill, not strict sliding-window counts. KiB means 1024 bytes. All frame/byte/crypto/queue limits apply together before their associated work; no rate allowance grants trust.

### 6.1 Threat model

Spam on public channels from (a) enthusiastic humans, (b) modified clients. No server exists, so enforcement is (1) local UX friction and (2) the mesh collectively refusing to carry violations.

### 6.2 Parameters (v1)

| Scope | Capacity / refill | Accounting |
|---|---|---|
| General+Confessions CHAT |5 /1 per 12 s|Per claimed sender across both channels/all links|
| Ordinary Event Updates CHAT |2 /1 per 30 s|Per claimed sender|
| Structurally organizer-signed Event Updates |10 /1 per 6 s|Preserves mixed-adoption allowance; no badge or trust implied|
| Private CHAT |15 /1 per 4 s|Per sender across private channels|
| All CHAT |20 /1 per 3 s|Additional sender-wide bucket|
| REACTION |30 /1 per 2 s|Separate sender bucket|
| Each link ingress AND egress |15 frames /1 per second; 24 KiB /2 KiB per second|Complete GATT values including headers/fragments/control|
| Node ingress AND egress |60 frames /8 per second; 64 KiB /8 KiB per second|Independent direction buckets, shared across links/reconnects|
| Crypto |20 units /20 per second per link; 40 /40 per second node|Actual operations, including failed/embedded work|

MC-007 defines all control/unknown/session/connection buckets, finite tables, mode forwarding budgets and fairness. Do not retain the old redundant 60-relayed-packets/10 s limit or interpret per-logical-item budgets as per-frame quotas.

### 6.3 Sender-side (UX)

The composer disables at 0 tokens and shows a countdown ("You can post again in 0:09"). Tokens are computed locally from the same buckets, so honest users simply never hit the mesh-side limit.

### 6.4 Receiver-side (the real enforcement)

Every node runs the same per-sender token-bucket table **keyed by `sender_id` × channel class** for all traffic it sees. A packet exceeding its per-sender bucket is dropped at §3.1 step L5 — **neither displayed nor relayed** (this is the single authoritative rule; see the §6.5 clarification below for how it interacts with the per-link ingress bucket). Honest nodes compute the same per-sender verdict, so a spammer using *one stable identity* sees its reach collapse toward its 1-hop radius. The residual case — a spammer *rotating* identities to dodge per-sender buckets — is bounded not by per-sender verdicts but by the per-link aggregate ingress bucket (§6.5).

The sender table holds at most 4096 entries/1 MiB allocated, covering the node ingress bound of 2460 new one-frame senders over a 300-second idle window. Expire after 300 s inactivity; when full, reject a new sender rather than evict an active bucket and grant fresh burst credit. Link/node admission applies before accessing this table.

### 6.5 Known bypass and accepted residual risk

A modified client can rotate sender IDs. Admission therefore uses per-link frame and byte budgets independently of claimed identity, charged before reassembly or crypto. **An over-budget frame is dropped; it cannot subsequently be displayed.** The previous display-without-relay rule for such frames is withdrawn.

MC-007 selects the 15-frame/1-per-second link bucket, 24 KiB/2-KiB-per-second byte bucket and 20-unit/20-per-second link work bucket, with additional node ceilings and explicit reconnect handling. The 120-second 8-item/8192-byte selected-set target includes all frame overhead and concurrent traffic. Its worksheet proves arithmetic feasibility only; actual simulator and device gates remain required.

Per-sender admission remains an additional check; passing it cannot override frame rejection. New identities do not obtain a new physical-link allowance. Crypto budgets count actual verification/decryption operations, including multiple operations for credential chains and packets extracted from SYNC.

The enforceable attack bound is the **sum of the budgets of the attacker's actual admitted links**, including the declared reconnect policy. One physical attacker may open several links; there is no defensible single-link bound on that attacker's total mesh contribution. MC-013/MC-036 test both single-link rotation and multi-link abuse. Signatures prevent forgery under another valid key but do not by themselves prevent an attacker generating many keys.

---

## 7. Identity & Nicknames

### 7.1 Device identity

- **Identity keypair:** on first launch each device generates an **Ed25519 keypair** (same primitive as organizer signatures §17), private key in the OS keystore. This is the root of the friend-verification feature (§7.4).
- **`sender_id`** = **first 8 bytes of SHA-256(public key)**. Making the ID the key's fingerprint means it is *self-certifying*: a valid signature from a device proves it holds the key whose hash is that `sender_id`, so a signed message cannot be forged under someone else's ID. Unsigned traffic still carries `sender_id` and remains spoofable (accepted, §18.5) — the fingerprint only becomes a *guarantee* once a message is signed and checked against a known key.
- **Stability:** `sender_id` is **stable for the device's lifetime** (regeneratable only via "Reset identity," which rotates the keypair and clears history + friend pins). Not derived from hardware IDs.
- **Why stable, not rotating (B1 decision):** the app's core value is friends staying reachable in a crowd with no cell service; a rotating ID makes "is my friend nearby / who am I talking to" unreliable, which guts that use case. The tracking risk is **managed by disclosure** (§18-B1), not eliminated: the app never claims to be private-on-air.
- **BLE MAC**: platform address randomization stays enabled (Android 8+/iOS default). No static MAC, no identifying data in advertisements beyond the service UUID. ANNOUNCE metadata travels only over established GATT links. (The stable `sender_id` inside packets is itself a tracking vector regardless of MAC randomization — disclosed, not eliminated; §18-B1.)

### 7.2 Nicknames

- **1–20 bytes** of UTF-8 (not characters — a multibyte glyph costs more; the editor shows remaining *bytes* once near the limit and rejects input that would exceed 20 bytes), stored locally, embedded per-message, sanitized per §10.2. Editable anytime; changes apply to future messages only. (Reconciles the wire `nick_len` byte field, §2.3.)
- **Disambiguation (not authentication — see §18-M6)**: each sender's nickname carries a deterministic **color + 4-char suffix** derived from `sender_id` (e.g., "DJ_Steve ·a4f9"), purely so two people who both pick "DJ_Steve" are visually distinguishable. **The suffix's only job is collision resistance among honest users, not impersonation resistance.** At 4 hex chars (65,536 values) even a 300-strong same-nickname cluster is near-collision-free, versus ~19 for 2 chars — so 4 removes the accidental-collision failure mode a common handle like "raver" would hit at a 40k event. It is emphatically **not** proof of identity: `sender_id` is spoofable, so an impersonator can reproduce any suffix at will (§18-M6) — a longer suffix does nothing against that, and because "·a4f9" *looks* more official, the styling stays deliberately muted (small, low-contrast, clearly decorative) so it never competes with real trust markers. Real proof of identity comes only from a verified friend pin (§7.4) or an organizer signature (§17); UI copy never presents color+suffix as trusted. Supporter-tier users may override the auto-color with a custom nickname color (§19.1), but the **4-char suffix and any verified/staff badge are never overridable** — decoration cannot masquerade as identity or trust.

### 7.3 Anonymity

- **Anonymous posting is confined to #Confessions.** An anonymous post is sent under a **freshly generated single-use keypair/`sender_id`** unlinkable to the device's stable identity, discarded immediately after send. No other channel offers anonymity — offering it everywhere would gut friend-finding and rate-limiting for marginal benefit.
- **Honesty note:** the mesh is **plaintext on the air**. Anyone within radio range with a BLE sniffer can read every message in every channel, including "private" ones, and log the sending `sender_id`. Single-use IDs mean there is no cross-post continuity for a sniffer to exploit *between* anonymous posts, but each message is still plaintext. Composer copy states this: *"Anyone nearby with the right equipment can read this. Don't post anything you'd need kept secret."* Real confidentiality requires the v2 encryption path (§20).

### 7.4 Verified Friends

Closes §18-M6 (targeted impersonation) for the people who matter — your friends — using per-friend cryptographic identity established **in person, offline**. The mesh is never the root of trust; a face-to-face QR scan is.

**Adding a friend (out-of-band pairing):**
1. Your app shows a **friend-code QR** encoding your Ed25519 signing key **and X25519 encryption key** (§7.5) + current nickname: `meshfest://friend/<base64-keys>/<nick>`.
2. Your friend scans it while you are physically together. Their app **pins** (pubkey → a local **petname** they assign, e.g. "Sarah"). Optionally mutual — you scan theirs back.
3. Pinning happens off a screen in the physical world, so there is no network path to MITM the exchange — the structural strength of out-of-band verification (same principle as §17.2's organizer QR).

Pins are stored locally in the `friends` table: `friends(pubkey PK, petname, added_ts, last_seen_ts)`. A device may hold many friends; "Reset identity" and "Remove friend" clear pins.

**Signing (when identity matters):**
- A signed friend message sets `flags.signed` and appends a **friend signature block** — distinct from the organizer credential chain (§17.1) and from DM AEAD (§7.5), which are three separate authentication mechanisms:

| Field | Size | Notes |
|---|---|---|
| `signer_key_id` | 8B | SHA-256(signer Ed25519 pubkey)[:8] = the signer's `sender_id` |
| `pubkey_included` | 1B | 0 = pubkey already sent this session; 1 = full pubkey follows |
| `signer_pubkey` | 0 or 32B | Present on first signed message sent per link and every signed ANNOUNCE |
| `sig` | 64B | Ed25519 over the canonical transcript below |

- **Signature coverage:** immutable header bytes 0–2 and 4–25 (TTL excluded) plus every payload byte before the final signature, including cosmetics, lengths and key-inclusion metadata. Use the exact MC-008 §5 domain and length-prefixed transcript. Check signer_key_id and sender_id both match the resolved full public key. Omitted keys resolve only from pins/bounded cache; ambiguous or missing keys remain unverified/pending until an included-key message resolves them. There is no unspecified key-request opcode. Relays preserve original bytes, including reserved header bits.
- **Scope (airtime discipline):** v1 signs (a) ANNOUNCE beacons and (b) messages in **private channels where the sender has ≥1 pinned friend**, plus an optional "sign all my messages" setting. Public channels stay unsigned-by-default at scale — a ~73-byte friend-sig block (or +32B with pubkey) roughly doubles a short packet, and public channels don't need per-sender identity. Verification is a friend-group guarantee, not a global one.

**What the recipient sees:**

| Condition | Display |
|---|---|
| Signed, verifies against a pinned key | **✓ Petname** ("✓ Sarah") in verified style — always the *petname*, ignoring whatever nickname is broadcast |
| Signed, verifies, not a pinned friend | Plain nickname + subtle "signed" dot (authentic device, unknown person) |
| `sender_id`/nickname resembles a pinned friend but is **unsigned or fails verification** | **Warning chip: "claims to be Sarah · not verified"** — this is the impersonation catch |
| Ordinary unsigned traffic | Plain nickname + color/suffix (§7.2), no trust marker |

**Presence freshness (MC-008/MC-019):** the whole-only LINK_PROOF signs both exact HELLOs and the signer role. A valid proof authenticates a recent response bound to that session; freshness expires 60 seconds after local HELLO nonce creation or earlier on disconnect/discontinuity. Replayed proof or signed ANNOUNCE cannot refresh it. Long-lived links retain session identity but show last-response age, not current proximity. No periodic renewal/reconnect is required. Real-time relays remain possible; RSSI is advisory and cannot establish identity or distance. Exact deadlines, input bytes and state transitions are in MC-008 §6.

**Key-change handling:** pin the full Ed25519/X25519 tuple and a local generation. A network claim with a new key or matching nickname cannot identify a reinstall, replace a pin or disable the existing friend. In an explicit user-selected replacement flow, show old/new tuple confirmation and require a fresh QR scan before enabling sends to the new tuple. Retain old history as that old identity; never merge it silently. MC-008 specifies persistent replay tombstones and a 48h-past/300s-future signed-content window; a bounded relay dedup cache alone is not replay protection.

**Metadata note:** the identity pubkey/`key_id` is a stable identifier with the same tracking properties as the stable `sender_id` already accepted in §18-B1 — no new exposure. Anonymous #Confessions posts use throwaway keys and are never signed, so they carry no friend-linkable identity.

### 7.5 Direct Messages (encrypted, friends only)

DMs are end-to-end encrypted or they do not exist. The normative [MC-008 DM and trust contract](decisions/MC-008-crypto-contract.md) selects RFC 9180 Auth with DHKEM(X25519, HKDF-SHA256), HKDF-SHA256 and ChaCha20Poly1305, using pinned HPKE 0.14.1. It supersedes the historical bespoke HKDF/AES-GCM analogue. The [approved dependency exceptions](decisions/MC-008-dependency-scope-proposal.md) apply only to their listed exact version pairs at integration. MC-020 implements; MC-022 independently reviews the construction and full wire before freeze.

Encrypted CHAT/REACTION use the ordinary 26-byte header with low flag bits `01`, followed by `profile:01 || sender_key_id[8] || enc[32] || ciphertext_and_tag`. Profile01 fixes the suite, so no negotiation or transmitted AEAD nonce exists. A fresh HPKE context handles exactly one message. Its `info` binds both full Ed/X tuples in sender/recipient order; its AAD binds the exact immutable header (all bytes except TTL) and 41-byte envelope prefix. Resolve the sender against one explicitly pinned tuple matching both sender_id and sender_key_id; ambiguous/missing pins cannot acquire trust by trial or nickname. The protected provider runs HPKE internally without exporting private keys to the application.

DM CHAT plaintext is `timestamp:u32 || text_len:u16 || UTF-8 text || zero_padding`, with 1–280 text bytes, minimally padded to 64/144/304 bytes. The larger bucket preserves the full text allowance plus metadata. Logical sizes are147/227/387 bytes. DM REACTION uses timestamp plus the ordinary ten-byte target/action/code payload and zero-pads to64 bytes. It affects only an unambiguous authenticated target in the same full-key conversation. The exact profile, domains, boundary fixtures and rejection rules are in MC-008 and its [vector plan](../tests/vectors/crypto/README.md).

The keyed recipient tag remains `HMAC-SHA256(X25519(own_private, peer_public), ASCII("meshfest-dmtag-v1") || u64_be(epoch))[0:4]`, where epoch is the local Unix hour. Reject all-zero DH. Origins use the current hour; recipients compare previous/current/next hour for the uniquely resolved pin. Tags are hints, never authentication. No all-friend trial-decryption search is allowed. Relays carry valid-shaped encrypted traffic opaquely under the existing budgets; loss, mixed-channel SYNC selection, clock skew and expired tag windows may prevent catch-up.

New authenticated effects require valid timestamps and atomic encrypted-store replay tombstones, retained through their acceptance window even when visible history is pruned. MC-008 specifies conflicts, clock uncertainty, overflow refusal and reset; MC-018 implements those transactions and bounded storage. Decrypted text is stored only in the encrypted database. An unavailable/locked provider, bad tag or key change never permits plaintext or silent key replacement.

The Messages tab contains only encrypted friend conversations with petname titles. Sending requires a confirmed recipient tuple; mutual pairing is needed for both endpoints to accept DMs as friends. Security information discloses no forward secrecy/post-compromise recovery, recipient-compromise impersonation, and stable sender/time/size metadata. A recipient can fabricate transcripts addressed to itself, so a DM is not a third-party-verifiable signature. Rotating tags do not guarantee conversation anonymity. Ratchets and group DMs remain v2 work. Device verification and independent security gates are unchanged.

---

## 8. BLE Transport Layer

### 8.1 Roles and topology

Every device runs **both** GATT roles simultaneously:

- **Peripheral**: advertises the Meshfest service UUID; hosts a GATT server
- **Central**: scans for that UUID; initiates connections to discovered peers

Target: maintain **3–6 concurrent connections** per device (fewer burns coverage; more burns battery and hits platform connection caps). Peer selection is driven by **locally observable signals only** (finding R2.1): (a) prefer peers newly discovered or with weak recent traffic overlap — i.e. peers whose recent-digest (§2.5) shares few msg_ids with ours, a real signal that they bring *new* reachability rather than redundancy; (b) then strongest RSSI; (c) reserve slots for unseen peers (§18-M3). `peer_count` is a coarse advisory density hint; the §3.4c row uses locally known connected-peer count — it is **not** treated as evidence of which peers are mutual, because the protocol exchanges no adjacency data. Re-evaluate every 60s; drop the weakest link when a peer bringing more novel reachability appears.

Connection bootstrap cannot use `sender_id` before a link exists: discovery advertisements expose only the service UUID. The MC-006 HELLO state machine exchanges roles, full public keys, fresh nonces and capacities after connection. Keep a sole asymmetric link. Consolidate duplicate links only after both have the MC-008 fresh identity proof, by the common initiator-key/nonce tuple; unproved HELLO claims cannot evict a verified link. Exact fields and UUIDs are in the [discovery contract](decisions/MC-006-wire-contract.md#3-discovery-and-duplicate-links).

**Slot-exhaustion defense (see §18-M3):** reserve two of the six Normal Android slots for new peers, rotated every 60s; with a lower native/mode cap L, reserve min(2, L−1), preserving one existing slot. A peer with no valid CHAT/ANNOUNCE within 20s of connection is dropped and back-listed for five minutes. Keep the existing RSSI-diversity rule of at most three links in a near-identical RSSI cluster; this is a heuristic, not proof that several links belong to one attacker. MC-007 caps total links including handshakes, adds node-wide connection admission, and defines the observed-address bucket as capacity 3/refill 1 per 20s. Address/identity rotation cannot refill node buckets or evict a verified connection through an unproved HELLO claim.

### 8.2 GATT service definition

```
Service UUID:            c11b1d76-75da-4ae0-b0fe-cb273609c526
  TX (write without response): 78db2e71-ff31-46e0-8a8e-371f8199bcdc
  RX (notify):                 87bbc0ab-e60a-4802-b45f-445ed492cf30
  INFO (read):                 ac933506-2294-4d92-8a0c-58d9f23acfb3
```

INFO is exactly `01 02 00`: version 1 and a 512-byte protocol ceiling, not a measured MTU. HELLO (whole transport type 2) is exactly 54 body bytes: version1, role1, TX limit2, RX limit2, nonce16, Ed25519 public key32. Admit ordinary traffic only after both HELLOs and the 146-byte directional floor pass. These production UUIDs are distinct from feasibility probes.

Write-without-response + notify gives symmetric pipes matching the protocol's best-effort semantics. MTU: request 517 on connect (Android), then use the observed capacity; iOS limits are queried at runtime. No minimum iOS MTU is assumed. Fragmentation (§2.4) operates only when the value fits the approved capacity/fragment limits; otherwise refuse explicitly.

**Per-link transmit state machine (normative — finding R4).** "Write-without-response" is *not* fire-and-forget, but the two platforms expose backpressure through **different, non-interchangeable models** — imposing one on the other was the round-two bug. The queue/priority/cleanup rules are shared; the flow-control primitive is platform-specific:

*Shared:* one object send in progress per link, with fragments contiguous and increasing by index. MC-007 defines frame-cost fair queues (32 objects/48 KiB per link, 128/192 KiB globally), one-frame/second pacing and overflow order; platform readiness can slow this further. On disconnect flush queued/in-flight work with explicit local failure and free link-scoped partial reassembly. Node buckets and bounded address admission records do not refill on reconnect.

*iOS (readiness model — no per-write completion for writes-without-response):* the driver **must not** await `didWriteValueFor` (that fires only for writes *with* response). Instead it writes while `peripheral.canSendWriteWithoutResponse == true`, and when that returns false it **stops and waits for the `peripheralIsReady(toSendWriteWithoutResponse:)` delegate callback** before resuming. There is no per-frame timeout; liveness is inferred from the link staying connected and readiness callbacks continuing to fire. A link that stops delivering readiness callbacks *and* shows no inbound traffic for 15s is torn down (triggers §8.1 re-selection).

*Android (completion model):* `writeCharacteristic` completes via the async `onCharacteristicWrite` callback (API 33+ returns a status; pre-33, gate on the callback); the next frame is issued only after it, and `ERROR_GATT_WRITE_REQUEST_BUSY` means retry-after-callback. A frame with no completion callback within 5s is retried once, then the link is torn down.

*Both:* RX egress (notify) respects the platform's notify-ready signal (iOS `CBPeripheralManager` `readyToUpdateSubscribers`; Android send-queue). SYNC sessions abort cleanly on teardown; the requester retries on reconnect.

MC-004 records online API contracts and compiled probes for this model. Actual throughput/backpressure behavior remains hard physical driver-acceptance evidence at MC-025/MC-027; no early measurement is claimed.

### 8.3 Android driver

- Foreground service (`connectedDevice` type) with persistent notification supports ongoing scan/advertise; process termination and OS restrictions can interrupt it. Indefinite survival is not guaranteed ([Android background guidance](https://developer.android.com/develop/connectivity/bluetooth/ble/background)).
- Scanning: `SCAN_MODE_BALANCED` normally; `SCAN_MODE_LOW_LATENCY` for 10s bursts when peer count is 0 (fast first join), with the §11.3 backoff schedule when isolation persists
- Advertising: `ADVERTISE_MODE_BALANCED`, **service UUID only** in the advertisement (all ANNOUNCE metadata travels over GATT per §2.5.1, for cross-platform parity)
- Permissions: MC-004 selects the location-disclosed path, retaining advisory RSSI scoring/proximity and omitting `neverForLocation`. Request `BLUETOOTH_SCAN`/`ADVERTISE`/`CONNECT` on API 31+ and coarse/fine location together for location-derived proximity, with clear disclosure; denial disables proximity. API 29/30 discovery needs fine location and background discovery needs separate background-location access. This adds no GPS collection or transmitted location. Production denial/revocation behavior follows the [MC-004 decision](decisions/MC-004-online-feasibility.md).
- Known pain: per-OEM connection caps (~4–7 concurrent GATT client connections is a safe envelope) and aggressive battery managers (Xiaomi/Samsung) killing services — ship the standard "exempt this app from battery optimization" onboarding step

### 8.4 iOS driver

- Background modes: `bluetooth-central` + `bluetooth-peripheral`
- **Foreground**: full capability, parity with Android
- **Background reality (finding 4 — corrected against Apple docs):**
  - Backgrounded iOS advertising carries **only the service UUID in an overflow area discoverable *only by other iOS devices* explicitly scanning for that UUID.** Android discovery of a backgrounded iPhone is **unsupported in our baseline**. The baseline has no Live Activity; newer conditional iOS privileges are not assumed (see the MC-004 decision).
  - Therefore discovery strategy is asymmetric and explicit: **iOS-as-central** does the discovering (it can scan for the service UUID and connect out, foreground and — for known peripherals — via state restoration in background). An iPhone is discovered by others primarily while **foregrounded**; an established GATT connection may continue to handle events in background, but survival, wake timing and throughput remain conditional rather than guaranteed.
  - `CBCentralManager`/`CBPeripheralManager` state restoration re-wakes the app for connection events after suspension.
  - No ANNOUNCE metadata is ever placed in an advertisement on any platform (§2.5.1) — so nothing is "lost" to iOS stripping; metadata always flows over GATT.
  - UX accepts the residual gap: banner "Backgrounded iPhones relay less — keep the app open during peak moments," instant SYNC catch-up on foregrounding, and beacons (§15) as the structural fix for iOS-heavy crowds.
- Max concurrent connections: budget 4

### 8.5 Cross-platform interop test matrix

This is a **development support matrix**, not a physically certified device list. Supported entries mean conditional implementation targets under the [MC-004 decision](decisions/MC-004-online-feasibility.md); MC-025/MC-027 must establish actual support.

Split into **discovery cases** (which pairings can find each other from cold) and **established-connection cases** (behavior once linked), because not all discovery directions are physically possible (finding 4):

**Discovery (cold):**
| Initiator role | Peer role | Foreground | Backgrounded peer |
|---|---|---|---|
| Android central | Android peripheral | supported | supported |
| Android central | iOS peripheral | supported | **unsupported (Apple limit)** |
| iOS central | Android peripheral | supported | supported |
| iOS central | iOS peripheral | supported | supported (iOS-only overflow scan) |

**Established connection (once linked, either party may background):** all four pairings must keep the GATT link alive and continue to carry CHAT/ANNOUNCE/SYNC — verifying MTU negotiated, fragment reassembly, and SYNC completion — with one side backgrounded or screen-off. This is the case the "connect-while-foreground, then hold" strategy targets. The impossible cell above is explicitly **out of the exit gate**; it is documented as a known limitation, mitigated by beacons.

---

## 9. Local Storage

SQLCipher-encrypted SQLite is owned through the shared core with platform-protected key access. MC-005 selects the provisional development protection model; MC-017/018 implement and test with synthetic data. MC-043/044 must verify the corresponding production platform provider and storage behavior before real sensitive data is persisted on that platform.

MC-018 defines versioned migrations and persistence for message history, subscriptions/settings, both-key friend pins and petnames, stable DM conversation identity independent of rotating tags, verification provenance, adopted event roots and staff credentials, and reaction state where required. Explicitly classify ephemeral forward-cache, reassembly, orphan and credential-recovery state; not every cache needs disk persistence. The previous four-table sketch is insufficient as a complete implementation schema.

Message history retains the intended maximum of 48h or 5,000 rows per channel, whichever first, with bounded pruning during operation as well as on launch. Apply independent byte/count limits to caches and DM history. Parameterize SQL; never interpolate untrusted text.

Encrypt the DB and auxiliary persistent files; verify wrong-key failure and absence from platform cloud/device-transfer backups. MC-005/MC-018 record the exact supported Android and iOS exclusion/protection configuration. Distinguish platform-held wrapping keys from encrypted software curve keys; do not claim all curve operations are hardware-backed.

Identity reset/delete must coordinate both keypairs, pins, trust/history and encrypted storage with explicit failure/recovery behavior. Key loss is a visible recovery state, never an excuse to silently recreate identity or write plaintext. No ephemeral message key or staff provisioning secret is stored in logs or ordinary settings.

---

## 10. UI / UX Design

### 10.0 Default theme — "Afterhours" dark palette

The app defaults to a dark theme (the palette first used on the Beacon Mode screen), for three reasons: the app is used overwhelmingly at night, near-black UI meaningfully reduces OLED display power (the display dwarfs mesh radio cost, §11.2), and it doesn't blind the user or their neighbors in a dark crowd. Light mode remains available free (§19.1) and follows the same token structure.

| Token | Value | Use |
|---|---|---|
| `bg/page` | `#16161A` | App background |
| `surface` | `#1F1F25` | Cards, list rows, composer field |
| `border` | `#3A3A41` | Hairlines, input outlines |
| `text/primary` | `#F1EFE8` | Message text, titles |
| `text/secondary` | `#D3D1C7` | Previews, secondary labels |
| `text/muted` | `#B4B2A9` | Timestamps, hints, suffixes |
| `accent (teal)` | `#5DCAA5` | Mesh-active state, links, primary actions |
| `accent-strong` | `#9FE1CB` / `#E1F5EE` | Accent text/emphasis on dark |
| `warning (amber)` | `#EF9F27` on `#2A2115` | Pinned banners, unverified warnings |
| `danger (red)` | `#F09595` on `#2A1717` | Errors, destructive confirms |

Rules: all themes (including this default) must pass WCAG AA contrast for message text (§19.4); nickname auto-colors are drawn from a mid-brightness palette tuned for legibility on `#16161A`; the verified ✓ and Event Staff chips keep their own distinct backgrounds in every theme so trust chrome never blends into decoration (§18-B2). Beacon Mode's status screen (§15.7) uses this palette at reduced brightness — it is the same theme, dimmed, not a separate skin.

### 10.1 Navigation & screen inventory

**Bottom tab bar (3 tabs), split along the privacy boundary so the current tab always signals your privacy level:**

- **Channels** — public channels + semi-private word-triple channels. Plaintext-on-air, open-join. Open-lock iconography.
- **Messages** — end-to-end encrypted conversations only: 1:1 DMs (§7.5) in v1, joined by encrypted group chats when they ship in v2 (§20). E2E encrypted. Closed-lock iconography.
- **Friends** — pinned friends, friend-code QR/scanner, nearby.

Settings is a header gear on each tab. The two lock icons (open vs. closed) are visually distinct and never interchangeable — the open lock means "semi-private, readable on air"; the closed lock means "end-to-end encrypted." This is a deliberate anti-confusion rule (§18-B1/§7.3): nothing in Messages is ever plaintext, nothing in Channels is ever E2E, so the tab is a truthful privacy cue.

Screens:

1. **Onboarding (3 screens)**: pick nickname + animal avatar and color (§10.6) → permissions walkthrough (Bluetooth, notifications, Android battery exemption) → "How the mesh works" one-pager with honest expectations
2. **Channels tab (home)**: the 3 public channels + subscribed word-triple channels, unread badges + last-message preview; mesh status header (§10.4); FAB → "Join private channel"
3. **Chat screen (channel)**: message list with reaction counts under each message (long-press opens the 8-emoji palette, §2.7; tap your own reaction to remove); composer with char counter (280 bytes ≈ live count) and rate-limit countdown when throttled; header shows peer estimate. Semi-private channels show the open-lock + "anyone with the words can read this" affordance.
4. **Join private channel**: three dropdowns (large touch targets, haptic detents), live preview "melodic-techno-valley", Join + Share-link + randomize buttons; "Scan QR instead" entry point
5. **Channel info sheet**: word combo (private), share link / QR, mute, leave. Copy actions mark the clipboard entry sensitive where the platform supports it (Android 13+ `EXTRA_IS_SENSITIVE`); the sheet nudges toward share sheet / QR over raw copy (§18-m3). Its **Share page** layout, top to bottom: channel name with open-lock + "anyone with these words can read" reminder; QR on a **white card regardless of theme** (scanners need dark-on-light with a quiet zone — a functional requirement, not a style break) encoding `meshfest://j/<words>`, captioned "scanning works fully offline in the app"; the three words displayed large per slot with a "say them out loud — same channel" hint (the zero-technology sharing path); then Share-link (primary) and Copy-link actions with the honest caveat that the HTTPS link needs internet or the app installed. Sharing paths are ordered by offline reliability, matching festival conditions.
6. **Messages tab**: 1:1 DM threads (petname-titled), each with the closed-lock indicator and "Encrypted" preview styling; FAB → "New message" (picks from pinned friends — encryption requires a held key). A DM with a friend whose key changed shows a blocked state with the §7.4 re-scan prompt. The list design reserves room for group threads (group-name-titled, member-count cue) so v2 encrypted group chats (§20) slot in without a layout change.
7. **DM thread (§7.5)**: encrypted thread with lock affordance ("Encrypted for Sarah's verified device"); reactions and the same composer. (v2 group threads will add a roster sheet with rekey-on-membership-change — designed with the §20 group-chat work, not in v1.)
8. **Friends tab (§7.4)**: "My friend code" QR + scanner; list of pinned friends with petname, verified state, nearby/last-seen; per-friend re-scan prompt on key change; remove-friend. "Friends nearby" strip surfaces pinned friends in direct range. **My-friend-code page**: nickname + suffix, QR on a white card (same scannability rule as §10.1 item 5) encoding `meshfest://friend/<keys>/<nick>`, the key fingerprint in grouped hex beneath it, an in-person-verification explainer, and a "Scan a friend's code" primary action. **Add-friend confirmation sheet** (heavier than a channel join, since it creates a trust pin): scanned broadcast nickname shown as a claim ("they broadcast as…"), the scanned key's fingerprint captioned "matches the code on their screen" to invite glance-comparison against the other phone, a petname field with the "only you see this — can't be spoofed by a nickname change" note, an amber in-person warning ("a code received over the internet can be someone else's", stronger when the code arrived via deep link rather than camera, §18-m2), and a closing "show them your code so it's mutual" nudge.
9. **Settings**: nickname editor, theme picker (Afterhours dark default + light free; Supporter theme pack §19.1), power mode (Auto default / Normal / Saver, §11.4), **Beacon Mode** (§15.7, Android), manage friends (§7.4), Supporter tier (§19), reset identity, delete data, licenses

### 10.2 Message rendering rules (security-critical, see §18-B2/§18-m1)

- **No WebView, ever, for any user-supplied content.** Messages and nicknames render only in native text widgets (Compose `Text` / SwiftUI `Text`). This eliminates the entire XSS/script-injection class by construction.
- **No markdown, HTML, or rich-text interpretation** of message bodies in v1.
- **No auto-linkification** in v1. URLs display as inert text; users may long-press to copy. If linkification ships later it must be https-only allowlist, show the full URL in a confirmation sheet, and never auto-open.
- **Badges are chrome, not text.** The "Event Staff" ✓ (§17.3) is a separate UI element outside the nickname/message text field, with its own background. The nickname sanitizer strips ✓/✔/☑, "staff", and "official" — a nickname can never render into something that reads as a verified badge.
- **Unicode sanitization** on nicknames and message text: NFC normalize; reject C0/C1 control chars, bidi overrides (U+202A–U+202E, U+2066–U+2069), and zero-width chars (U+200B–U+200D, U+FEFF); cap combining marks at 2 per base char (anti-zalgo); clamp rendered nickname to a single line with ellipsis. Nicknames additionally get a confusable-skeleton check (Unicode TR39) against the local user's own nickname to flag homoglyph impersonation.

### 10.3 Composer details

- Send button disabled at 0 rate-limit tokens with countdown ring
- Byte-aware counter (UTF-8 — emoji cost 4 bytes; show bytes remaining, not chars, once under 40)
- On send: message renders immediately with a single hollow check = "handed to mesh." That is the only state; there is deliberately no "delivered"

### 10.4 Mesh status indicator (persistent header strip)

| State | Display |
|---|---|
| 0 peers | Gray dot — "Searching for nearby people…" |
| 1–3 peers | Yellow — "Small mesh · 2 nearby" |
| 4+ peers | Green — "Mesh active · ~12 nearby" |
| BT off / no permission | Red — tappable, deep-links to fix |

Peer estimate = direct connections + unique sender_ids heard in last 90s (gives a crowd-scale feel beyond 1 hop).

### 10.5 Empty/edge states

- New channel, no traffic: "Quiet so far. Messages appear when people within mesh range post."
- Delayed-message tint (§3.6) with tooltip "Arrived late via the mesh"
- Airplane-mode education: BLE works in airplane mode if BT is re-enabled — onboarding explicitly teaches this, it's the #1 battery trick at festivals

### 10.6 Icon system — animal avatars & channel glyphs

Replaces the generic `#` with personality, at a cost of exactly one wire byte.

**User avatars (animal face × color).** Each user picks an animal face *and* a color theme in onboarding/settings; both pack into the single avatar byte in CHAT and ANNOUNCE (§2.3/§2.5):

- **Low nibble — animal (16 slots, append-only):** `0` = mask (reserved for anonymous posts), `1` cat, `2` dog, `3` lizard, `4` raccoon, `5` turtle, `6` fish, `7` bird, `8` whale; `9–15` reserved for future animals. Unknown indices from newer clients render as a generic paw.
- **High nibble — color theme (16, versioned):** a curated palette drawn from the app's color ramps (teal, coral, purple, pink, blue, green, amber, plus lighter/deeper variants), every entry tuned to pass contrast on the Afterhours `#16161A` background and on the light theme. The color tints the avatar's circle background and line art — 8 animals × 16 colors = **128 distinct looks** for one wire byte.

Icon color is free for everyone (it's part of avatar identity); the Supporter perk remains nickname *text* color (§19.1), a separate lane. Rendered from a **bundled vector icon set**, never platform emoji (consistent cross-platform look, no Unicode parsing of attacker-controlled glyphs — same reasoning as the reaction palette §2.7). Anonymous #Confessions posts always render the fixed **mask icon in neutral gray** regardless of the sender's avatar byte — anonymity must not leak a distinctive animal *or* color choice, so the client sends `0x00` on anonymous posts and receivers force-render the neutral mask for any anonymous-channel display.

**Trust rules (same lane as nickname color, §7.2/§18-B2):** the avatar is decoration — freely spoofable in unsigned traffic, never a trust signal, and never overridable *onto* trust chrome (the ✓ verified and Event Staff chips keep their own geometry in every theme). One genuine upgrade: for **verified friends**, the avatar arrives in their *signed* ANNOUNCE (§7.4), so a pinned friend's animal face is authenticated — the Friends tab and DM threads render the avatar from the signed source, meaning Sarah's axolotl really is Sarah's axolotl there, while in public channels any avatar is just a claim.

**Channel glyphs.** Channels drop the `#` for a glyph. The three public channels get fixed assignments: **#General → wave**, **#Event Updates → lightning bolt**, **#Confessions → fire**. These three glyphs are **reserved exclusively** for their public channels — they are excluded from the private-channel pool, so no private channel can ever visually resemble an official one (fire means Confessions, everywhere, always). Private word-triple channels get a glyph **derived deterministically from `channel_id`** (hash mod pool length), so every device shows the same glyph for the same channel with zero coordination and zero wire bytes. v1 private pool (9, append-only): moon, sun, star, mushroom, crystal, spiral, comet, cactus, disco ball. Lock badges (§10.1 open/closed) overlay the glyph corner and remain the privacy signal — the glyph is flavor, the lock is meaning.

---

## 11. Power Management

### 11.1 Measuring power

Radio scans, held connections, advertising, packet work and screen use must be measured on named devices. The earlier component estimates and cross-app drain comparisons were unverified hypotheses; they are not release evidence or in-product promises. GATT frame counts exclude BLE link-layer overhead and cannot be converted into energy without physical instrumentation.

### 11.2 Predeclared physical acceptance

MC-007 specifies paired 4-hour screen-off runs, three repetitions per device/mode, idle-app control, fixed temperature/start charge and randomized order. Report raw and incremental battery percentage-point drain/hour, energy where available, traffic and delivery. Targets are Normal≤ 5 percentage points/hour, Saver≤ 2 and held-background iOS≤ 2, alongside actual delivery/lifecycle gates. A suspended phone cannot pass by consuming no energy while doing no required work. Beacon power and 30% auto-downgrade are measured separately. Screen-on results must be separated from screen-off acceptance. MC-025/027/038 own actual measurements; none are claimed here.

### 11.3 The lone-wanderer trap

A phone with 0 peers runs low-latency burst scanning to find someone (§8.3) — so a user drifting alone at the venue edge drains *faster* than one in a crowd. Fix (add to both drivers): after 5 minutes without discovering any peer, back off to 10s low-latency bursts every 30s; after 15 minutes, every 60s; reset to aggressive on any discovery or on app foregrounding. Worst-case isolated drain then converges toward saver-mode levels instead of exceeding normal mode.

### 11.4 Power modes and hard limits

**Default mode is "Auto"** — a policy, not a static setting. It picks between Normal and Saver from two signals the phone already tracks (battery tier and visible peer count), downgrading only phones whose absence the mesh can afford:

| Battery | Peers visible | Mode chosen |
|---|---|---|
| > 40% | any | Normal |
| 20–40% | ≥ 4 (redundancy exists nearby) | Saver |
| 20–40% | ≤ 3 (this phone may be a bridge) | Normal |
| < 20% | any | Saver |
| Charging | any | Normal (or Beacon Mode if auto-beacon §15.7 is enabled) |

Rationale: uniform Saver would slow peer discovery ~6× (deadly in high-churn crowds — topology tears faster than it repairs), flatten the battery-tier heterogeneity that §3.4d's load-shifting depends on, and effectively raise §14's venue-wide adoption threshold — all to save battery mostly where Normal already costs only 2–3%/hr. Auto keeps sparse-area bridges at full duty while letting redundant phones in dense clusters coast. Mode transitions are hysteresis-damped (no flapping at a threshold: switch only after the condition holds 60s). Manual override to forced-Normal or forced-Saver remains in settings.

- **Forwarding budget:** Normal capacity 120 frames/refill 2 per second; Saver 40/refill 2 per 3 seconds. Includes every per-egress relay and served-SYNC fragment/marker. Own/control traffic still pays aggregate egress limits. Forwarding exhaustion may shed relays while otherwise valid local acceptance proceeds; ingress rejection always blocks display. Counts bound work, not measured battery drain.
- **TX pacing** (§3.3): at most one separately framed GATT value/second/link, plus native backpressure and all node/link limits; no catch-up burst.
- **Normal**: continuous balanced scan, 3–6 connections, ANNOUNCE every 30s
- **Saver**: scan duty-cycled 10s on / 50s off, max 3 connections, ANNOUNCE every 60s, battery tier broadcast forces relay de-prioritization per §3.4d
- Screen-off ≠ background on Android (foreground service keeps full function); iOS screen-off follows background rules (§8.4)

---

## 12. Codebase Structure

The repository layout is governed by [AGENTS.md](../AGENTS.md):

```text
src/
  core/               shared Rust protocol/application core
  android/            native BLE/security and Compose UI
  ios/                native BLE/security and SwiftUI
  organizer-tools/    offline root/credential issuance (MC-041)
  share-site/         minimal HTTPS installation fallback (MC-031)
tests/
  simulator/          deterministic point-to-point GATT model
  adversarial/        controlled abuse/flood harnesses
  vectors/            golden wire and crypto fixtures
  integration/        feature/native acceptance
  fuzz/               hostile-input targets and corpora
  bench/              physical radio/device checks
  field/              mixed-platform scale checks
docs/
  ticketboard/        active implementation plan and tickets
  decisions/          approved explicit protocol/platform choices
  testing/            measurement evidence
```

Native projects retain conventional internal source layouts. Root manifests, lockfiles and CI paths remain conventional. Generated bindings/build outputs and temporary work stay ignored inside the repository.

The core exposes a sans-IO interface: drivers provide link lifecycle, inbound bytes, directional capacities, monotonic time and power events; the core returns bounded send commands and UI events. Native key-provider interfaces make protected-key access explicit. MC-003 establishes this interface before mesh implementation and proves Kotlin and Swift bindings.

### 12.1 Hardened-core build requirements (see §18)

These are enforced by build config and CI, not left to discipline:

- **Memory & panic safety (§18-M4):** `#![forbid(unsafe_code)]` in the core crate; all packet-length arithmetic uses `checked_*`/`saturating_*` (never raw `+`/`-` on lengths or offsets); the parser returns `Result` and never indexes a slice without a bounds check; `overflow-checks = true` in the release profile; every UniFFI entry point wraps the call in `catch_unwind` and converts a panic into an error return so a malformed packet can never unwind across the FFI boundary (which is UB) or crash-loop the app.
- **Fuzzing as a CI gate (§18-M4/m2):** `cargo-fuzz` targets for the packet parser and the deep-link/URL handler run in CI with a persisted corpus; a new panic or OOM fails the build.
- **Supply chain (§18-m5):** `cargo-audit` and `cargo-deny` run in CI; dependencies are version-pinned; the dependency budget is deliberately tiny (target: `sha2`, `ed25519-dalek`, `x25519-dalek`, an AEAD crate (`aes-gcm` or libsodium via `sodiumoxide`), `hkdf`, `rusqlite`/SQLCipher, `rand` — plus their unavoidable transitive deps; crypto primitives are always library-provided, never hand-rolled), and any addition needs manual review.
- **No content in logs (§18-m7):** message text and channel words are never logged at any level; `sender_id` is redacted to 2 chars in any diagnostic output; all logging is compiled out of release builds. A CI grep gate fails the build if banned log macros reference message/text fields.
- **Native-side safety:** the Android/iOS drivers treat every inbound byte array as untrusted and hand it straight to the core without pre-parsing; confirmation UIs set `setFilterTouchesWhenObscured(true)` / the iOS equivalent to defeat tapjacking (§18-m4).

---

## 13. Test & Validation Plan

Tests are implemented with their owning feature, not deferred to hardening. Ticket exit criteria and MC-007's predeclared workload/threshold decision are authoritative.

1. **Core and wire (MC-009–MC-016):** exact vectors for all frame/type forms; logical and transport fragmentation; unknown-type compatibility; malformed/reserved flag cases; channel and QR normalization; parser/deep-link fuzzing with retained regression inputs.
2. **GATT-overlay simulation (MC-012–MC-016):** deterministic seeds; point-to-point egress counts; runtime capacities; loss, churn, clock skew and topology. Compare dense suppression against an unsuppressed baseline. Sparse chain tests distinguish TTL-reachable recipients from out-of-range endpoints. Include cut vertices and measured battery-tier work shares. The previous absolute 0.3-relays/node/message and ten-node end-to-end gates are withdrawn.
3. **SYNC (MC-007/MC-015):** a defined byte-bounded eligible set and completion target fitting all frame/byte/crypto/relay budgets; maximum-size signed/encrypted records; mixed-channel traffic; empty sessions, pagination under cache mutation, deterministic Bloom omissions and disconnect/cursor expiry. The previous unconditional 95-of-100-in-30s gate is withdrawn.
4. **Physical radios (MC-025/MC-027; procedure retained by MC-004):** separate discovery and held-connection matrices, both transmission directions and capacities, realistic background/suspension/force-quit states, backpressure and reconnects; record actual device/OS versions and battery methodology.
5. **Ingress and resource abuse (MC-013/MC-036):** fragment/session/credential/orphan/cache exhaustion, sender rotation, multiple admitted attacker links, invalid signatures/AEAD, reserved-bit bypass attempts, and invalid-first/valid-second dedup poisoning. Measure per-link and node-wide memory/work bounds. No rejected frame reaches the display path.
6. **Identity/storage/UI (MC-017–MC-019/MC-028–MC-035):** protected key lifecycle, wrong-key DB failure, backup exclusion, retention/migration/reset; friend re-pairing and presence replay; native text rendering and independent trust chrome; UI never downgrades a DM to plaintext.
7. **Crypto/full-wire (MC-008/MC-020–MC-022):** selected-library reference vectors, sender-forgery and immutable-header tampering, malformed/all-zero DH rejection, epoch/collision behavior, encrypted reactions and replay, root/staff/message/pin binding, multiple staff credentials and multi-hop recovery. Reference implementations and independent construction review must be distinguished from same-core Kotlin/Swift binding parity.
8. **Release evidence (MC-037–MC-040):** independent integrated assessment with remediated blockers; predeclared 30–50-device mixed-platform field study and beacon comparison; both platform release readiness and actual authorized store outcomes. Missing evidence blocks the gate.

Each report identifies commands/build IDs, seeds or device matrix, workload, duration, thresholds, observed results and limitations. “Specified” is not “tested”; no hardware measurement or independent review is implied by this design document.

---

## 14. Adoption & Coverage Model

How many users does the mesh need? The controlling variable is **density, not headcount** — the mesh percolates when the average node has ~5 neighbors in radio range; below that it fragments into islands, above it a giant connected component forms sharply.

### 14.1 Assumptions

| Parameter | Value | Basis |
|---|---|---|
| Effective BLE range in crowds | ~15m (700m² disc) | 2.4GHz absorption by human bodies; open-field range is far higher but irrelevant |
| Neighbors needed for percolation | ~5 mean | Random geometric network threshold, with margin |
| Required active-user density | 1 per ~140m² | 5 neighbors ÷ 700m² |
| Install → active discount | ×0.5 | iOS backgrounded, battery saver, phones off |
| **Required install density** | **1 per ~70m²** | |

### 14.2 Two regimes

**Dense clusters (stage crowds, 1–2 people/m²):** 1 install per 70m² is ~**1% of people present**. The app feels instant and rich here at trivially low adoption — this is the cold-start regime the product must be satisfying in.

**Venue-wide (40k event, 15–30 hectares):** the binding constraint is sparse connective tissue (walkways, food areas, camp edges at ~1 person per 20–50m²). Blanketing the grounds at 1 install per 70m² requires **~2,000–4,000 installs (5–10% of attendance)**.

| Adoption at 40k | Experience |
|---|---|
| ~400 (1%) | Great inside any crowd cluster; islands between areas; cross-venue delivery in minutes via walkers |
| ~2,000 (5%) | Venue-wide mesh mostly connected at peak; near-real-time across grounds |
| ~4,000 (10%) | Robust through quiet hours, sparse corridors, and iOS background penalty, with margin |

### 14.3 Softening factors

- **Data mules:** store-and-forward (§3.5) means anyone walking between areas physically carries the last 15 minutes of messages and re-seeds them on arrival — sub-threshold adoption degrades to minutes of latency, not failure.
- **Locality of use:** #Event Updates near a stage and friend-group private channels are naturally local; they work in the 1% regime.

### 14.4 Product/launch implications

1. Design and market for the 1% regime being genuinely useful (crowd-local chat); treat venue-wide coverage as an emergent bonus, not a promise.
2. **Seed density where it pays:** QR codes on screens/totems ("no signal? chat here") spike adoption inside dense areas — far more effective than chasing a % of ticket buyers pre-event.
3. **Organizer partnership is the strongest lever:** official posts in #Event Updates give the low-adoption regime a killer use case and a reason to install on the spot — and partnered venues can deploy fixed relay beacons (§15), which removes the 5–10% venue-wide adoption requirement entirely.
4. Instrument (debug builds) hop-count and delivery telemetry at field tests to calibrate this model against reality before publishing any coverage claims.

---

## 15. Festival Partnership: Fixed Relay Beacons

A partnered festival can deploy dedicated, mains/solar-powered relay nodes across the grounds. Beacons are protocol-normal mesh participants — phones need no special code path to benefit — but their placement and always-on nature transform the coverage math: **venue-wide reach stops depending on adoption percentage.**

### 15.1 Hardware

| Option | Unit cost | Notes |
|---|---|---|
| **ESP32-S3 module in weatherproof box (recommended)** | $15–30 built | BLE 5, runs a stripped C/Rust port of the mesh core, powered by USB brick, PoE splitter, or 10W solar + 18650 pack. Draws <1W. |
| Raspberry Pi Zero 2 W | $30–50 | Runs the actual Rust core unmodified (fastest path to a pilot); higher power draw |
| Retired Android phones on chargers | ~$0 | Install the app, enable **Beacon Mode** (§15.7), tape to a pole. Perfect for the first pilot event. |

The pilot answer is "old Android phones," the production answer is ESP32. Both speak the identical wire protocol.

### 15.2 What makes a beacon different (all within existing protocol)

- **Placement:** mounted 3–5m up (light poles, vendor stalls, delay towers), above the bodies that soak up 2.4GHz. Effective radius jumps from ~15m to **50–150m**.
- **`infra` flag** (§2.5 status bit 2) + permanent battery tier 3. Mechanism under the corrected per-egress rule (§3.4, finding R9): the beacon's fast rebroadcast (10–40ms hold-off) enters nearby phones' recent-digests quickly, so those phones gain *per-egress coverage evidence* for the peers the beacon also reaches and can suppress relays **to those specific peers** — but only where the beacon demonstrably covers them, not globally. A beacon's fast relay proves the *beacon* has the message, not that a phone's other peers do, so phones still relay toward any peer lacking coverage evidence. **Expected effect on phone battery is therefore a measured hypothesis, not a design guarantee:** beacons should reduce redundant phone transmissions in their radius, but the magnitude is validated by per-phone TX measurement with vs. without a beacon (M6/M7 field test), not asserted as a marketing number.
- **No additional mode forwarding bucket:** phone Beacon remains bounded by every aggregate ingress/egress, crypto, session, queue and memory limit in MC-007. It has no protocol privilege and never has an unbounded actual rate.
- **Big store-and-forward:** phone Beacon extends retention to 60 minutes/5,000 messages with MC-007's encoded/allocated byte caps. Each arriving phone still receives only the budgeted newest-first mixed-channel SYNC selection; no full-hour or subscription-specific replay is promised.
- **Connection capacity:** 8+ concurrent GATT links (ESP32-S3 handles this), with connection slots reserved preferentially for phones advertising 0–1 peers (rescuing isolated users first).

### 15.3 Optional backbone (the real superpower)

Beacons within radio range of each other mesh normally. For distant zones (main stage ↔ camping), pairs of beacons can bridge over a **side-channel backhaul**: Ethernet/venue WiFi where it exists, or **LoRa** point-to-point where nothing exists (kilometer range, no infrastructure, ~kbps — fine for 300-byte text packets at human typing rates).

Encapsulation rule: a packet crossing the backhaul is decremented **exactly 1 TTL hop** at the ingress beacon regardless of physical distance, and carries its original `msg_id` (dedup cache still prevents loops if backhaul topology has cycles). To phones, a cross-venue message simply looks like it took one hop. Backhaul link details never leak into the wire protocol — it's a private tunnel between two beacon processes.

With backbone: TTL 7 comfortably spans any venue (phone → 2 hops → beacon → backbone → beacon → 2 hops → phone = 6).

### 15.4 Deployment math (40k event, ~20 hectares)

- **Corridor coverage:** beacons at 100–120m spacing along main walkways and at each cluster (stages, food court, entrance, camping, medical) → **~20–35 units**, i.e. under ~$1k of production hardware
- Combined with §14: dense-crowd chat already works at 1% adoption; beacons supply the connective tissue that previously demanded 5–10% adoption. **With beacons, the 1% regime is venue-wide.** The adoption table's top row effectively inherits the bottom row's experience.

### 15.5 Organizer features unlocked

- **Official announcements:** an ops laptop/phone wired to any backbone beacon injects signed #Event Updates (§17) from one place and reaches the entire grounds; beacons also broadcast EVENT_INFO so arriving phones learn the event supports verified updates.
- **Health monitoring:** beacons publish a heartbeat (peer counts, relay volume, uptime) on a reserved diagnostics channel readable by an ops dashboard over the backbone; a dead beacon is visible in minutes.
- **Coverage telemetry (finding 7 — privacy-preserving by construction):** beacons export only aggregate counts, never raw `sender_id`s. Unique-device estimates use a **cardinality sketch (HyperLogLog) over `HMAC(daily_event_salt, sender_id)`**, computed locally on each beacon; the salt rotates daily and is never exported, so the exported number is a count with no per-device identifiers and cross-beacon correlation of individuals is not possible from the telemetry. This makes "no cross-beacon tracking" an enforced property, not a promise (the earlier design leaked raw stable IDs, which would have made tracking trivial — corrected). Message content is never touched. State the mechanism in the partnership agreement and privacy copy, because "festival installs tracking beacons" is a headline the design must rebut structurally. (Note the residual, separately disclosed fact from §18-B1: any always-on BLE device broadcasting a stable `sender_id` is sniffable in the raw by third parties on the air — that is a property of the plaintext mesh, independent of and not worsened by beacon telemetry.)

### 15.6 Trust boundary

Beacons get **zero protocol privileges**: no TTL refresh, no rate-limit exemption for traffic they originate, no special message types phones must obey. The `infra` bit is a scheduling hint, not authority — a malicious actor faking it gains nothing except winning suppression races in its own radio radius, which the per-link flood cap already bounds. This keeps the security model unchanged: the mesh never trusts any single node, including ours.

### 15.7 Beacon Mode (in-app)

A settings toggle that turns any phone into a high-capacity relay — for organizers running the retired-phone pilot, and for power users (the friend with the 20,000mAh power bank at camp). It reuses every mechanism already specified; it's a configuration profile, not a fork:

| Behavior | Normal app | Beacon Mode |
|---|---|---|
| Relay hold-off (§3.3) | 80–400ms | 10–40ms — enters neighbors' digests fast, enabling per-egress suppression where the beacon covers those peers (offload magnitude is measured, not assumed — §15.2) |
| Relay probability (§3.4c/d) | adaptive | always 1.0 |
| Forwarding budget (§11) |120-frame burst +2 frames/s|No extra mode bucket; all aggregate limits still apply|
| Concurrent connections | 3–6 | 7–8, slots reserved first for phones announcing 0–1 peers |
| Store-and-forward |15 min/500/256 KiB encoded|60 min/5000/5 MiB encoded; allocated caps MC-007|
| Scanning | balanced/duty-cycled | continuous low-latency |
| ANNOUNCE `infra` bit | never | set **only while external power is connected** |

**Power guardrails.** Beacon Mode is designed around external power, not required to have it: enabling while unplugged shows a drain warning ("Beacon Mode uses more power; keep your phone charged"), and the mode **auto-downgrades to normal operation at 30% battery** (notifying the user) so nobody accidentally kills their phone playing hero. An "auto-beacon while charging" toggle lets a phone on a power bank flip into beacon duty whenever plugged in and back out when unplugged — zero attention required. Note the `infra` bit tracks charging state, not the toggle: an unplugged beacon-mode phone still relays generously but doesn't claim infrastructure permanence to the mesh.

**Beacon screen.** Activating the mode swaps the UI for a dim, burn-in-safe status display: peers connected, packets relayed, messages cached, uptime, battery/power state, and a hold-to-exit button (prevents pocket-taps from killing a deployed relay). Screen-off operation works fully on Android (foreground service); the screen display is for the taped-to-a-pole case where ops staff want at-a-glance health.

**Platform note.** Beacon Mode ships on Android only at first. iOS backgrounding restrictions (§8.4) make an iPhone a poor dedicated relay — it would need the screen permanently on in Guided Access to match, which is battery-hostile and fragile. The settings entry on iOS explains this rather than offering a degraded version.

**Trust unchanged.** A beacon-mode phone gets the same zero privileges as hardware beacons (§15.6). It volunteers more work; it gains no authority.

---

## 16. Keeping the Mesh Alive: Retention & Incentives

The app has a structural advantage over most cooperative systems: **receiving and relaying use the same radio.** Nobody can listen without contributing in the stock client, so the enemy isn't defection — it's attrition (users toggling BT off on battery folk-beliefs, or drifting away between messages). Three layers, in priority order:

### 16.1 Remove the disincentive (most of the battle)

- **Prove the battery cost, don't claim it:** an in-app meter shows mesh drain. Where the OS exposes per-app battery attribution it shows the *measured* figure ("Mesh used ~2% in the last hour, measured"); where attribution is unavailable it shows a clearly-labeled *estimate* from the §11 model ("estimated"), never presenting a model figure as a measurement. Trust in a number beats any "battery friendly!" copy. The Auto power default (§11.4) reinforces the story: "the app manages its own power draw based on your battery" is a stronger pitch than either always-full-power or always-throttled.
- **Teach the airplane-mode trick** (onboarding + a contextual tip when signal bars die): airplane mode with BT re-enabled kills the expensive cell-tower hunting and keeps the mesh alive — the phone lasts *longer* than in normal no-signal operation. If users internalize this one trick, the app flips from "battery drain" to "battery saver" in their mental model.

### 16.2 Make turning it off carry a felt (and honest) cost

- **Lost history is real:** store-and-forward covers ~15 min; there is no server to backfill. Surfaced copy: "Messages sent while you're off are gone — the mesh can't replay what it never gave you."
- **Your friends lose you:** shown at the moment of quitting/disabling — "Your channel melodic-techno-valley won't be able to reach you." The friend-group use case is the strongest retention force in the design and costs nothing.
- **Exclusive value via organizers:** announcements posted *only* to verified #Event Updates (§17) — secret sets, meet-and-greets — make an open mesh connection the thing people check, festival-radio style.

### 16.3 Make contributing feel good — gamification with guardrails

- Live contribution stat: "Your phone carried 1,240 messages for ~85 people today."
- Locally-earned titles: **Data Mule** (relayed between disjoint peer clusters — detectable from local connection history), **Night Shift** (relaying 2–6am), **Backbone** (top-decile relay volume vs. own history).
- **"Mesh Wrapped"** end-of-event share card (image export) — doubles as the morning-after viral loop.
- **Guardrail:** all stats are locally measured and private by default. **No leaderboards over the mesh** — broadcast rankings invite Sybil/fake-relay gaming that pollutes the network for points, and cost airtime. Self-shared brag cards deliver the social payoff without the attack surface.

### 16.4 Explicitly ruled out: transferable rewards

No tokens, credits, or payments for relaying. The moment relaying earns something transferable, Sybil-flooding the mesh with fake traffic becomes economically rational and §6 becomes an arms race. The festival social contract — *we keep each other connected* — is the stronger motivator here, and it's free.

---

## 17. Organizer-Verified #Event Updates

Restricts posting in #Event Updates to event staff — without any server — via per-event signing keys. Everything is opt-in and backward compatible: events that don't adopt a key get today's open-channel behavior.

### 17.1 Cryptography

**Key hierarchy.** An event has an offline **root** Ed25519 keypair. The root signs **staff credentials**; a staff device signs individual messages with its staff key. Verification chain: root → staff credential → message. Root private key never leaves ops custody; staff devices hold only their own staff private key + a root-signed credential.

**Staff credential (the wire object, finding R3).** A credential is a fixed binary structure, transported both in the staff-provisioning QR and inline in the first signed message a staff device sends per session (so any recipient can verify without a side channel):

| Field | Size | Notes |
|---|---|---|
| `cred_version` | 1B | `0x01` |
| `event_root_id` | 8B | SHA-256(root pubkey)[:8] — identifies which event |
| `staff_pubkey` | 32B | The staff device's Ed25519 public key |
| `not_before` | 4B | Unix seconds |
| `not_after` | 4B | Unix seconds (≤ event end + 24h recommended; staff shifts ≤ 12h) |
| `label_len` + `label` | 1B + ≤16B | Display label, e.g. "MainStage Ops" |
| `root_sig` | 64B | Ed25519 by the root covering all credential bytes before this signature, including label_len; exact domains/encoding are MC-008 §5 |

Credential size ≈ 130 bytes.

**Signed message block (organizer, `flags.signed` + `sig_type=1`).** Located after the five pin bytes that follow `text` (§2.3):

| Field | Size | Notes |
|---|---|---|
| `event_root_id` | 8B | Which event this claims authority under |
| `staff_key_id` | 8B | SHA-256(staff_pubkey)[:8] — identifies *which staff device* even when the credential is omitted (finding R3: without this, a credential-omitted message can't be matched to a cached credential) |
| `cred_included` | 1B | 0 = credential omitted (use cache); 1 = full credential follows |
| `cred_len` | 2B | Byte length of `credential` (0 when omitted) — explicit length so the parser locates `staff_sig` unambiguously (finding R3) |
| `credential` | `cred_len` B | The ~130B credential when included |
| `staff_sig` | 64B | Ed25519 by the staff key over the canonical transcript below |

**Signature coverage:** immutable header bytes 0–2 and 4–25 plus every payload byte before staff_sig, including pin state/expiry, cosmetics, all lengths and credential metadata/bytes. TTL alone is mutable. Use MC-008 §5 exact domain/transcript encoding; independent full-construction review remains MC-022. Staff verification proves credential authority; the signed header sender_id remains a separate device-identity claim, not proof of that device key or a friend pin.

**Verification:** (a) resolve `event_root_id` → adopted root pubkey; if none, unverified (relayed per §17.3, not badged). (b) Resolve the staff key: if `cred_included`, verify `root_sig` and cache the credential **keyed by `(event_root_id, staff_key_id)`**; else look up that cache key. If neither yields a credential, mark **unverified-pending** and emit CRED_REQ (below). (c) **Binding checks (finding R5.6), all mandatory:** `staff_key_id == SHA-256(credential.staff_pubkey)[:8]`, and `credential.event_root_id == message.event_root_id` — without both, an attacker could pair a valid credential with an unrelated message or key id. (d) Check `not_before ≤ now ≤ not_after ≤ root expiry` (no implicit lifetime extension). (e) Verify `staff_sig` with `staff_pubkey`. All pass ⇒ Event Staff badge.

**Credential distribution — multi-hop safe (finding R5.6).** One-hop CRED_REQ was insufficient: the immediate peer is often an ordinary relay that never adopted the root and holds no credential. So credentials are a **floodable, cacheable object**:

- **CRED_OFFER (type `0x08`)** carries exactly one staff credential (~130B) and nothing else. It floods the mesh like any other packet (TTL 7, dedup, rate-limited normally) and **any node caches it regardless of whether it adopted that event's root** — public data still consumes memory/work: enforce MC-007's 128-entry/64-KiB credential cache and keep unverified entries separate from adopted trust. A node with an adopted root additionally verifies `root_sig` before trusting it; nodes without the root cache it opaquely for later.
- A staff device emits CRED_OFFER on launch, on first post to a channel, and every 5 minutes while active — cheap (130-byte packets at that cadence are negligible) and it means credentials propagate ahead of, and independently of, the messages that need them.
- **CRED_REQ (type `0x07`)** remains as a fast path: payload `event_root_id ‖ staff_key_id`, unicast to the arrival peer, rate-limited 1/5s/link. **Any** node holding the credential in cache may answer with a CRED_OFFER — not only the staff device — so the request usually resolves locally. If the peer has neither the credential nor a route, it simply doesn't answer (no negative response needed); the requester relies on CRED_OFFER flooding and retries at most twice, 5s apart, then waits.
- Pending credential recovery expires after 30 seconds under MC-007; loss, missing offers or budget exhaustion may leave the message unverified. Multi-hop recovery where the immediate peer lacks the credential remains a required test; no seconds-level recovery guarantee is assumed.

**QR encodings (single canonical form each).** Adoption QR (public): `meshfest://event/<base64url(0x01 ‖ root_pubkey[32] ‖ root_not_after[4] ‖ root_self_sig[64])>/<url-encoded name>` — a 101-byte bundle. The root self-signature must cover its first 37 bytes (version, public key and expiry); Use the exact MC-008 §5 domain prefix. The recipient derives `event_root_id = SHA-256(root_pubkey)[:8]`; it is never a separate URL field. Staff provisioning is `meshfest://staff/<base64url(credential)>/<base64url(staff_seed[32])>`, with an exact 32-byte Ed25519 seed whose derived public key must match the credential. The canonical URI grammar (§5.3) governs decoding and confirmation. The **root private key is never in any QR or on any device**.

**Wire types added:** `0x07` CRED_REQ (above). Verification ≈ two Ed25519 verifies, sub-millisecond.

Worst-case organizer CHAT is 26 header + 312 clear CHAT payload + 5 pin fields + 213 organizer block = **556 logical bytes**, five fragments at the 128-byte admission slice or four at the 164-byte example slice. Omitting the credential gives **426 bytes**. CRED_OFFER carries the exact 114–130-byte credential (140–156 logical bytes). These sizes include the explicit cosmetic and pin fields.

### 17.2 Trust bootstrap — how phones learn the real key

The mesh itself can never be the root of trust (anyone can broadcast a key). Adoption happens **out of band only**:

1. **Official QR codes** at entrances, screens, totems, wristbands use the single canonical adoption layout (§17.1): `meshfest://event/<base64url(0x01 ‖ root_pubkey[32] ‖ root_not_after[4] ‖ root_self_sig[64])>/<url-encoded name>` — `event_root_id` is *derived* from the pubkey (its hash), never carried separately, so there is one layout, not two. Scan → confirmation sheet → root trust anchor adopted.
2. **The same link shared digitally** pre-event (works via §5's link machinery).
3. **Beacons broadcast a new `EVENT_INFO` packet** (type `0x05`: event name + `event_root_id`, *not* trusted for adoption) so arriving phones get a prompt: "This event supports verified updates — scan any official QR to enable." Discovery over the mesh, trust only via QR.

A phone can hold multiple event roots. **Root-adoption expiry is independent of any staff credential (finding R5 non-blocking):** the adoption QR carries the root's own `root_not_after` (a signed field in the adoption payload), and the phone drops the adopted root at that time regardless of which staff credentials it has seen. Staff credentials have their own, necessarily shorter, `not_after` (§17.1). Two independent clocks: the root anchor's lifetime, and each staff device's shift.

### 17.3 Client behavior after adopting a key

| Aspect | Behavior |
|---|---|
| Display | Valid-signature messages show a ✓ "Event Staff" badge. Unsigned/invalid messages in #Event Updates are **hidden behind a collapsed "unverified messages" drawer** (viewable, never silently deleted — the mesh shouldn't memory-hole content, and false-positive hiding must be recoverable). |
| Composer | Replaced with "Only event staff can post here" unless the device holds the private key (§17.4). |
| Rate limit | Structurally organizer-signed Event Updates use the sender bucket capacity 10/refill 1 per 6s instead of ordinary capacity 2/refill 1 per 30s, preserving the mixed-adoption allowance. All sender-wide, link/node, crypto, queue and forwarding limits still apply. This structural rate classification grants no badge: only an adopted root and valid signature establish staff authority. Invalid signatures remain untrusted. |
| Replay guard | MC-008 §4/5: 48h-past/300s-future signed-content window, persistent atomic accepted-effect tombstones, conflict handling and clock-uncertainty refusal. Stale content grants no current authority; relay dedup alone cannot supply this protection. |
| Pinning | Five bytes after CHAT text: pin_state:u8 then pin_expiry:u32, before the organizer block. State 0 requires expiry 0; state 1 requires positive expiry within both root and credential lifetimes. All bytes are signed; only authenticated, adopted, unexpired authority affects pin display. |
| No key adopted | Legacy behavior: open channel, everything displays normally. EVENT_INFO sightings produce the §17.2 prompt. |

### 17.4 Posting side (staff)

- **Organizer console:** staff import the private key via a staff-only QR (or passphrase entry); the app gains a "post as Event Staff" composer in #Event Updates with the pin control. Key held in OS keystore; "forget key" wipes it.
- **Ops injection:** a laptop/phone at the backbone (§15.5) posts once, reaches the whole venue.
- Staff messages still carry the device's normal `sender_id` and nickname (e.g. "MainStage Ops") — the signature conveys the authority, not the identity.
- **Private keys never live on beacons** (§18-M5). Beacons relay signed traffic; they cannot produce it. A stolen or firmware-compromised beacon therefore yields no signing capability.

### 17.5 Relay policy and honest limits

Relay stays **content-neutral**: nodes relay unsigned #Event Updates traffic normally (enforcement is display-layer). Rationale: mixed enforcement (some nodes dropping, some relaying) buys little suppression while breaking the relay-everything invariant that §3's behavior depends on; hiding-not-dropping also degrades gracefully for phones that never scanned a QR. Honest limit, stated in UI copy: **a user who never scans an official QR can still be shown spoofed "event updates"** (unverified-styled at best, unstyled if they ignore the prompt). Mitigations are the EVENT_INFO nudge and physical QR ubiquity — print it on every screen loop. This also motivates the §18 roadmap item of shipping well-known events' keys with app updates when online.

---

## 18. Security Model & Hardening Requirements

Historical review rounds below describe design changes at the time. Their “resolved” language means specified, not implemented/tested; any values or claims superseded by §0 and the active tickets are non-normative.

Findings from the design-stage adversarial audit, with their required mitigations. IDs are referenced from the sections they modify. Severity: **B** = blocker (fix before ship), **M** = major, **m** = minor. **Evidence status: specified at design stage only.** Each finding needs implementation and validation evidence, plus independent review where required; §0 records approved corrections and outstanding decisions. Items marked *(v2)* are mitigated in v1 and fully closed in §20.

### 18.1 Can an attacker reach the phone itself?

The design reduces the application attack surface through the following measures. These are defenses, not proof that compromise is impossible; native libraries, FFI, platform APIs and implementation bugs remain in the assessment scope:

1. **Sandbox.** The app can only touch its own container and the permissions it holds (Bluetooth, notifications). Even total compromise of the app process yields message history, nickname, and `sender_id` — not contacts, photos, SMS, or other apps.
2. **No code-execution primitives in the protocol.** No scripting, no markup interpretation, no dynamic content, no file transfer, no URL fetching, no deserialization of arbitrary object graphs. The only input is length-prefixed bytes → fixed-shape structs → UTF-8 text.
3. **Text-only is a security feature.** Excluding images/video removes the single largest historical RCE class on mobile (image/video codec parsers). Keep it excluded.
4. **Memory-safe parser.** The core is Rust with `#![forbid(unsafe_code)]`, which reduces memory-corruption risk in owned core code. It does not prove the safety of dependencies, native code, FFI boundaries or the OS Bluetooth stack.

**Residual risk outside our control:** vulnerabilities in the *OS* Bluetooth stack (BlueBorne/BleedingBit class). Mitigation: require Android 10+/iOS 15+, never implement custom L2CAP/pairing, and document that patch hygiene is the user's OS vendor's job.

### 18.2 Blockers

- **B1 — Persistent `sender_id` enables physical tracking; anonymity must not leak the stable ID.** A passive sniffer logs `sender_id` + RSSI over time and reconstructs an attendee's movements. *Decision:* the ID stays **stable** — rotation would break the friend-finding that is the app's core value, and the tracking risk applies to any always-on BLE device the user carries anyway. This is **managed by disclosure**, not eliminated: the app never claims to be private-on-air, and onboarding + the #Confessions composer say so. *Required fix, the part that is non-negotiable:* anonymous posts (#Confessions only) MUST use a **single-use `sender_id`** unlinkable to the stable ID, so the one place the app promises anonymity actually delivers it (§7). Anonymity is offered on no other channel. Platform MAC randomization stays on. Residual tracking of non-anonymous traffic is accepted and documented (§18.5).
- **B2 — Spoofable "Event Staff" badge.** If the badge is rendered from message/nickname text, an attacker sets nickname `✓ Event Staff` and posts a fake evacuation or gate-closure instruction — a genuine crowd-safety issue at 40k scale, requiring no crypto break. *Fix:* badge is separate UI chrome, never text; nickname sanitizer strips badge glyphs and the words "staff"/"official"; badge renders **only** after Ed25519 verification against an adopted key (§10.2, §17.3).
- **B3 — Plaintext local message DB in cloud backups.** A stolen or forensically imaged phone (or an extracted cloud backup) exposes all "private" channel history. *Fix:* SQLCipher with keystore-held key + backup exclusion on both platforms (§9).

### 18.3 Major

- **M1 — Fragment reassembly memory exhaustion.** Repeated fragment-0-of-4 packets with random `frag_group` allocate buffers held for 30s → OOM kill of nodes within radio range. *Fix:* per-peer and global concurrent-group caps with oldest-eviction (§2.4).
- **M2 — SYNC amplification / battery-drain attack.** A small SYNC_REQ can elicit many batch packets; looping it drains victims' batteries and airtime. *Fix:* paginated SYNC with per-peer/per-link session admission, a per-session item+byte budget, and a global served-item ceiling (§2.6). (Current values live in §2.6; SYNC_REQ is a 546-byte fragmented logical packet.)
- **M3 — Connection-slot exhaustion isolates victims.** One attacker with spoofed identities fills all 6 GATT slots, cutting a phone out of the mesh. *Fix:* reserved rotating slots, idle-peer eviction, RSSI-diversity scoring, inbound connect rate cap (§8.1).
- **M4 — Panics and integer overflow in the core = crash-loop DoS.** A malformed packet that trips a slice-index panic crashes the app repeatedly; a panic unwinding across the UniFFI boundary is undefined behavior. *Fix:* all length arithmetic uses `checked_*`/`saturating_*`; parsing returns `Result`, never indexes unchecked; `catch_unwind` at every FFI entry point converting panics to errors; `overflow-checks = true` in release; cargo-fuzz on the parser as a **CI gate** with a persisted corpus, not just a test-plan bullet.
- **M5 — Event signing key compromise is unrevocable offline.** A leaked staff QR (photographed, shoulder-surfed) grants permanent staff authority with no CRL possible in a serverless mesh. *Fix:* keys carry a mandatory expiry ≤ event duration + 24h; staff QR is displayed once on a trusted screen and never persisted as an image; private keys never on beacons (§17.4); rotation procedure documented for ops. v2 adds daily subkeys signed by the event root.
- **M6 — `sender_id` spoofing enables targeted impersonation.** IDs travel in cleartext, so an attacker can copy a specific person's ID *and* nickname, reproducing their color+suffix exactly — worse than no disambiguator, because a naive UI would imply verified continuity. *Fix (v1):* (1) §7.2 wording never presents color+suffix as verification (the 4-char suffix is for honest-collision resistance only — suffix length is irrelevant to a deliberate impersonator, who can spoof any suffix); (2) **Verified Friends (§7.4)** gives real cryptographic authentication for pinned friends — a copied ID/nickname produces an unsigned or invalid-signature message that the app flags as *"claims to be Sarah · not verified,"* so impersonation of a verified friend is detected. Public unsigned traffic remains spoofable by accepted design (§18.5). Global signing (all senders, all channels) is the v2 extension (§20).
- **M7 — Seen-cache pollution forces stale re-relay.** Flooding unique `msg_id`s evicts legitimate entries from the 4096-slot LRU, so already-delivered messages get relayed again (wasted airtime and battery, duplicate display). *Fix:* partition the cache per source peer so one peer cannot evict another's entries; on suspected pollution, fall back to a coarse time-bucketed bloom filter.

### 18.4 Minor

- **m1 — Unicode abuse:** bidi overrides to reverse displayed text, homoglyph nicknames, zalgo layout breakage. *Fix:* sanitizer in §10.2.
- **m2 — Deep-link abuse:** a crafted `meshfest://` link auto-joining a channel, adopting an event key, or **pinning a friend**. *Fix:* every deep link (`/j/` channel, `/event/` key, `/friend/` pubkey) requires an explicit confirmation sheet showing decoded contents (§5.2/§17.2/§7.4) — a friend link in particular shows the pubkey fingerprint and asks the user to confirm they trust the source, since a link received over the network lacks the in-person guarantee of a scanned QR; reject malformed base64/words; treat the handler as an untrusted-input boundary and fuzz it (§12.1).
- **m3 — Clipboard leakage:** copied channel words persist in the OS clipboard (and Android clipboard-read by other apps). *Fix:* mark clipboard items sensitive where the platform allows; prefer the share sheet and QR over copy.
- **m4 — Tapjacking on confirmation sheets:** an overlay app tricks the user into confirming a channel join or key adoption. *Fix:* `setFilterTouchesWhenObscured(true)` on all confirmation UI.
- **m5 — Supply chain:** the Rust core's transitive dependencies are an unaudited attack surface. *Fix:* `cargo-audit` + `cargo-deny` in CI, pinned versions, dependency budget (target: sha2, ed25519-dalek, rusqlite, rand only), manual review of any addition.
- **m6 — Beacon physical compromise:** stolen/reflashed beacons can sniff and log. *Fix:* signed firmware, no network services listening, tamper-evident enclosures, and the §15.6 rule that beacons hold zero privileges and zero keys.
- **m7 — Debug logging leakage:** message content in logcat/Console is readable by other apps on older Android and by anyone with the device. *Fix:* no content logging at any level; strip logging in release builds; redact `sender_id` to 2 chars in diagnostics.

### 18.5 Accepted, unfixable, or out of scope

- **Jamming.** A 2.4GHz jammer or aggressive flooder can deny service locally. No mitigation exists at this layer; not claimed against.
- **Passive eavesdropping.** Channel traffic is plaintext on the air by design (§4.4), documented in-product. The exception is **DMs (§7.5)**, which are end-to-end encrypted — the one confidential surface; channels remain readable by relays and sniffers.
- **Movement tracking of non-anonymous traffic.** The stable `sender_id` (B1 decision) lets a sniffer correlate a device's positions over time. Accepted as the cost of reliable friend-finding, applies to any always-on BLE device regardless, and disclosed in onboarding. Anonymous #Confessions traffic is exempted via single-use IDs.
- **Sybil-based spam.** Bounded, not eliminated, by per-link caps (§6.5).
- **Malicious relay dropping.** A hostile node can silently refuse to relay. Flood redundancy makes this ineffective unless the attacker controls a cut vertex of the topology.

### 18.7 Second adversarial review (design-stage) — protocol findings

A second review targeting protocol/platform internals (vs. §18's security focus) surfaced seven confirmed high-severity design inconsistencies, all resolved in-spec before M1 freeze:

| # | Finding | Resolution |
|---|---|---|
| R1 | Fragmentation vs. dedup ordering undefined; spoofable `frag_group` | §2.4 rewritten: fragmentation is a per-link transport envelope reassembled *before* the §3.1 pipeline; reassembly key = arrival link + `frag_msg_id` + `frag_group`; dedup/TTL/sig operate only on logical packets |
| R2 | Counter-based suppression wrongly assumed shared-broadcast coverage on a GATT overlay; could sever bridges | §3.4 rewritten to per-egress-peer suppression with explicit per-link knowledge and forced relay on sole paths; `peer_count` demoted to advisory hint; cut-vertex simulator case now gates thresholds |
| R3 | SYNC batch couldn't fit packet cap; Bloom filter grossly undersized (16–64B for 500 items) | §2.6 rewritten as cursor-based multi-packet SYNC; Bloom sized correctly (k=6, ~520B, fragmented); §2.5 recent-digest resized to ~256B with real math |
| R4 | Interop matrix required impossible Android→backgrounded-iOS discovery; iOS can't advertise ANNOUNCE metadata | §8.4/§8.5 corrected against Apple docs; all metadata moved off advertisements to GATT (§2.5.1); matrix split into achievable discovery vs. established-connection cases; impossible case out of gate |
| R5 | "Flooder contained to 1 hop" unenforceable; `sender_id` rotation bypassed buckets | §6.5 adds a per-link aggregate ingress bucket independent of claimed identity; gate rewritten as a measurable volume bound |
| R6 | Org key expiry contradictory (7-day vs. event+24h); no signed validity; emergency bursts throttled by non-key relays | §17.1 adds signed `not_before`/`not_after` credential + staff subkeys (root stays offline); §17.3 grants verified bursts a raised relay budget on all nodes |
| R7 | `neverForLocation` claim conflicts with RSSI proximity; stable-ID beacon telemetry enables the tracking it disclaims | §8.3 defers the permission choice to a pre-M0 ruling; §15.5 telemetry rebuilt on salted rotating HLL sketches, never raw IDs |

Non-blocking review items also applied: version-tolerant relay (§2.8, prevents mixed-version partitioning); nickname length clarified as bytes not chars (§2.3/§7.2); timestamp ordering reconciled as display-hint-only with arrival order authoritative (§3.6); battery copy distinguishes measured vs. estimated (§16.1); storage corrected to four tables (§9). The review's verdict — do not freeze M1 as originally written — was accepted; the M0 feasibility spike (implementation plan) now gates the freeze.

### 18.8 Third adversarial review (wire-contract) — interop findings

A third pass checked whether independent Rust/Kotlin/Swift/ESP32 implementations could build interoperably from the text. Nine findings, all resolved:

| # | Finding | Resolution |
|---|---|---|
| R3.1 | Bridge detection relied on adjacency data the protocol never exchanges | §3.4(c) inverted to **default-to-relay under uncertainty**: suppress only with positive per-egress coverage evidence, force 1.0 otherwise — needs no topology inference; §8.1 stops treating `peer_count` as mutual-neighbor evidence |
| R3.2 | 520-byte SYNC filter vs. 512-byte "cap" | §2.1 defines separate `MAX_FRAME_BYTES` (244) and `MAX_LOGICAL_PACKET_BYTES` (1024); SYNC_REQ is a fragmented logical packet; fragments carry byte slices with `frag_msg_id == inner.msg_id` |
| R3.3 | Organizer subkey chain absent from wire format | §17.1 fully specifies the staff-credential binary object, both QR encodings, the root→credential→message chain, caching, and updated size math |
| R3.4 | DM "sign the ciphertext" underspecified; authenticated-replay possible | §7.5 gives the canonical transcript: HKDF salt/info, AES-GCM **AAD binding header+msg_id+recipient-tag+ephemeral-key+nonce**; authenticity from AEAD + enclosed sender pubkey; replay-with-modified-header negative vectors required |
| R3.5 | GATT backpressure non-normative | §8.2 adds a normative per-link transmit state machine (one in-flight write, readiness callbacks, retry/timeout, disconnect cleanup, priority fairness) |
| R3.6 | Superseded requirements still present | Swept: §6.4/§6.5/§3.1 reconciled into one display-vs-relay rule; §13 flood gate corrected; §18-M2 stale 64-byte text fixed; plan M3 permission + M5 matrix + M2 SYNC aligned |
| R3.7 | M0 exit gate didn't gate on spike results | Plan M0 gate now requires recorded spike results + permission decision before M1 freeze; iOS capacity pulled into M0 |
| R3.8 | Version-tolerant relay lacked validation/rate/size rules | §2.8 adds envelope-only validation, a conservative unknown-type per-link bucket, opaque-payload size cap, reserved-bit handling, explicit major/minor nibble encoding |
| R3.9 | Beacon battery-offload asserted, not derived | §15.2 demotes it to a measured hypothesis validated by per-phone TX measurement with/without a beacon |

Verdict accepted: M1 freeze still gated on completing the M0 spike **and** landing canonical binary golden vectors (§13) for every packet type, fragment, DM envelope, and organizer credential chain — the cross-implementation contract.

### 18.9 Fourth adversarial review (crypto & binary grammar) — findings

The fourth pass found two blockers (one a real vulnerability) and five high-severity gaps; all resolved:

| # | Finding | Resolution |
|---|---|---|
| R4.1 (blocker) | **DM encrypted but did not authenticate the sender** — an enclosed pubkey is a forgeable claim; anyone can encrypt to Bob and impersonate Alice | §7.5 replaced with **sender-authenticated encryption** (RFC 9180 auth-mode analogue): `key = HKDF(dh_ephemeral ‖ dh_static)` where `dh_static = X25519(sender_static_priv, recipient_static_pub)` — only Alice's private key produces it. Sender identity comes from which pinned key yields a valid tag, not an enclosed claim. Mandatory forgery negative-vector added |
| R4.2 (blocker) | Fragment grammar unparseable (`flags.fragmented` not at a stable offset), envelope math exceeded the MTU floor, version nibble encoding self-contradictory | §2.1 adds an explicit **4-byte outer frame header** with `frame_kind` discriminator; slice capacity derived at runtime (`maximumWriteValueLength`); `version` is a single integer (no nibbles) |
| R4.3 | Signed blocks located by "remainder"; staff cache keyed by root alone (multi-staff collision); no staff_key_id when credential omitted; two QR layouts | §2.3 adds `text_len`; §17.1 adds `staff_key_id` + `cred_len`, caches by `(event_root_id, staff_key_id)`, defines CRED_REQ (`0x07`) and a resend policy, and specifies one canonical QR each |
| R4.4 | iOS backpressure waited on `didWriteValueFor`, which never fires for writes-without-response | §8.2 split by platform: iOS uses the `canSendWriteWithoutResponse` / `peripheralIsReady` readiness model (no per-write completion); Android keeps the completion-callback model |
| R4.5 | SYNC_BATCH nested a ≤1024B packet in a ≤1024B packet (impossible); dedup/TTL/Bloom underspecified | §2.6 makes SYNC_BATCH a **transport container** carrying opaque stored bytes (exempt from the logical cap, never relayed); inner msg_id controls dedup; stored TTL used as-is (no laundering); exact Bloom (m=4096, k=6, double-hash, fixed salt); recovery via live flooding, not deterministic-FP retry |
| R4.6 | Wire freeze could precede crypto implementation | Plan splits into **base-transport freeze (M1)** and **full-wire freeze (end M2 + cross-language crypto vectors)** |
| R4.7 (minor) | Supporter badge unknowable (entitlement never broadcast) | §19.1 makes it a **self-asserted, spoofable cosmetic hint** (a status bit), zero trust weight — honest for a decoration |

Non-blocking also fixed: duplicated screen-off line removed (§11.4); Xcode/iOS tooling confirmed in M0 not M4.

> **Note on historical rows.** The tables in §18.7–§18.9 record what each review round changed *at that time*. Some cite values later superseded (e.g. `MAX_FRAME_BYTES = 244`, the major/minor nibble version encoding, the 25-item SYNC quota). They are kept as an audit trail; **§2–§17 are the normative spec** and always win.

### 18.10 Fifth adversarial review (transport grammar & DoS) — findings

| # | Finding | Resolution |
|---|---|---|
| R5.1 (blocker) | SYNC_BATCH had no representation in the outer frame grammar — declared "not a logical packet" yet told to borrow logical fragmentation (which requires a `msg_id` and caps at 1024), while still holding logical type `0x04` | §2.1 adds **transport-object frame kinds `0x02`/`0x03`** with their own 12-byte envelope, `transfer_id` reassembly key, `MAX_TRANSPORT_OBJECT_BYTES = 1200`, and `MAX_TRANSPORT_FRAGMENTS = 16`; §2.6 recasts the response as **SYNC_ITEM** (`obj_type 0x01`); logical `0x04` retired to reserved |
| R5.2 (blocker) | 25 items/session could not deliver "95% of a 500-message cache" (5%), and repeating sessions outlived the 15-min cache; "cursor-based" with no cursor field | §2.6 adds a real `cursor` to SYNC_REQ and `next_cursor`/`more_available` to SYNC_ITEM, **100 items or 32 KB per session with free in-session continuation**, newest-first walk; acceptance target restated honestly as **≥95% of the newest 100 within one session (≤30s)** |
| R5.3 (blocker) | Signature/AEAD verification ran *before* rate limiting, so invalid crypto traffic cost CPU/battery at line rate without consuming any budget | §3.1 restructured into **frame layer then logical layer**: the per-link budget is charged in **bytes and frames immediately after envelope parse (F2)**, before reassembly or crypto; added a **20 verify-ops/sec/link cap (F4)**; dedup moved before crypto; §6.5 bucket is now byte+frame based; new §13 ingress/DoS suite |
| R5.4 | DM primitive fixed but contract incomplete: tag derivation, QR key framing, X25519 lifecycle, all-zero rejection, KCI disclosure, and the M2 gate all unspecified | §7.5 specifies the exact tag (`HMAC-SHA256(pair_secret, "meshfest-dmtag-v1" ‖ u64_be(epoch))[0..4]`, ±1 epoch tolerance), the 65-byte two-key QR bundle, dual-keypair lifecycle, all-zero/malformed-key rejection per RFC 7748/9180, and discloses KCI; plan M2 gate now mirrors §13 including the forgery vector |
| R5.5 | Signed ANNOUNCE unimplementable — no binary layout, no timestamp, though the friend transcript binds one | §2.5 gives ANNOUNCE a complete field table **including `timestamp`**, exact digest Bloom (m=2048, k=6, own salt), signature-block location, and `ttl=1`/never-relayed rule; §2.5.2 does the same for EVENT_INFO |
| R5.6 | One-hop CRED_REQ couldn't reach staff or a cache across a mesh | §17.1 adds **CRED_OFFER (`0x08`)**, a floodable credential object any node caches regardless of adoption, emitted periodically by staff; CRED_REQ answerable by any caching node; plus the two mandatory binding checks (`staff_key_id` = hash of credential key; credential root = message root) |
| R5.7 | DM sender linkability undisclosed (stable `sender_id` + `sender_key_id` outside ciphertext) | §7.5 residual-metadata now states precisely what a sniffer learns and what would be required to fix it (encrypted key id + trial decryption), rather than implying sender privacy |

Non-blocking also fixed: organizer worst-case size recomputed and itemised (547 B with credential, 417 B without); **root-adoption expiry made independent** of staff credentials via a self-signed `root_not_after` in the adoption bundle; historical-row note added above.

---

## 19. Supporter Tier (consumer monetization)

A voluntary paid tier funding the small server/CI footprint and letting fans chip in. **Iron rule: it never gates communication.** Every messaging capability — channels, private channels' existence, friends, DMs, relaying, organizer updates — stays free forever, because the mesh needs maximum density to work (§14) and a paywall on reach is self-sabotage. Supporter perks are strictly cosmetic + convenience, and all of them run **fully offline** (no server check to unlock), since the app can't assume connectivity.

### 19.1 Perks

- **Color schemes / theming.** The "Afterhours" dark palette (§10.0) is the app default; a light theme ships free alongside it. Supporters unlock a theme pack modeled on well-known Linux terminal palettes — e.g. Solarized (Light/Dark), Gruvbox, Dracula, Nord, Tokyo Night, Catppuccin, Monokai, One Dark, Everforest. These are *inspired-by* palettes defined by our own hex tokens, not copied assets, to stay clear of any trademark/branding issues (see §19.4). Themes are pure design-token swaps (§10 / frontend-design), so they touch no protocol and no message data.
- **Extra private-channel slots.** Free tier includes the 3 public defaults + a reasonable number of private channels (e.g. **5**); supporters raise the cap (e.g. **to 30**). This is a local subscription-list limit only (§4.1) — it changes nothing on the wire, doesn't affect who can join a channel, and someone you share a channel with never needs the tier to participate.
- **Custom nickname color.** Supporters pick their own nickname color instead of the deterministic auto-color (§7.2). **Critical constraint:** the 4-char `sender_id` suffix and the verified-friend / organizer badges (§7.4, §17) are unaffected — a custom color is decoration, never an identity or trust signal, so it cannot be used to fake a verified look. The color travels only as an optional cosmetic hint in the sender's own packets; receivers may honor or ignore it, and it is never trusted for authentication.
- **Supporter badge (self-asserted, spoofable — finding R7).** A supporter's *own* client sets bit1 of the fixed four-byte CHAT/ANNOUNCE cosmetic field (§2.3/§2.5), so receivers *can* render it — resolving the round-three gap where entitlement was never broadcast and thus unknowable. It is explicitly **self-asserted and trivially spoofable** (any modified client can set the bit), carries **zero trust weight**, and is styled distinct from the ✓ verified-friend and Event Staff badges (§19.4). It's a flair, exactly like a nickname color — never an identity or trust marker. A surrounding message signature authenticates authorship of this hint, never payment or entitlement; unsigned hints remain freely spoofable. Neither form has authority over identity or trust chrome.

### 19.2 How entitlement works (offline-safe)

- Purchase via the platform store (Apple/Google IAP) — a one-time unlock or an annual "supporter" — validated through the store's **on-device receipt**, cached so the perks persist with no connectivity. No custom billing server, no login.
- Because perks are non-functional and non-gating, receipt-validation edge cases fail *open* toward the user (worst case a supporter briefly keeps cosmetics they paid for) — there is no incentive to crack something that only changes colors, and nothing security-relevant rides on it.
- Entitlement (the *paid* status) is device-local and validated by the store receipt (§19.2). The **badge hint** the client broadcasts is separate and unverified — it says "this client is presenting as a supporter," not "this device paid," and the mesh never treats it as proof. This keeps the paid check honest (store-validated, local) while letting the cosmetic actually appear to others without a global-signing dependency. Spoofing the bit gains nothing but a free ornament.

### 19.3 Why this shape

Consumer revenue here is a tip jar, not the engine — festival/venue licensing (§15, §17) is the real business, and this tier exists to (a) offset baseline costs, (b) give enthusiasts a way to support the project, and (c) do so without ever touching the network-effect-critical free experience. Modeling the paid themes on terminal palettes is also on-brand: the audience skews technical, and "Gruvbox for your festival chat" is the kind of detail that markets itself.

### 19.4 Guardrails

- **No pay-to-impersonate.** Cosmetic color and badge must never be renderable in a way that mimics the verified ✓ or Event Staff chrome; the nickname sanitizer + badge-is-separate-chrome rules (§10.2, §18-B2) already enforce the structural separation, and supporter cosmetics live in that same non-trusted lane.
- **No IP leakage.** Terminal-inspired themes use original hex tokens under our own names; ship them as our color definitions, not vendored theme files, and keep names descriptive rather than trademark-laden where a mark is involved.
- **Accessibility floor.** Every shipped theme (free and paid) must pass WCAG AA contrast for message text; a pretty palette that hurts readability doesn't ship.
- **Offline honesty.** Nothing about the tier implies the app needs the internet to function; the store interaction is the only online touchpoint and it's optional.

---

## 20. Deferred to v2 (designed-for, not built)

- **Encryption**: `flags.encrypted` + HKDF(three words) → AES-256-GCM; nickname moves inside ciphertext
- **Global signing / Sybil resistance**: v1 already ships per-device keypairs, `sender_id = hash(pubkey)`, and signed friend/organizer messages (§7.4, §17). v2 extends signing to *all* traffic by default (airtime permitting) and uses key-fingerprint identity for Sybil-resistant rate limiting — closing the residual spoofability of public unsigned messages (§18-M6)
- **Daily organizer subkeys** signed by the event root, limiting the blast radius of a leaked staff key (§18-M5)
- **DM forward secrecy & encrypted group chats**: upgrade §7.5 DMs from RFC 9180 Auth to a Double Ratchet (forward secrecy + post-compromise recovery); add small-group encrypted chats (closed roster of verified friends, roster changes trigger rekey) landing in the Messages tab (§10.1), whose everything-encrypted contract and list layout already anticipate them
- **Pre-distributed event keys**: app fetches known events' public keys (§17) while online, removing the QR-scan requirement for major partnered festivals
- **Wi-Fi Aware / Wi-Fi Direct transport** as a second, higher-bandwidth rail on Android
