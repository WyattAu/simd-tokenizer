// Zero-allocation hot-path gate: a counting global allocator proves the
// PERF-SLO.md claim that `count` performs 0 allocations per call.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

//! Allocation counter tests for the SWAR hot path.
//!
//! PERF-SLO.md claims, from code reading: **0 allocations per `count`
//! call** — the scanner reads `&[u8]` slices in 8-byte lanes with
//! `popcount` reduction; no `Vec`/`String`/`format!` on the path. This
//! binary turns that reading claim into a measured fact on every
//! `cargo test` run. The iai-callgrind instruction-count gate (CI-only;
//! requires valgrind) pins the cycle cost; this file pins heap behavior.
//!
//! The `tiktoken` feature backend is a separate exact-count path and is
//! explicitly *not* covered by the zero-alloc claim — these tests only
//! exercise the core SWAR tokenizer.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use simd_tokenizer::{SimdWhitespaceTokenizer, TokenCounter};

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

fn allocations() -> usize {
    ALLOCATIONS.load(Ordering::Relaxed)
}

const WORDS: &str = "The quick brown fox jumps over the lazy dog. ";
const ITERATIONS: usize = 100;

/// One sequential test: the allocation counter is process-global, so
/// parallel test threads would pollute each other's counts.
#[test]
fn count_hot_path_is_allocation_free() {
    let tok = SimdWhitespaceTokenizer::new();

    // --- `count`: 0 allocations per call, small and large inputs. ---
    let text_100w = WORDS.repeat(100 / 9 + 1);
    let text_1000w = WORDS.repeat(1000 / 9 + 1);
    // Warm-up outside the measured window.
    std::hint::black_box(tok.count(&text_1000w));

    let before = allocations();
    for _ in 0..ITERATIONS {
        std::hint::black_box(tok.count(&text_100w));
        std::hint::black_box(tok.count(&text_1000w));
    }
    assert_eq!(
        allocations(),
        before,
        "count must not allocate (before: {before}, after: {})",
        allocations()
    );

    // --- `count_splits`: 0 allocations per call. ---
    let bytes = text_1000w.as_bytes();
    let before = allocations();
    for _ in 0..ITERATIONS {
        std::hint::black_box(tok.count_splits(bytes));
    }
    assert_eq!(
        allocations(),
        before,
        "count_splits must not allocate (before: {before}, after: {})",
        allocations()
    );

    // --- Boundary shapes: empty, control bytes, chunk-crossing runs. ---
    let odd = "a\tb\rc\n d\u{1}e  f".repeat(137); // spans 8-byte lanes unevenly
    let before = allocations();
    std::hint::black_box(tok.count(&odd));
    std::hint::black_box(tok.count(""));
    assert_eq!(
        allocations(),
        before,
        "boundary-shaped inputs must not allocate"
    );

    // --- Counter sanity guard. ---
    let before = allocations();
    std::hint::black_box(format!("fresh-{before}"));
    assert!(
        allocations() > before,
        "format! must allocate (counter sanity check)"
    );
}
