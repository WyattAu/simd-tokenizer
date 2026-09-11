//! Property-based tests for the tokenizer.
//!
//! Properties (per extraction spec):
//! 1. Counting is deterministic.
//! 2. Counts are monotonically nondecreasing under append.
//! 3. Counting never panics on arbitrary bytes (any bit pattern, including
//!    invalid/truncated UTF-8) and on arbitrary UTF-8 text.
//! 4. The SWAR path matches the scalar reference byte-for-byte on every
//!    input (covers all chunk-boundary alignments).

use proptest::prelude::*;
use simd_tokenizer::{scalar_count_splits, SimdWhitespaceTokenizer, TokenCounter};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1024))]

    #[test]
    fn prop_count_is_deterministic(text in ".*") {
        let t = SimdWhitespaceTokenizer::new();
        prop_assert_eq!(t.count(&text), t.count(&text));
    }

    #[test]
    fn prop_count_is_nondecreasing_under_append(
        prefix in ".*",
        suffix in ".*"
    ) {
        let t = SimdWhitespaceTokenizer::new();
        let before = t.count(&prefix);
        let after = t.count(&format!("{prefix}{suffix}"));
        prop_assert!(
            after >= before,
            "append decreased count: {before} -> {after}"
        );
    }

    #[test]
    fn prop_count_never_panics_on_arbitrary_bytes(bytes in proptest::collection::vec(any::<u8>(), 0..4096)) {
        let t = SimdWhitespaceTokenizer::new();
        let _ = t.count_splits(&bytes); // must not panic on any bit pattern
    }

    #[test]
    fn prop_count_never_panics_on_arbitrary_text(text in ".*") {
        let t = SimdWhitespaceTokenizer::new();
        let _ = t.count(&text);
    }

    #[test]
    fn prop_swar_matches_scalar(bytes in proptest::collection::vec(any::<u8>(), 0..4096)) {
        prop_assert_eq!(
            SimdWhitespaceTokenizer::new().count_splits(&bytes),
            scalar_count_splits(&bytes),
            "SWAR/scalar divergence on {} bytes",
            bytes.len()
        );
    }

    #[test]
    fn prop_splits_at_most_one_per_byte(bytes in proptest::collection::vec(any::<u8>(), 0..4096)) {
        let splits = SimdWhitespaceTokenizer::new().count_splits(&bytes);
        prop_assert!(splits <= bytes.len() + 1);
    }
}
