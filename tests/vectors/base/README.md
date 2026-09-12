# Logical packet vectors

MC-009 implements the structural portions of the canonical MC-006 wire contract and MC-008 profile01. `logical.txt` is a byte-exact text corpus with columns `id context result hex`; `-` means an empty byte string. `live` and `stored` select live ingress or a stored CHAT inside SYNC. Results are `ok` or the codec error category. Every case is synthetic and has **no authenticated identity, authority, dedup, UI or storage effect**. Included public-key bytes and zero signatures are deliberately structural fixtures, not valid cryptographic vectors. Link identity and directional capacity are outer-frame concerns belonging to MC-010; these inputs begin at the logical header.

`generate.py` independently assembles the documented fields with Python's standard-library big-endian encoders. Its default mode checks the committed bytes; `--write` regenerates them and requires reviewing the resulting diff. Rust tests consume the committed bytes without calling Python. A separately hardcoded encoder input and the MC-006 reaction literal check header offsets independently of decoder/encoder round trips. The corpus includes each supported form, maximum sizes, all disallowed semantic flags (also with high reserved bits), malformed UTF-8, control scope, credentials and encrypted-profile boundaries. Tests additionally cover every truncation, length/version mutations, every unknown type/flag pair, reserved-bit preservation and TTL clamp/decrement.

The parser allocates no heap memory: it checks the 1,024-byte ceiling before reading fields and returns borrowed slices/strings with fixed-size metadata. Serialization and forwarding use a caller-supplied bounded output slice. Original received TTL, reserved flags, raw text and signature inputs remain available. `serialize` writes a wire representation; it is not an origin-policy or signing API. A returned error means the caller must discard its output buffer. Only forwarding changes TTL, and it never mutates the input packet.

Structural success is intentionally distinct from admission, display and authentication. MC-011 supplies display sanitization; MC-013/014 supply rate/budget/dedup decisions; MC-019/020/021 verify key hashes, strict keys/signatures, DH/AEAD, time, pins and authority. This codec checks cheap equalities (friend block ID against header ID and embedded credential root against organizer root), but does not perform cryptographic hashing or trust lookup. A malformed signature or mismatched public-key hash must never be treated as authenticated merely because this parser returned a view. Canonical X25519 enc is checked before future DH; all-zero DH rejection remains mandatory at authentication. Unknown payload views carry no display/history/trust interpretation.

Run from the repository root:

```
python -B tests/vectors/base/generate.py
cargo test --test codec --test codec_fuzz --locked
```

These are host structural tests, not native interoperability, real message authentication or independent security assessment evidence.
