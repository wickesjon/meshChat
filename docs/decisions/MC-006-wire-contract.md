# MC-006 — Canonical wire and discovery contract

Evidence state: specified, with arithmetic and structural examples checked; no codec implementation or cryptographic review is claimed.

This contract resolves the base-wire decisions delegated by design §0.2. The subsequent [MC-008 contract](MC-008-crypto-contract.md) specifies encrypted profile01, exact signature transcripts and whole-only transport03 LINK_PROOF. Read both contracts together; MC-022 independent review/full-wire freeze remains required. MC-008's extension rules supersede the earlier pending-allocation statements below without changing base field offsets or clear payloads.

## 1. Encoding and directional admission

Offsets and lengths count bytes. Integers are unsigned, big-endian. Concatenation introduces no alignment padding. A GATT value contains exactly one outer frame; there is no batching prefix. Reject truncation, inconsistent lengths and trailing bytes before interpreting payloads. Text must be valid UTF-8 within its byte bound and pass design §10.2 sanitization. Authenticate received bytes before constructing a normalized display copy; normalization must not change the bytes verified or forwarded.

For each direction, compute `C = min(local native transmit limit, peer receive limit, 512)`. Query write-without-response and notification limits independently. A missing native limit is not permission to assume 512. Both directions must support at least **146 bytes** before admitting ordinary traffic:

`146 = 4 outer-header bytes + 14 logical-fragment bytes + ceil(1024 / 8)`.

Thus the smallest admitted link carries 128 logical bytes per fragment and every allowed 1,024-byte logical packet fits in eight fragments. A transport fragment has a 130-byte slice at the same capacity; 1,200 bytes fit in ten of the sixteen allowed fragments. The 182-byte example in the earlier design gives 164/166-byte slices, but is not the admission floor or a measured device guarantee.

Below the floor, refuse admission with an unsupported-capacity result; do not silently shorten supported messages, exceed fragment limits or fall back to a different grammar. A decrease below the admitted directional capacity cancels queued sends and partial reassembly and restarts admission. Use later increases on a new admission as well, so a transfer never changes its capacity midstream. Device support and real throughput remain separate physical radio evidence.

## 2. Outer frames and reassembly

| Offset | Bytes | Outer field |
|---:|---:|---|
| 0 | 1 | `kind`: 00 whole logical, 01 logical fragment, 02 whole transport, 03 transport fragment |
| 1 | 1 | `flags`: zero; reject nonzero values |
| 2 | 2 | `body_len`: must equal the remaining value length |

The entire value, including this four-byte header, must fit the admitted receiving direction. During bootstrap, the only permitted frame is whole HELLO and it must fit the known local native receive limit. Unknown kinds reject. A whole logical body is one complete 26–1,024-byte logical packet. A whole transport body is a one-byte object type followed by its object body. Object types are a separate namespace: 01 is SYNC_ITEM, 02 is HELLO; other types reject. MC-008 may allocate its explicitly reviewed proof extension later. An unknown object type never reserves a reassembly buffer.

| Offset | Bytes | Logical-fragment envelope |
|---:|---:|---|
| 0 | 8 | `frag_msg_id` |
| 8 | 2 | `group` |
| 10 | 1 | `index`, zero-based |
| 11 | 1 | `count`, 1–8 |
| 12 | 2 | `total_len`, 26–1,024 |
| 14 | remaining | Nonempty slice of the encoded logical packet |

| Offset | Bytes | Transport-fragment envelope |
|---:|---:|---|
| 0 | 1 | `object_type`; only fragmentable known types permitted |
| 1 | 2 | `transfer_id` |
| 3 | 1 | `index`, zero-based |
| 4 | 1 | `count`, 1–16 |
| 5 | 2 | `total_len`, object-body length, excluding the type byte |
| 7 | 5 | Reserved, all zero; otherwise reject |
| 12 | remaining | Nonempty slice of the object body |

SYNC_ITEM bodies have length 11–1,035 and may fragment. HELLO must be whole; fragmented HELLO rejects. The absolute transport-body ceiling remains 1,200 for reviewed future extensions, not permission to exceed a known type's tighter bound.

