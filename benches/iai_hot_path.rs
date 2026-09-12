// iai-callgrind benchmarks run once under Valgrind on fixed inputs; the
// harness measures instruction counts, so there is no "expected failure"
// recovery path — a panic aborts the run visibly, which is what we want.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

//! Deterministic regression gate for the SWAR hot loop behind the
//! PERF-SLO.md claims (≥ 3 GB/s on ≥ 100 B inputs, "0 allocations per
//! count"). Criterion (`benches/count.rs`) owns the wall-clock numbers;
//! instruction counts are the load-independent pass/fail signal.
//!
//! Workflow:
//!
//! - main: `cargo bench --bench iai_hot_path -- --save-baseline=main`
//! - PRs: `cargo bench --bench iai_hot_path -- --baseline=main --fail-fast`
//! - Locally this needs `valgrind` installed; without it, compile-check
//!   only: `cargo bench --no-run --bench iai_hot_path`.

use std::hint::black_box;

use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use simd_tokenizer::{SimdWhitespaceTokenizer, TokenCounter};

/// Same fixture family as `benches/count.rs`: english-ish 1000-word text
/// (~4.5 KB) — the steady-state workload the 4.1 GB/s claim is about.
fn setup_text_1000w() -> (SimdWhitespaceTokenizer, String) {
    (
        SimdWhitespaceTokenizer::new(),
        "The quick brown fox jumps over the lazy dog. ".repeat(1000 / 9 + 1),
    )
}

fn setup_text_100w() -> (SimdWhitespaceTokenizer, String) {
    (
        SimdWhitespaceTokenizer::new(),
        "The quick brown fox jumps over the lazy dog. ".repeat(100 / 9 + 1),
    )
}

// Full `count` hot loop (SWAR scan + blend) on ~4.5 KB.
#[library_benchmark]
#[bench::steady_state(setup = setup_text_1000w)]
fn count_1000w(env: (SimdWhitespaceTokenizer, String)) -> usize {
    let (tok, text) = env;
    black_box(tok.count(black_box(&text)))
}

// `count` on ~450 B: the SLO "≥ 3 GB/s on inputs ≥ 100 B" regime.
#[library_benchmark]
#[bench::steady_state(setup = setup_text_100w)]
fn count_100w(env: (SimdWhitespaceTokenizer, String)) -> usize {
    let (tok, text) = env;
    black_box(tok.count(black_box(&text)))
}

// Raw SWAR scan without the blend — isolates the scanner.
#[library_benchmark]
#[bench::steady_state(setup = setup_text_1000w)]
fn count_splits_1000w(env: (SimdWhitespaceTokenizer, String)) -> usize {
    let (tok, text) = env;
    black_box(tok.count_splits(black_box(text.as_bytes())))
}

library_benchmark_group!(
    name = iai_hot_path;
    benchmarks =
        count_1000w,
        count_100w,
        count_splits_1000w
);

main!(library_benchmark_groups = iai_hot_path);
