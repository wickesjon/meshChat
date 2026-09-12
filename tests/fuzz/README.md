# Logical codec fuzz smoke

`codec_smoke.rs` contains an arbitrary-byte target used by a deterministic mutation campaign. It parses both live and stored contexts plus standalone credentials, asserts successful round trips preserve exact bytes, and checks forwarding changes only TTL. It uses the persisted MC-009 corpus in `tests/vectors/base/logical.txt`, all seed truncations and single-byte inversions, 100,000 seeded insertion/removal/XOR/truncation mutations (half repairing the declared length to exercise deeper fields), and random lengths 0–1,100. Seed: `0x4d433030395f7631`.

Run `cargo test --test codec_fuzz --locked` and repeat with `--release`. This is reproducible mutation fuzzing, not a coverage-guided/libFuzzer campaign or proof of absence of vulnerabilities. No toolchain/dependency additions are needed. Persist future minimized failures in the base corpus with an explicit expected outcome and add the targeted assertion before fixing them. Existing malformed-length, UTF-8, flag, credential and canonical-enc cases remain regression seeds.

The test harness bounds mutation buffers to 1,100 bytes; production parsing allocates nothing and rejects inputs over 1,024 before payload parsing. Output buffers are fixed 1,024-byte stack arrays. No crypto, native radio, physical device or independent security result is claimed.
