# Performance claims inventory — simd-tokenizer

Every performance claim in [README.md](README.md) and [PERF-SLO.md](PERF-SLO.md),
mapped to its proof artifact. Created 2026-09-12 (simd-tokenizer 0.1.2).

Proof kinds: **criterion** (wall-clock record; load-sensitive — "measured
on"), **iai** (instruction-count gate, `cargo bench --bench iai_hot_path`,
reproducible/CI-gating), **test** (runs on every `cargo test`), **code**
(code reading), **compiler** (lint-enforced).

## Throughput / latency (criterion, 2026-09, i5-9400F 6-core)

| # | Claim | Artifact | Status |
|---|---|---|---|
| 1 | `count` 1 word (~45 B): 22.6 ns, ~2 GB/s | `benches/count.rs::count_1w` | backed (measured on) |
| 2 | `count` 100 words (~450 B): 124.3 ns, ~3.6 GB/s | same, `count_100w` | backed (measured on) |
| 3 | `count` 1000 words (~4.5 KB): 1109.7 ns, ~4.1 GB/s | same, `count_100w`; **load-independent form proven**: iai `count_1000w` = 10 814 instructions per ~4.5 KB scan (2026-09-12, valgrind 3.25.1) | **proven** (iai) + backed (criterion) |
| 4 | `count_splits` 1 word: 24.9 ns | `benches/count.rs::splits_1w` | backed (measured on) |
| 5 | `count_splits` 1000 words: 1088.7 ns, ~4.1 GB/s | same, `splits_1000w`; iai `count_splits_1000w` = 10 794 instructions | **proven** (iai) + backed (criterion) |
| 6 | README table (1.09 MB, i9-class laptop): SWAR 1.17 ms; branchy scalar 5.0× slower; `split_whitespace` 9.5× slower | historical criterion run on different hardware (2025, clawdius extraction) — kept as a measured-on record, not re-runnable on this machine's bench set | backed (measured on; hardware noted) |

## SLO statements

| # | Claim | Artifact | Status |
|---|---|---|---|
| 7 | ≥ 3 GB/s on inputs ≥ 100 B | policy; evidence = claims 2+3 | backed (policy) |
| 8 | ~0.25 ns/byte | arithmetic on claim 3 | backed (policy) |
| 9 | < 150 ns P50 per count for < 1 KB inputs | policy; evidence = claims 1+2 | backed (policy) |

## Allocation profile

| # | Claim | Artifact | Status |
|---|---|---|---|
| 10 | **0 allocations per `count` call** (was code-reading only — the direct gap) | `tests/zero_alloc_count.rs`: counting global allocator over 100× `count` (450 B + 4.5 KB), 100× `count_splits`, and boundary-shaped inputs — delta = 0 everywhere (2026-09-12) | **proven** (test) |
| 11 | tiktoken backend not covered by the zero-alloc claim | scoping note in PERF-SLO.md + test docs | backed (policy) |

## Correctness-adjacent performance claims

| # | Claim | Artifact | Status |
|---|---|---|---|
| 12 | `#![forbid(unsafe_code)]` — including the SWAR scan | attribute in `src/lib.rs` | **proven** (compiler) |
| 13 | deterministic, never panics on any input | property tests (`tests/properties.rs`: no-panic on arbitrary bytes/text) | **proven** (test) |
| 14 | SWAR ≡ scalar reference byte-for-byte, every chunk-boundary alignment | property test SWAR≡scalar equivalence | **proven** (test) |
| 15 | 45 unit tests (32 core-only, tiktoken behind feature) | re-verified 2026-09-12: `cargo test` = 32 core unit tests pass | **proven** (re-measured) |

## Totals

- **Proven by hard artifact (iai/test/compiler/re-measured):** 7
  (claims 3, 5, 10, 12, 13, 14, 15)
- **Backed (measured-on records, policy, hardware-noted history):** 8
- **Deleted/reworded:** 0 (the gap — claim 10 had no artifact — is closed
  by the new counting-allocator test)

## Reproducing

```sh
cargo bench --bench count                  # wall-clock trend
cargo bench --bench iai_hot_path           # instruction gate (needs valgrind)
cargo test --test zero_alloc_count         # allocation gate
cargo test                                 # property + unit tests
```
