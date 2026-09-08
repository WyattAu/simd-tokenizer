//! Integration tests: trait-object usage, estimator dispatch, and
//! (feature-gated) tiktoken backend behavior.

use simd_tokenizer::{
    estimate_from_whitespace_splits, SimdWhitespaceTokenizer, TokenCounter, TokenEstimator,
};

#[test]
fn test_trait_object_usage() {
    let counter: Box<dyn TokenCounter> = Box::new(SimdWhitespaceTokenizer::new());
    assert!(counter.count("hello world") >= 1);
    assert_eq!(counter.backend_name(), "simd-whitespace");
    assert_eq!(counter.count(""), 0);
}

#[test]
fn test_estimator_default() {
    let e = TokenEstimator::default();
    let c = e.count("hello world");
    assert!(c >= 1);
    // Backend name depends on features; either way it must be non-empty.
    assert!(!e.backend_name().is_empty());
}

#[test]
fn test_estimator_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<TokenEstimator>();
    assert_send_sync::<SimdWhitespaceTokenizer>();
}

#[test]
fn test_estimate_blend_monotone_in_splits_and_len() {
    // The blend is nondecreasing in both arguments.
    for splits in 0..50 {
        for len in 0..2000usize {
            let a = estimate_from_whitespace_splits(splits, len);
            let b = estimate_from_whitespace_splits(splits + 1, len);
            let c = estimate_from_whitespace_splits(splits, len + 1);
            assert!(b >= a, "blend decreased in splits at {splits}/{len}");
            assert!(c >= a, "blend decreased in len at {splits}/{len}");
            if splits >= 1 {
                assert!(a >= 1, "blend below 1 for splits={splits} len={len}");
            } else {
                assert_eq!(a, 0, "no splits must estimate 0 tokens");
            }
        }
    }
}

// --- tiktoken backend (feature "tiktoken") ----------------------------------

#[cfg(all(not(target_arch = "wasm32"), feature = "tiktoken"))]
#[test]
fn test_estimator_prefers_tiktoken_with_feature() {
    let e = TokenEstimator::default();
    assert_eq!(e.backend_name(), "tiktoken(cl100k_base)");
}

#[cfg(all(not(target_arch = "wasm32"), feature = "tiktoken"))]
#[test]
fn test_tiktoken_exact_counts() {
    let t = simd_tokenizer::TiktokenCounter::new().expect("tiktoken init");
    assert_eq!(t.count(""), 0);
    // "hello world" is exactly 2 cl100k tokens.
    assert_eq!(t.count("hello world"), 2);
    // Non-empty input always yields at least one token.
    assert!(t.count("a") >= 1);
}

#[cfg(all(not(target_arch = "wasm32"), feature = "tiktoken"))]
#[test]
fn test_tiktoken_never_panics_on_unicode() {
    let t = simd_tokenizer::TiktokenCounter::new().expect("tiktoken init");
    let texts = [
        "🌍",
        "こんにちは世界",
        "mixed 🌍 こんにちは text",
        "\u{0}\u{1}\u{7f}",
        &"a".repeat(10_000),
    ];
    for text in &texts {
        let _ = t.count(text); // must not panic
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "tiktoken"))]
#[test]
fn test_o200k_trait_object_usage() {
    let counter: Box<dyn TokenCounter> =
        Box::new(simd_tokenizer::TiktokenCounter::o200k().expect("o200k init"));
    assert_eq!(counter.backend_name(), "tiktoken(o200k_base)");
    assert_eq!(counter.count(""), 0);
    assert!(counter.count("hello world") >= 1);
}

#[cfg(all(not(target_arch = "wasm32"), feature = "tiktoken"))]
#[test]
fn test_o200k_estimator_dispatch() {
    let e = simd_tokenizer::TokenEstimator::try_new_tiktoken_o200k().expect("o200k init");
    assert_eq!(e.backend_name(), "tiktoken(o200k_base)");
    assert!(e.count("hello world") >= 1);
}

#[cfg(all(not(target_arch = "wasm32"), feature = "tiktoken"))]
#[test]
fn test_o200k_matches_cl100k_on_english() {
    // For plain ASCII English the two vocabularies mostly agree on these
    // canonical samples; divergence is pinned on emoji/multilingual in the
    // backend unit tests.
    let cl = simd_tokenizer::TiktokenCounter::new().expect("cl100k init");
    let o2 = simd_tokenizer::TiktokenCounter::o200k().expect("o200k init");
    assert_eq!(cl.count("hello world"), o2.count("hello world"));
    assert_eq!(cl.count(""), o2.count(""));
}

#[cfg(all(not(target_arch = "wasm32"), feature = "tiktoken"))]
#[test]
fn test_o200k_never_panics_on_unicode() {
    let t = simd_tokenizer::TiktokenCounter::o200k().expect("o200k init");
    let texts = [
        "🌍",
        "こんにちは世界",
        "mixed 🌍 こんにちは text",
        "\u{0}\u{1}\u{7f}",
        &"a".repeat(10_000),
    ];
    for text in &texts {
        let _ = t.count(text); // must not panic
    }
}
