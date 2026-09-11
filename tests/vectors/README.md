# Canonical vector conventions

The normative [MC-006 contract](../../docs/decisions/MC-006-wire-contract.md) specifies framing. MC-009/010 implement the codec/reassembler and MC-008/020–022 supply the final cryptographic fixtures. This README does not claim any implementation passes.

Use lowercase hexadecimal pairs, no endian reversal, with whitespace permitted only as fixture presentation. JSON fixtures must carry a unique id, the contract revision, input bytes, ingress context (live/SYNC, directional capacity and link identity), expected structural result, expected authentication state, and expected side effects separately. Structural success never implies signature validity. Cryptographic fixtures must identify the selected library/reference source and reproducible public synthetic inputs; no production private material, provisioning QR or user message content belongs here.

Keep encoder and decoder expectations distinct. Encoder canonicalizes whole-vs-fragment choice; decoder accepts any valid nonempty partition with the correct exact total. Future compatibility cases explicitly state the raw reserved bits to preserve through relay/authentication.

## Structural fixtures

Empty successful SYNC completion, session 1, sequence 0:

`02 00 000c 01 0001 0000 05 00000000 0000`

This is 16 bytes: four frame-header bytes, one object-type byte and an 11-byte body. It is valid only for an outstanding matching request and expected sequence. A zero-length blob is a terminal marker, never an empty logical packet. Changing body_len to `000b`/`000d`, flags to `01`/`02`, or flags to `03` with cursor 0 must reject. A nonzero blob length without corresponding bytes rejects. A different session ID is ignored as unrelated or stale.

Whole unsigned reaction, sender fixture `1112131415161718`, message id `0102030405060708`, target `2122232425262728`, #General, add palette code 0:

`00 00 0024 01 06 00 07 0102030405060708 1112131415161718 bcf0eae3 000a 2122232425262728 00 00`

This is 40 bytes total, with one 36-byte logical packet. Synthetic identifiers are not real identities or trust evidence. Receiving an unsigned reaction does not authenticate the sender. Mutation expectations:

- Logical payload_len `0009`/`000b` rejects because the exact body is ten bytes.
- Reserved type `04`, known REACTION flags `02`, a different version, outer flags `01`, or a trailing byte rejects.
- Logical flags `80` remains unsigned, with the raw reserved bit preserved.
- Live TTL 0 rejects; TTL `ff` clamps to 7 before the ordinary forwarding rule.
- This REACTION inside SYNC rejects because only CHAT is history-eligible. Stored TTL 0 local-only behavior requires a separate eligible CHAT fixture.

Same reaction split into two legal small fragments at capacity 146, grouping `0001`:

`01 00 0020 0102030405060708 0001 00 02 0024 010600070102030405060708111213141516`

`01 00 0020 0102030405060708 0001 01 02 0024 1718bcf0eae3000a21222324252627280000`

Each value is 36 bytes: four outer-header bytes, 14 envelope bytes and an 18-byte slice. A decoder accepts either arrival order; an encoder emits whole form at this capacity. An identical duplicate has no effect. Different bytes for an already accepted fragment index abort only the scoped group; a later valid whole packet with this msg_id remains eligible. A different arrival link has independent group state. Count 0/9 rejects; conflicting count/total metadata across the group rejects; nonzero outer flags rejects; an inner msg_id mismatch rejects at completion. None of these failures poisons accepted dedup. Late fragments cannot extend the original group's deadline.

## Size worksheet

The floor C=146 gives 128-byte logical slices and 130-byte transport slices. Example C=182 gives 164 and 166 bytes respectively. A whole logical frame has four bytes of overhead; whole transport adds a type byte. Fragmentation overhead is 18 bytes per logical fragment and 16 per transport fragment. Counts below prefer whole form when it fits.

| Form | Logical/body bytes | Frames at 146 | Frames at 182 |
|---|---:|---:|---:|
| Max unsigned CHAT |338|3|3|
| Max friend CHAT, omitted key |411|4|3|
| Max friend CHAT, included key |443|4|3|
| Max organizer CHAT, omitted credential |426|4|3|
| Max organizer CHAT, included credential |556|5|4|
| Max unsigned ANNOUNCE |316|3|2|
| Max signed ANNOUNCE (key included) |421|4|3|
| SYNC_REQ with explicit session id |546|5|4|
| Max EVENT_INFO |67|1|1|
| Clear REACTION |36|1|1|
| CRED_REQ |42|1|1|
| Max CRED_OFFER |156|2|1|
| Maximum logical packet, including future crypto |1024|8|7|
| SYNC_ITEM with maximum logical blob |1035|8|7|
| Maximum transport body |1200|10|8|
| SYNC terminal marker |11|1|1|
| HELLO body |54|1|1|

All logical forms fit the 1,024-byte/eight-fragment bounds at the floor; all transport forms fit the 1,200-byte/sixteen-fragment bounds. A capacity below 146 refuses link admission. A payload exceeding its known type's tighter bound rejects even below the global cap. MC-008 must calculate its selected envelope/padding within 1,024 bytes or add an explicit sender refusal; there is no plaintext fallback.

Reproduce the structural/count check with Python 3 from the repository root. This reads this README and writes no files; an optional saved copy belongs in ignored `.work/`.

```python
from pathlib import Path
from math import ceil
import re

text = Path('tests/vectors/README.md').read_text(encoding='utf-8')
frames = [bytes.fromhex(line.strip('`')) for line in text.splitlines()
          if line.startswith('`') and re.fullmatch('[0-9a-f ]+', line.strip('`'))]
assert len(frames) == 4
assert all(len(f) == 4 + int.from_bytes(f[2:4], 'big') for f in frames)
assert frames[2][18:] + frames[3][18:] == frames[1][4:]
rows = [line for line in text.splitlines() if re.match(r'\|[^|]+\|[0-9]+\|', line)]
assert len(rows) == 17
for index, row in enumerate(rows):
    size, at146, at182 = map(int, row.split('|')[2:5])
    logical = index < 13
    assert size <= (1024 if logical else 1200)
    for capacity, expected in [(146, at146), (182, at182)]:
        count = (1 if size + (4 if logical else 5) <= capacity else
                 ceil(size / (capacity - (18 if logical else 16))))
        assert count == expected and count <= (8 if logical else 16)
assert 4 + 14 + ceil(1024 / 8) == 146
print('Four frames, reassembly, 17 size rows and admission arithmetic pass')
```

## Negative corpus requirements

The implementation corpus must cover:

- Every known type and all eight semantic flag patterns, with zero and nonzero reserved high bits; unknown/reserved type and version distinctions.
- Every truncated field boundary, exact lengths and trailing bytes; UTF-8/sanitizer bounds; each boolean/enum; payload and directional-capacity extrema.
- Cross-link, out-of-order, duplicate, conflicting and timeout events; SYNC gaps, late responses, stale cursors, empty pages and budget completion.
- QR malformed alphabets, pad bits, repeated decoding, extra URL components, invalid bundles and credential lengths.
- Organizer pin and signed-metadata mutations after MC-008 defines exact domains.

Keep expected parse, crypto, pending/unverified, accepted-dedup, relay and UI outcomes separate. Never label a deliberately invalid zero-signature structural fixture as authenticated. Golden-vector consumers across Rust/Kotlin/Swift must compare exact bytes; calling the same shared core from three languages is FFI parity, not independent crypto interoperability.