Encoders use whole form when it fits. Otherwise they fill each slice to capacity except the last. Decoders accept other nonempty partitions within the same capacity/count/total bounds; this permits harmless partition variation without changing the logical bytes. Require `index < count`, `total_len >= count`, and `total_len <= count * maximum_slice` before allocation. The sum of accepted slices cannot exceed `total_len` and must equal it at completion.

Logical groups are keyed by `(arrival link, frag_msg_id, group)`; transport groups by `(arrival link, object_type, transfer_id)`. Count and total length must agree across a group. Identical duplicate indices have no effect. A conflicting duplicate or inconsistent metadata aborts that group; it never overwrites an earlier slice. After logical reassembly, the inner `msg_id` must match `frag_msg_id` before entering the mesh pipeline.

Incomplete groups expire 30 seconds after their first admitted fragment; duplicates cannot extend the deadline. Existing concurrency bounds are eight per link/64 globally for logical groups and four per link/32 globally for transport. Overflow evicts the oldest incomplete group.

An aborted or evicted incomplete group becomes a rejected-group entry retaining that group's original deadline. If the first observed fragment is rejected before admission, anchor its rejected-group deadline at that observation plus 30 seconds instead. Create such an entry only when a complete fragment envelope provides the scoped group key; truncated envelopes and unknown transport types are dropped without group state. Admission budgets still apply before any tracking allocation. While an entry exists, drop fragments with that scoped key without allocating reassembly storage; repeats never reset its deadline. At the deadline remove the entry; later fragments may start a new attempt under ordinary admission rules. A whole packet or a different group/link is not blocked by that entry. Never poison accepted-message dedup with a rejected fragment's claimed `msg_id`. MC-007 fixes aggregate buffer bytes, rejected-attempt ceilings and their overflow policy before MC-010. Disconnect frees all link state.

## 3. Discovery and duplicate links

Production discovery uses these fixed UUIDs, distinct from MC-004 probes:

| Role | UUID | Property |
|---|---|---|
| Service | `c11b1d76-75da-4ae0-b0fe-cb273609c526` | Advertised service only |
| TX | `78db2e71-ff31-46e0-8a8e-371f8199bcdc` | Write without response, central → peripheral |
| RX | `87bbc0ab-e60a-4802-b45f-445ed492cf30` | Notify, peripheral → central |
| INFO | `ac933506-2294-4d92-8a0c-58d9f23acfb3` | Read |

INFO is exactly three bytes: version `01`, then `max_frame:u16 = 0200` (512). It is a protocol ceiling hint, not a substitute for runtime native limits. Advertisements contain no identity, nickname or ANNOUNCE payload. Establish the GATT link, discover the characteristics, enable notifications and determine native directional limits before exchanging HELLO.

HELLO is transport type 02, exactly 54 body bytes and 59 bytes including type and outer header:

| Offset | Bytes | Field |
|---:|---:|---|
| 0 | 1 | Version, 01 |
| 1 | 1 | Local physical role: central 00, peripheral 01 |
| 2 | 2 | Local transmit capacity, 146–512 |
| 4 | 2 | Local receive capacity, 146–512 |
| 6 | 16 | Fresh nonzero local nonce |
| 22 | 32 | Local device Ed25519 public key |

Refuse local capacities below 146 before sending HELLO. Send HELLO only within the known local native transmit limit. The received role must be opposite the local physical role; reject other role/version/capacity values. An identical retransmitted HELLO is harmless; changed fields require a new admission/link. Local nonces come from the CSPRNG and cannot duplicate another live link's nonce; failure to generate one refuses admission. Ordinary traffic starts only after both HELLOs have been exchanged and both effective capacities pass admission. MC-007 supplies handshake time/resource budgets.

HELLO contains unauthenticated claims. It cannot establish a friend pin, fresh presence, or authority to evict an authenticated link. MC-008 must bind both roles, keys, nonces and capacities in its fresh proof before peer identity is trusted. Base transport may carry unverified traffic within ordinary bounds before that proof exists, but must not claim authenticated presence or consolidate duplicates based on the unproved key.

