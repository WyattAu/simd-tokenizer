# simd-tokenizer

[![docs.rs](https://docs.rs/simd-tokenizer/badge.svg)](https://docs.rs/simd-tokenizer)
[![crates.io](https://img.shields.io/crates/v/simd-tokenizer.svg)](https://crates.io/crates/simd-tokenizer)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

Safe, SIMD-style (SWAR) whitespace token estimator with an optional exact
tiktoken backend — extracted from
[clawdius](https://github.com/WyattAu/clawdius).

```rust
use simd_tokenizer::{SimdWhitespaceTokenizer, TokenCounter};

let t = SimdWhitespaceTokenizer::new();
let tokens = t.count("The quick brown fox jumps over the lazy dog.");
```

## What's inside

| Type | Description |
| --- | --- |
| `TokenCounter` | Trait abstracting token counting backends (`count`, `backend_name`). |
| `SimdWhitespaceTokenizer` | SWAR byte-scan for whitespace boundaries + BPE-approximate blend. |
| `TokenEstimator` | Enum dispatching to the best available backend. |
| `TiktokenCounter` | Exact `cl100k_base` and `o200k_base` counting (feature `tiktoken`, off by default). |

## The "SIMD" in the name, honestly

clawdius's original scanner used `std::arch` SSE2/NEON intrinsics behind
`unsafe`. This port replaces it with **SWAR** ("SIMD Within A Register"): 8
bytes are loaded per iteration into a `u64` and classified with branch-free,
borrow-free 7-bit arithmetic; word boundaries are counted with `popcount`.
It is fully portable (no `cfg(target_feature)`, no runtime detection) and
lets the whole crate declare `#![forbid(unsafe_code)]` on stable Rust.

Measured on a 1.09 MB English text (`i9-class laptop`, release build):

| path | per call | vs SWAR |
| --- | --- | --- |
| SWAR scan (`count_splits`) | 1.17 ms | — |
| branchy scalar reference | 5.79 ms | 5.0× slower |
| `str::split_whitespace().count()` | 11.1 ms | 9.5× slower |

So: several times faster than the naive paths, ~1 GB/s, but **not** as fast
as hand-written SSE2/AVX2 for huge buffers (SWAR moves 8 bytes per step vs
16-32 for vector ISAs). For token *estimation* granularity — the result
feeds a budgeting heuristic, not an invoice — this trade is usually
invisible end-to-end.

## Estimate semantics

Splits on any byte ≤ `0x20` (space, tab, newline, CR, other C0 controls),
then blends the word count with a ~4 chars/token heuristic and a punctuation
overhead term (`estimate_from_whitespace_splits`). It approximates BPE
behavior for budgeting and rate-limiting decisions; it is **not** a
tokenizer table.

## Features

- `tiktoken` (default **off**, keeps the dependency tree light) — adds
  `TiktokenCounter` for exact `cl100k_base` (`TiktokenCounter::new`) and
  `o200k_base` (`TiktokenCounter::o200k`) counts and makes
  `TokenEstimator::new` prefer tiktoken. Gated off on `wasm32` like clawdius.

```toml
simd-tokenizer = "0.1"
# or with exact counting:
simd-tokenizer = { version = "0.1", features = ["tiktoken"] }
```

```rust
use simd_tokenizer::{TiktokenCounter, TokenCounter};

let t = TiktokenCounter::o200k().expect("o200k_base loads");
assert_eq!(t.count("hello world"), 2);
// o200k is markedly cheaper on emoji / non-Latin scripts than cl100k_base.
assert_eq!(t.count("🌍"), 2);
```

Counting uses `CoreBPE::encode_ordinary`, so special-token strings
(`<|endoftext|>` and friends) are counted as the ordinary text they are.

## Guarantees

- `#![forbid(unsafe_code)]` — including the SIMD-style scan.
- `#![deny(missing_docs)]`.
- Deterministic, allocation-free, never panics on any input — the
  allocation-free claim is **proven by a counting-allocator test**
  (`tests/zero_alloc_count.rs`, runs on every `cargo test`), and
  determinism/no-panic by property tests over arbitrary byte strings (any
  bit pattern, including invalid/truncated UTF-8) and arbitrary UTF-8 text.
- The SWAR path is property-tested byte-for-byte against the scalar
  reference (`scalar_count_splits`), covering every chunk-boundary alignment.

## Testing

- 45 unit tests ported/derived from clawdius's `tokenizer/` module plus the
  tiktoken backends (32 build core-only; the tiktoken-backend tests are
  behind the `tiktoken` feature).
- Integration tests: trait-object usage, estimator dispatch, blend
  monotonicity; feature-gated tiktoken exactness tests for `cl100k_base` and
  `o200k_base` (including pinned cross-encoding divergence on emoji and
  multilingual samples).
- Property tests (`proptest`): determinism, nondecreasing counts under
  append, no-panic on arbitrary bytes/text, SWAR≡scalar equivalence.
- Run: `cargo test` (add `--features tiktoken` for the exact backend tests).

## License

Apache-2.0 (matching clawdius).

## Performance

Measured hot-path SLOs and allocation profile: [PERF-SLO.md](PERF-SLO.md),
with every numeric claim mapped to its proof artifact in
[CLAIMS.md](CLAIMS.md). The hot loop is pinned by an iai-callgrind
instruction gate (`benches/iai_hot_path.rs`); the allocation-free claim is
proven by `tests/zero_alloc_count.rs`.
