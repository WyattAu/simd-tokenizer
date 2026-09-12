# Performance SLOs — simd-tokenizer

Every numeric claim in this file and the README is inventoried against its
proof artifact in [CLAIMS.md](CLAIMS.md).

Measured with criterion (`cargo bench --bench count`), 2026-09.
Hardware: Intel(R) Core(TM) i5-9400F CPU @ 2.90GHz, 6 cores, Linux x86_64.
Criterion reports mean/median/stddev, not percentiles; **P50 column = criterion
mean** (P99 is not directly measured; the CI bench job compares means against
the saved `ci` baseline).

## Measured (mean per call; inputs are N-word english-ish text)

| Benchmark | Input | P50 (mean) | Throughput |
|---|---|---|---|
| `TokenCounter::count` (SWAR) | 1 word (~45 B) | **22.6 ns** | ~2 GB/s |
| `count`, 100 words (~450 B) | | 124.3 ns | ~3.6 GB/s |
| `count`, 1000 words (~4.5 KB) | | 1 109.7 ns | ~4.1 GB/s |
| `count_splits` (SWAR raw) | 1 word | 24.9 ns | |
| `count_splits`, 1000 words | | 1 088.7 ns | ~4.1 GB/s |

## SLO statements

- `SimdWhitespaceTokenizer::count` runs at **≥ 3 GB/s on inputs ≥ 100 B**
  (measured 2026-09, 6-core x86_64), i.e. **~0.25 ns per byte**.
- Small-input latency (LLM request path, < 1 KB): **< 150 ns P50 per count**.

## Allocation profile (proven, not just read)

- **0 allocations per `count` call.** The SWAR scanner reads `&[u8]` slices
  in 8-byte lanes with `popcount` reduction; both `count` and `count_splits`
  are `(usize, &str) -> usize` computations with no `Vec`/`String`/`format!`
  on the path. **Verified 2026-09-12 with a counting global allocator**
  (`tests/zero_alloc_count.rs`, runs on every `cargo test`): delta = 0 over
  repeated `count`/`count_splits` calls on small, large, and
  boundary-shaped inputs. The `tiktoken` feature backend is a separate
  exact-count path and is not covered by this claim.
- Instruction-count gate: `cargo bench --bench iai_hot_path` (iai-callgrind;
  CI-only) pins the SWAR hot loop — 10 814 instructions per ~4.5 KB `count`
  (the 4.1 GB/s regime), reproducible for a given binary.

## Regression policy

- Baselines are saved on main in CI by the shared bench job
  ([rust-kit.yml](https://github.com/WyattAu/engineering-standards/blob/main/.github/workflows/rust-kit.yml),
  `cargo bench -- --save-baseline ci`), non-gating (regression visibility).
- Local: `cargo bench --bench count -- --save-baseline main`, compare with
  `-- --baseline main`.
- Alert threshold: >1.5× mean regression on `count/count_1000w` (large inputs
  are the steady-state workload).