Keep an available sole asymmetric link regardless of a preferred connection direction. When both established links independently prove the same pair of full identity keys, both endpoints retain the lexicographically smaller tuple `(initiator public key, initiator nonce, responder nonce)`. The distinct local nonces distinguish the links. Reject a claimed self-key connection. Bound unverified links separately; an unauthenticated duplicate claim must never evict a working verified link. Physical background-discovery limitations and later proof implementation remain separate gates.

## 4. Logical envelope and type matrix

| Offset | Bytes | Field |
|---:|---:|---|
| 0 | 1 | Version, 01 |
| 1 | 1 | Type |
| 2 | 1 | Flags: bit 0 encrypted, bit 1 signed, bit 2 organizer signature type |
| 3 | 1 | TTL, mutable transport metadata |
| 4 | 8 | Fresh random `msg_id` |
| 12 | 8 | `sender_id` |
| 20 | 4 | `channel_id` |
| 24 | 2 | `payload_len` |

The total is exactly `26 + payload_len`, at most 1,024 bytes. Version is a whole integer, not a nibble split. Senders zero flag bits 3–7; receivers ignore those bits for layout, preserve them in relayed/authenticated bytes, and still enforce the known semantic matrix below. Extra reserved bits never bypass validation, work accounting or authentication.

| Type | Allowed flags masked with 07 | Payload | Scope |
|---|---|---|---|
| 01 CHAT | 00, 01, 02, 06 | Clear CHAT, encrypted envelope, friend-signed CHAT, organizer-signed CHAT respectively | Flood |
| 02 ANNOUNCE | 00, 02 | Clear or friend-signed ANNOUNCE | Direct link |
| 03 SYNC_REQ | 00 | Exactly 520 bytes | Direct link |
| 05 EVENT_INFO | 00 | Root id and name | Flood |
| 06 REACTION | 00, 01 | Exactly 10 clear bytes or encrypted envelope | Flood |
| 07 CRED_REQ | 00 | Exactly 16 bytes | Direct link |
| 08 CRED_OFFER | 00 | Exactly one credential | Flood |

Reject all other known-type flag combinations, including signed+encrypted and organizer-bit without signed-bit. Types 00 and 04 are reserved and reject. Types 09–ff are unknown and flood-only: origins use TTL 7, and receivers apply the same flood clamp, live-ingress rejection and decrement-before-forwarding rules below. Accept only a valid version 1 fixed envelope, capacity/length/TTL and conservative unknown-type budgets; preserve opaque flags, channel and payload without display, history storage or an authentication claim. Payload length still cannot exceed 998. Unknown types cannot introduce direct-link controls within this version's compatibility rule. A different version rejects without interpretation or relay. Encrypted CHAT/REACTION payload layouts remain blocked on MC-008; flags 01 never reinterpret invalid encrypted bytes as cleartext.

Direct controls have TTL 1 and channel 0, never relay, never enter history and reject inside SYNC. Flood origins use TTL 7. Clamp received flood TTL above 7 to 7; reject TTL 0 on live ingress. Forward only after decrement and only while the result remains positive. An otherwise eligible stored CHAT with TTL 0 may process locally but never relay; SYNC cannot refresh its TTL.

EVENT_INFO and CRED_OFFER use #Event Updates (`d91ae76a`); organizer CHAT is restricted to that channel. Clear CHAT/REACTION use their associated channel. The design's other public identifiers are #General `bcf0eae3` and #Confessions `0e9d0974`, derived from the unchanged SHA-256 domain/name rule. Encrypted channel tags are MC-008's extension. No control packet escapes ingress accounting.

## 5. Clear payloads, cosmetics and signatures

Lengths below locate every field; there are no trailing extensions. Unknown avatar/reaction palette values follow the design's generic rendering behavior rather than causing an out-of-range index.

The fixed four-byte cosmetic field is `cosmetic_flags:u8 || rgb[3]`. Bit 0 means custom color, bit 1 supporter hint; senders zero other bits and receivers ignore them. Senders set RGB to zero without custom color; receivers ignore RGB when bit 0 is clear. Hints remain self-asserted. A signature may authenticate who supplied a hint, but never proves payment or grants trusted styling. Anonymous Confessions origins use zero cosmetics and a neutral avatar; receivers must not associate anonymous posts with stable profile metadata.

