// Benchmarks run on fixed, known-good inputs; unwrap failures abort the
// bench run visibly, which is the desired behavior here.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

//! Hot-path throughput: `TokenCounter::count` (SWAR whitespace estimator),
//! measured with criterion over 1 / 100 / 1000-word inputs.

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use simd_tokenizer::{SimdWhitespaceTokenizer, TokenCounter};

fn text_of(n_words: usize) -> String {
    "The quick brown fox jumps over the lazy dog. ".repeat(n_words / 9 + 1)
}

fn bench_count(c: &mut Criterion) {
    let tok = SimdWhitespaceTokenizer::new();
    let mut group = c.benchmark_group("count");
    for n in [1usize, 100, 1000] {
        let text = text_of(n);
        group.throughput(Throughput::Bytes(text.len() as u64));
        group.bench_function(format!("count_{n}w"), |b| {
            b.iter(|| black_box(tok.count(black_box(&text))))
        });
    }
    group.finish();
}

fn bench_count_splits(c: &mut Criterion) {
    let tok = SimdWhitespaceTokenizer::new();
    let mut group = c.benchmark_group("count_splits");
    for n in [1usize, 100, 1000] {
        let text = text_of(n);
        let bytes = text.as_bytes();
        group.throughput(Throughput::Bytes(bytes.len() as u64));
        group.bench_function(format!("splits_{n}w"), |b| {
            b.iter(|| black_box(tok.count_splits(black_box(bytes))))
        });
    }
    group.finish();
}

criterion_group!(benches, bench_count, bench_count_splits);
criterion_main!(benches);
