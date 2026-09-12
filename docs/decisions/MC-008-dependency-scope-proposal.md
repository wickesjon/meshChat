# MC-008 — HPKE dependency compatibility proposal

Status: researched proposal, awaiting an explicit dependency-gate scope decision. This document does not select a production dependency or change the normative crypto contract. MC-008 remains inprogress; no DM implementation or security-review pass is claimed.

## Finding and recommendation

The existing `src/core/deny.toml` denies multiple versions of any crate. An isolated dependency-resolution experiment combined the current core's `security-probe` and `bindgen` features with three published HPKE releases while preserving the main lockfile's existing pins. Every candidate violates that rule. These are actual dependency-gate failures, not unavailable hosted runners.

Recommend RFC 9180 Auth mode with a fixed X25519/HKDF-SHA256/ChaCha20Poly1305 suite and `hpke = "=0.14.1"` as the baseline for the remaining MC-008 design work, subject to approval of the exact duplicate-version exceptions below. This is preferable to selecting an older release solely to reduce the number of duplicates, or upgrading the existing platform probe's dependency graph as an incidental change. Full application transcripts, replay/freshness rules, platform integration and independent assessment still need their owning work.

The published 0.14.1 package identifies source revision `b83b0011030b55ac74f389c112db784274d4d667`. Its [changelog](https://github.com/rozbb/rust-hpke/blob/b83b0011030b55ac74f389c112db784274d4d667/CHANGELOG.md) dates 0.14.1 to 2026-09-05 and reports improved KEM zeroization. The manifest declares Rust 1.85 compatibility; the local experiment below additionally checked compilation with Rust 1.85.1. This is host evidence, not Android/iOS build or device evidence.

## Reproduced comparison

| Candidate and selected features (defaults off) | Dependency-gate result |
|---|---|
| hpke 0.12.0: alloc, x25519 | Duplicate getrandom 0.2.17/0.4.3 |
| hpke 0.13.0: alloc, x25519 | Duplicate getrandom 0.2.17/0.4.3 and rand_core 0.6.4/0.9.5 |
| hpke 0.14.1: alloc, x25519, chacha, getrandom | Nine duplicate crate/version pairs below; host cargo check passes |

All three resolved graphs passed cargo-deny advisory, license and source checks; their bans checks failed. The disposable experiment initially omitted the version on its local path dependency and also failed the wildcard rule; that experiment-only declaration was corrected to `version = "=0.1.0"` before retaining these results. No exception for wildcard dependencies is proposed.

Use of 0.14.1 does not imply current independent audit coverage. Its [published README](https://github.com/rozbb/rust-hpke/blob/b83b0011030b55ac74f389c112db784274d4d667/README.md) reports Cloudflare's internal review of 0.8 and no known paid audit. [Cloudflare's first-party account](https://blog.cloudflare.com/using-hpke-to-encrypt-request-payloads/) is historical evidence, not an assessment of meshChat or 0.14.1. MC-022 must still require independent construction/transcript review, and MC-037 the integrated assessment.

An alternative implementation, [hpke-rs](https://docs.rs/hpke-rs/0.7.0/hpke_rs/struct.Hpke.html), exposes authenticated modes through configurable backends but was not built or dependency-gated in this experiment. Do not infer it solves this graph conflict. [RUSTSEC-2026-0070](https://rustsec.org/advisories/RUSTSEC-2026-0070.html) is an export-only-context panic affecting versions through 0.5.0 and patched in 0.6.0; it is not evidence that 0.7.0 is affected. The custom two-DH analogue has no selected-library implementation evidence and remains blocked; this proposal does not authorize falling back to it.

## Requested exception

Authorize only coexistence of these resolved version pairs for the selected HPKE 0.14.1 graph:

| Crate | Existing core version | HPKE graph version |
|---|---|---|
| block-buffer |0.10.4|0.12.1|
| cpufeatures |0.2.17|0.3.1|
| crypto-common |0.1.7|0.2.2|
| curve25519-dalek |4.1.3|5.0.0|
| digest |0.10.7|0.11.3|
| fiat-crypto |0.2.9|0.3.0|
| rand_core |0.6.4|0.10.1|
| sha2 |0.10.9|0.11.0|
| x25519-dalek |2.0.1|3.0.0|

Apply exact-version cargo-deny exceptions only when adding that dependency in the owning implementation ticket (MC-020, with MC-017 provider integration as needed). Keep `multiple-versions = "deny"` for everything else, no wildcard/subtree skips, and preserve advisory/license/source gates. Review dependency upgrades and removal/consolidation of these exceptions explicitly; this approval would not permit arbitrary future versions or a broad dependency refresh. Re-resolve against the then-current main lockfile and obtain a new scope decision if different pairs are needed.

Tradeoff: coexistence adds dependency/code surface and requires keeping both families under maintenance and review. Rust types from separate major versions are distinct: transport only the specified fixed public byte encodings across their boundary, with validation on each side; never cast key/context types or expose private keys to work around API incompatibility. An alternative is a separate coordinated upgrade of the existing core/platform probe dependencies, requiring its own scope and all affected platform checks. Neither path may waive independent security or physical verification.

## Reproduction and retained evidence

Base: main `edb3201b6f4b498ca521bcbe02b7406f875aa228`. Host: Windows, Rust/cargo 1.85.1 and cargo-deny 0.20.2. RustSec database revision at the retained check: `b50980aad8b8f14f77e25a97b32dd94bf008b0af`. The crates.io checksum for hpke 0.14.1 in the resolved lockfile is `a5324110b02044183df000f0bd7d2d7e61f1000631508627e77f63983b6fdffb`. Each experiment is an ignored repository-local directory with the following manifest (substitute the candidate version/features from the comparison), an empty `lib.rs`, and an initial copy of the base `Cargo.lock`:

```toml
[package]
name = "mc008-dependency-check"
version = "0.0.0"
edition = "2021"
rust-version = "1.85.1"
publish = false
[workspace]
[dependencies]
meshchat-core = { version = "=0.1.0", path = "../../src/core", features = ["security-probe", "bindgen"] }
hpke = { version = "=0.14.1", default-features = false, features = ["alloc", "x25519", "chacha", "getrandom"] }
[lib]
path = "lib.rs"
```

With repo-local Cargo/Rustup/temp paths configured, run `cargo tree --manifest-path <experiment>/Cargo.toml --duplicates` to resolve only the additions, then `cargo-deny --manifest-path <experiment>/Cargo.toml --all-features --locked --config src/core/deny.toml check`. For 0.14.1, `cargo check --manifest-path .work/mc008-dependency-014/Cargo.toml --locked` passes with `CARGO_TARGET_DIR` set to that experiment's `target/` directory.

Retained local logs: `.work/mc008-dependency-check/deny-0.13.0.log`, `.work/mc008-dependency-012/{tree,deny}.log`, and `.work/mc008-dependency-014/{tree,deny,check}.log`. The 0.14.1 experiment lockfile SHA-256 is `2e8c77eabca6534455cfa40297adeb477f353c24667440069357790b12c9ef0a`. Future online resolution may differ; retain the exact graph with implementation evidence and rerun current advisories at integration. Repository production manifests, lockfile and deny configuration remain unchanged.