| CHAT payload offset | Bytes | Field |
|---:|---:|---|
| 0 | 4 | Sender timestamp, Unix seconds |
| 4 | 1 | Avatar |
| 5 | 1 | `nick_len = N`, 1–20 |
| 6 | N | Nickname UTF-8 |
| 6+N | 4 | Cosmetics |
| 10+N | 2 | `text_len = T`, 1–280 |
| 12+N | T | Text UTF-8 |
| 12+N+T | conditional | No tail for unsigned; friend block for flags 02; pin fields then organizer block for flags 06 |

Organizer pin fields are `pin_state:u8 || pin_expiry:u32`, exactly five bytes after text. State 0 requires expiry 0; state 1 requires a positive expiry; other states reject. A pin can affect display only under a verified staff credential and adopted root, while pin, credential and root are unexpired. Senders must keep pin expiry within both credential `not_after` and root expiry. A receiver suppresses an expired or out-of-policy pin even if the text's signature otherwise verifies; pin policy does not turn valid signed text into an unsigned message. Missing authority never permits a pin. These raw fields must be signed. Other CHAT forms have no pin bytes, and no separate pin-update packet exists: each organizer post states its own pin claim.

| ANNOUNCE payload offset | Bytes | Field |
|---:|---:|---|
| 0 | 4 | Timestamp |
| 4 | 1 | Avatar |
| 5 | 1 | Peer count, advisory |
| 6 | 1 | Status: bits 0–1 battery tier, bit 2 infra; remaining bits sent zero/ignored |
| 7 | 1 | `nick_len = N`, 1–20 |
| 8 | N | Nickname UTF-8 |
| 8+N | 4 | Cosmetics |
| 12+N | 2 | `digest_len = D`, either 0 or 256 |
| 14+N | D | Digest |
| 14+N+D | conditional | Friend signature block iff flags 02; public key always included |

The supporter hint is solely in cosmetics, not status bit 3. Retain the design's digest double-hash/bit-order definition; MC-007 owns age/budget tuning. Signed ANNOUNCE authenticates historical content, not fresh direct presence without MC-008's proof.

| Other payload | Exact concatenation |
|---|---|
| EVENT_INFO | `root_id[8] || name_len:u8 || name[name_len]`, name length 1–32 UTF-8 |
| REACTION | `target_msg_id[8] || action:u8 || code:u8`; action bit 0 removes, others sent zero/ignored |
| CRED_REQ | `event_root_id[8] || staff_key_id[8]` |
| CRED_OFFER | One complete credential below, with no additional tail |

Friend signature block: `key_id[8] || included:u8 || public_key[32 if included=1] || signature[64]`, totaling 73 or 105 bytes. `included` must be 0 or 1. The key's SHA-256 prefix must equal both block key_id and logical sender_id. Include the key on the first signed message sent per link and every signed ANNOUNCE; relays preserve the original bytes. Resolve omitted keys only from pins or a bounded cache, without silently choosing a trusted identity from an ambiguous short id. Missing keys remain bounded pending/unverified until an included-key message can resolve them; no unspecified key-request opcode exists. Invalid variants cannot suppress a later valid message with the same msg_id.

Credential: `version:u8 || root_id[8] || staff_public_key[32] || not_before:u32 || not_after:u32 || label_len:u8 || label[label_len] || root_signature[64]`. Version 1, label length 0–16, valid UTF-8, not_before<=not_after. Offsets are 0, 1, 9, 41, 45, 49, 50 and 50+label_len; total 114+label_len. Existing root adoption, expiry and staff-key binding checks remain mandatory.

Organizer signature block: `root_id[8] || staff_key_id[8] || included:u8 || credential_len:u16 || credential[credential_len] || staff_signature[64]`. Offsets 0, 8, 16, 17, 19 and 19+credential_len. Included 0 requires length 0; included 1 requires exactly one 114–130-byte credential. Other values reject. The credential root must match the block root; its staff key hash must match staff_key_id. Organizer verification establishes staff authority. It binds the header's separate device sender_id as a claim, not as proof of ownership of that ordinary device key; never equate this with friend/device identity verification. MC-008 reviews this interaction before MC-021.

