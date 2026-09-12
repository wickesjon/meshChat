# Crypto vectors and MC-022 evidence

The normative [MC-008 contract](../../../docs/decisions/MC-008-crypto-contract.md) is specified, not an implemented cryptographic system. This file contains executable **transcript/size fixtures only** plus requirements for implementation vectors. No ciphertext, private key, signature verification, native parity or independent assessment is claimed by running the fixture below.

## Reference sources

- [RFC 9180 Appendix A.2.3](https://www.rfc-editor.org/rfc/rfc9180.html#appendix-A.2.3): Auth mode02, KEM0020, KDF0001, AEAD0003. MC-020 must reproduce the published encapsulation/context values and ciphertext/tag for sequence0, then compare the selected library with a separately maintained RFC-compatible implementation. Do not use two bindings into the same Rust core as the independent implementation.
- [CFRG HPKE test-vector repository](https://github.com/cfrg/draft-irtf-cfrg-hpke): retain the exact commit and source-file SHA-256 when importing permitted public fixtures. The selected crate also ships `test-vectors/origrfc-5f503c5.json`; inspect provenance and suite/mode before use. A moving branch is a discovery link, not a pinned evidence record.
- [RFC 8032 §7.1](https://www.rfc-editor.org/rfc/rfc8032.html#section-7.1) and [RFC 7748 §6.1](https://www.rfc-editor.org/rfc/rfc7748.html#section-6.1): Ed25519 and X25519 reference inputs. The public keys below are the first two Ed25519 vectors and Alice/Bob X25519 public values respectively. They have not been established as meshChat friend identities.
- [Pinned HPKE source](https://github.com/rozbb/rust-hpke/tree/b83b0011030b55ac74f389c112db784274d4d667): library0.14.1 source and RNG/zeroization behavior. Record the actual resolved graph and current advisory checks at integration; historical audit evidence does not satisfy MC-022.

## Canonical public transcript fixture

The header is encrypted CHAT with profile01, msg_id0102030405060708, derived sender hints, arbitrary channel bytes deadbeef and final payload length121. `enc` is canonical u=9. This **does not** provide a matching private key, valid recipient tag or ciphertext. It exercises unambiguous encoding before crypto. HELLOs use roles00/01, TX/RX146, nonces11×16/22×16 and the two public Ed keys. Both are synthetic, not captured radio sessions.

| Transcript | Bytes | Expected SHA-256 |
|---|---:|---|
|HPKE info|155|`011477f1305df9450173001a7e57c4c4889dfc840c4dd3ac58b38255b4afde55`|
|HPKE AAD|85|`0f8575a1adcbc3b009b3ff89868aebbcfc47d9f2ae006af5994057c3b1a7a115`|
|Peripheral LINK_PROOF input|132|`694d3234668eb14aeffc3de387135078a540398d33e45393ff20301457933e91`|

Copy the following standard-library Python block into ignored `.work/mc008-readme-check.py`, then run `python -B .work/mc008-readme-check.py` from the repository root. Its expected hashes and domain hex are fixed independently of runtime encoding. It also checks each immutable header byte and that TTL is the sole excluded byte.

```python
import hashlib

D = lambda label: ('meshfest/' + label + '/v1').encode('ascii') + b'\x00'
domains = {
    'dm-info': '6d657368666573742f646d2d696e666f2f763100',
    'dm-aad': '6d657368666573742f646d2d6161642f763100',
    'friend-sign': '6d657368666573742f667269656e642d7369676e2f763100',
    'staff-sign': '6d657368666573742f73746166662d7369676e2f763100',
    'credential': '6d657368666573742f63726564656e7469616c2f763100',
    'event-root': '6d657368666573742f6576656e742d726f6f742f763100',
    'link-proof': '6d657368666573742f6c696e6b2d70726f6f662f763100',
}
assert len(set(domains.values())) == len(domains)
for label, expected in domains.items():
    assert D(label).hex() == expected
ed_s = bytes.fromhex('d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a')
ed_r = bytes.fromhex('3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c')
x_s = bytes.fromhex('8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a')
x_r = bytes.fromhex('de9edb7d7b7dc1b4d35b61c2ece435373f8343c85b78674dadfc7e146f882b4f')
assert hashlib.sha256(ed_s).digest()[:8].hex() == '21fe31dfa154a261'
assert hashlib.sha256(x_s).digest()[:8].hex() == '300c9c9603b92a4b'
h = bytes.fromhex('01010107010203040506070821fe31dfa154a261deadbeef0079')
e = bytes.fromhex('01300c9c9603b92a4b09') + bytes(31)
assert len(h) == 26 and len(e) == 41
info = D('dm-info') + bytes.fromhex('02002000010003') + ed_s + x_s + ed_r + x_r
aad = D('dm-aad') + h[:3] + h[4:] + e
hc = bytes.fromhex('010000920092') + b'\x11'*16 + ed_s
hp = bytes.fromhex('010100920092') + b'\x22'*16 + ed_r
proof = D('link-proof') + b'\x01' + hc + hp
for value, length, digest in [
    (info, 155, '011477f1305df9450173001a7e57c4c4889dfc840c4dd3ac58b38255b4afde55'),
    (aad, 85, '0f8575a1adcbc3b009b3ff89868aebbcfc47d9f2ae006af5994057c3b1a7a115'),
    (proof, 132, '694d3234668eb14aeffc3de387135078a540398d33e45393ff20301457933e91'),
]:
    assert len(value) == length
    assert hashlib.sha256(value).hexdigest() == digest
for i in range(26):
    changed = bytearray(h)
    changed[i] ^= 1
    changed_aad = D('dm-aad') + changed[:3] + changed[4:] + e
    assert (changed_aad == aad) == (i == 3)
for text_len, expected_p, expected_zeroes in [
    (1,64,57), (58,64,0), (59,144,79),
    (138,144,0), (139,304,159), (280,304,18),
]:
    p = next(size for size in (64,144,304) if 6+text_len <= size)
    assert (p, p-6-text_len) == (expected_p, expected_zeroes)
    assert p+57 in (121,201,361) and p+83 in (147,227,387)
assert 4+1+66 == 71 <= 146
assert (387+127)//128 == 4 and (147+127)//128 == 2
assert 1+1+2+2+1 == 7 and 1+3+1+2+1 == 8
print('MC-008 transcript/size fixtures pass; no cryptographic or device result claimed.')
```

## Required implementation and independent-review cases

Each case must retain full public wire inputs, synthetic test-key provenance, exact expected result/identity/effect, selected library version, runner revision and reproducible command. Keep real private keys/message content/provisioning images out of the repository. MC-020/021 own executable fixtures and negative runners, MC-022 their independent review; this table is a test plan, not passing results.

| Case group | Required expectation |
|---|---|
|RFC Auth positive|Exact AppendixA.2.3 sequence0 enc/context/ciphertext/tag; independent implementation agrees|
|Profile positives|CHAT text lengths1,58,59,138,139,280 and multibyte UTF-8 byte boundaries; REACTION add/remove; both role directions; all stated payload/fragment sizes|
|Immutable binding|Mutate each non-TTL header byte, envelope profile/hint/enc, tag, each info tuple/role, length and ciphertext byte: no authenticated effect; TTL-only modification retains crypto validity but obeys live/SYNC TTL policy|
|Sender forgery|Attacker knowing all public keys/tags but neither sender nor recipient private key cannot authenticate as pinned sender; Base/PSK/AuthPSK or wrong suite/domain cannot pass as profile01|
|Compromise limits|Demonstrate recipient-key KCI/recipient-fabricated transcript as an expected limitation, not an impossible negative test; UI/security report states no forward secrecy|
|Key validation|31/33-byte X/Ed, noncanonical X including high-bit/reduced aliases, low-order/all-zero DH at every call, Ed weak keys/noncanonical R/S and wrong key IDs: reject without fallback|
|Padding/parser|Unknown profile, forbidden lengths, short tag, too-large allocation, invalid text length/UTF-8, nonzero padding, nonminimal bucket and reaction wrong size: no effect; no panic or unbounded work|
|Tags/collisions|Same-hour and ±1 rollover; epoch0/overflow; outside-window no extended search; shared tag with unrelated hints does not trial-decrypt; two tuples matching both hints refuse; unique sender hint with wrong tag does no HPKE work|
|Replay/clock/store|Duplicate/conflicting authenticated records, invalid-before-valid same ID, concurrent arrivals, pruning/restart, transaction failure, full tombstone store, backward/unknown clock and identity reset: no duplicate effect or trust renewal|
|Friend transitions|One-way pin, unknown key, Ed-only/X-only replacement, nickname impersonation, explicit replacement confirmation/cancel, locked/lost provider, remove/re-add and full reset; no silent pin change or downgrade|
|Link proof|Exact both HELLOs/roles/nonces/capacities; reflection/cross-link replay/changed HELLO/deadline/duplicate invalid candidate/nonce reuse; 60s expiry from nonce creation including suspend; ANNOUNCE cannot renew freshness|
|Organizer coverage|Root self-signature, label/credential/pin/cosmetics/inclusion bits/length tampering; missing/ambiguous roots or staff credentials, expired/not-yet-valid credentials, wrong root/staff binding; no friend badge from staff sender_id claim|
|Credential recovery|Valid inclusion/omission, multiple credentials, immediate peer lacking credential, delayed CRED_OFFER after bounded retries/expiry; no unsigned/opaque cache entry grants authority|
|Provider/work|Entropy failure before setup, unexpected buffered RNG consumption, locked/invalidation/reset, scratch cleanup and no seed-export API; instrument actual scalar/DH/tag/AEAD/signature calls including cold caches/failures under MC-007 limits|

A valid reference primitive vector does not prove meshChat replay/pin/presence behavior. Independent assessment must review the actual integration and all byte-domain/role bindings; Kotlin/Swift wrappers calling one Rust implementation only establish binding parity.