Every final message signature must cover immutable header bytes 0–2 and 4–25 (TTL excluded) and every payload byte before the final signature, including length fields, cosmetics, pin fields, signature metadata and any embedded credential. The credential root signature must cover all credential bytes before its signature, including label_len. The root-adoption signature covers bundle bytes 0–36. These coverage requirements supersede earlier partial transcripts. **MC-008 still must freeze exact domain bytes/encoding and review the construction; this ticket does not authorize a signing implementation from incomplete historical transcripts.**

## 6. SYNC request, pages and completion

SYNC_REQ payload is `session_id:u16 || item_count:u16 || cursor:u32 || bloom[512]`: offsets 0, 2, 4, 8, exactly 520 bytes, logical total 546. A requester allocates a session ID before sending its initial cursor 0 request. Do not reuse session IDs within a direction of a link lifetime; reconnect before u16 exhaustion. Responses echo this ID, so delayed responses cannot bind to a newer request. Both directions may walk independently, with at most one active walk per direction.

Retain the design's exact 4096-bit, six-position SHA-256 double-hash Bloom definition and `meshfest-bloom-v1` salt. `item_count` and the Bloom remain unchanged during continuation of one walk. Bloom false positives remain deterministic and are not repaired merely by retrying the same request.

SYNC_ITEM, transport type 01, has these body fields:

| Offset | Bytes | Field |
|---:|---:|---|
| 0 | 2 | Echoed session_id |
| 2 | 2 | Sequence number |
| 4 | 1 | Flags below |
| 5 | 4 | next_cursor |
| 9 | 2 | blob_len |
| 11 | blob_len | Exact stored logical packet, or no bytes for a marker |

| Flags | Meaning | next_cursor | blob_len |
|---|---|---|---|
| 00 | Data item | 0 | 26–1,024 |
| 03 | Page ended; more available in the same session | Nonzero | 0 |
| 05 | Page and session complete; no more eligible items | 0 | 0 |
| 07 | Page and session complete; budget ended with more available | 0 | 0 |

Every other flag pattern rejects. Bit 0 marks page end, bit 1 more available, bit 2 session completion. Every page ends with exactly one marker; data packets never substitute for a marker. An empty response is a marker at sequence 0. A marker's zero-length blob is not a zero-length logical packet. Current history policy admits CHAT only; controls, reactions and unknown types reject as embedded history. Apply full ordinary validation and crypto budgets to each embedded packet, not just its transport wrapper. Never refresh the stored TTL.

Sequence starts at 0 and increments for every data item and marker across pages, without wrapping. Bind responses to the active request ID and direction. Ignore identical duplicates; conflicting duplicates abort the session. A bounded gap buffer may hold out-of-order items until missing sequences arrive. Do not consume a page-end marker, issue its continuation or claim completion until all previous sequences have been processed. MC-007 fixes the gap-buffer ceiling and deadlines. Loss may end a session by timeout; missing items must not become a success claim.

The responder walks a newest-first snapshot, skipping Bloom matches. Later inserts belong to a later walk; evicted entries in the snapshot may be skipped. Nonzero cursors are opaque, single-use tokens bound to this link, direction, session, original filter and snapshot position. They are not trusted raw cache offsets. Continuation repeats session_id, item_count and Bloom with the returned cursor. Reject stale, reused, unknown or altered-filter continuations. Flags 03 permits immediate continuation within the existing admitted budget; flags 07 requires a later new session with cursor 0 and new admission. MC-007 owns numerical page, session, byte, crypto, selection and time budgets before implementation.

## 7. QR and link grammar

Binary segments use the [RFC 4648 base64url alphabet](https://www.rfc-editor.org/rfc/rfc4648.html#section-5), without `=` padding. Require zero unused pad bits and canonical decode/re-encode equality. Reject whitespace, nonalphabet characters, percent escapes and alternate alphabets in binary segments. These are explicit application-level strictness rules.

Display segments encode UTF-8 bytes using [RFC 3986 percent encoding](https://www.rfc-editor.org/rfc/rfc3986.html#section-2.1): generators leave only unreserved ASCII literal and use uppercase hexadecimal escapes. Receivers may accept either hex case, decode exactly once after splitting path segments, then validate text; decoded separators never become new path boundaries. Raw non-ASCII or reserved characters must be escaped. Reject invalid UTF-8, controls and design-forbidden display characters. Nicknames have 1–20 decoded bytes; event names 1–32. Neither cosmetic name is part of the binary key/root signature, so confirmation displays it as a claim.

| Purpose | Custom-scheme form | Binary content |
|---|---|---|
| Channel | `meshfest://j/<descriptor>-<genre>-<location>` | No binary segment |
| Friend | `meshfest://friend/<bundle>/<nickname>` | Version 01 + Ed25519 public key (32 bytes) + X25519 public key (32 bytes) = 65 bytes |
| Event | `meshfest://event/<bundle>/<name>` | Version 01 + root public key (32 bytes) + root expiry (4 bytes) + self-signature (64 bytes) = 101 bytes |
| Staff | `meshfest://staff/<credential>/<seed>` | Credential 114–130 bytes above; separate Ed25519 seed (32 bytes) |

Scheme and host matching is ASCII case-insensitive, including the custom authority names j/friend/event/staff. HTTPS path route names are case-sensitive lowercase. HTTPS equivalents use exactly host `meshfest.app` with `/j/`, `/friend/` or `/event/` paths. Staff seed provisioning is custom-scheme/offline only, never an HTTPS share URL. Reject userinfo, explicit ports, queries, fragments, extra/missing/empty path segments and URIs above 2,048 ASCII bytes. No domain ownership or deployment result is claimed here; those remain release prerequisites.

Channel words are validated against the three positional design lists, case-insensitively, then generated as lowercase ASCII with hyphen separators. Do not hash arbitrary unknown words or accept a different segment layout. The displayed hyphens differ intentionally from the pipe separators in channel hash input.

Friend bundles of another version/length reject; MC-008 owns key validation, Ed/X binding and trust transitions. Derive event_root_id from the root public key; it is not a separate URL field. Verify the root self-signature/expiry and require explicit adoption confirmation. For staff import, derive the public key from the 32-byte seed and match the credential; require the applicable adopted root and valid credential before enabling staff posting. Root private keys never enter QR provisioning. Do not retain private provisioning images, seed-bearing URLs or private samples in logs, source or reports. All joins, pins, adoption and imports require the design's explicit confirmation flow.

## 8. Compatibility, size proof and evidence

This is a pre-freeze correction to the unimplemented draft v1 grammar, not a claim of compatibility with hypothetical earlier draft encoders. Concrete production UUIDs distinguish it from feasibility probes. Later incompatible changes require an explicit versioned decision. There is no decoder fallback to older omitted-cosmetic/pin/session layouts.

| Form | Logical/body bytes at maximum | Frames at C=146 | Frames at C=182 |
|---|---:|---:|---:|
| Unsigned CHAT |338|3|3|
| Friend CHAT, key omitted |411|4|3|
| Friend CHAT, key included |443|4|3|
| Organizer CHAT, credential omitted |426|4|3|
| Organizer CHAT, credential included |556|5|4|
| Unsigned ANNOUNCE |316|3|2|
| Signed ANNOUNCE, key included |421|4|3|
| SYNC_REQ |546|5|4|
| EVENT_INFO |67|1|1|
| Clear REACTION |36|1|1|
| CRED_REQ |42|1|1|
| CRED_OFFER |156|2|1|
| Absolute logical ceiling |1024|8|7|
| SYNC_ITEM with maximum logical blob |1035|8|7|
| Absolute transport-body ceiling |1200|10|8|
| SYNC marker |11|1|1|
| HELLO |54|1|1|

Whole logical overhead is 4; whole transport overhead 5; logical fragmentation overhead 18 per value; transport fragmentation overhead 16 per value. Counts prefer whole form when it fits. The worksheet proves fit under protocol limits, not throughput or battery acceptance. MC-008 must fit its final encrypted plaintext/padding/envelope into 1,024 bytes or specify an explicit sender refusal. MC-007 must account for actual overhead and simultaneous traffic. No plaintext or oversize fallback is allowed.

The [companion vector conventions](../../tests/vectors/README.md) supply structural examples and mutation expectations. Four example frames have checked lengths and two fragments reconstruct the reaction example. These arithmetic checks are distinct from future codec tests, native interoperability and independent security review. Review/PR and final validation results belong in the ticket and PR as implementation and review proceed.
