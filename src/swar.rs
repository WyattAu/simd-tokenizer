//! SWAR-accelerated whitespace tokenizer.
//!
//! SWAR ("SIMD Within A Register") scans 8 bytes per iteration inside a
//! `u64`: bytes are classified with branch-free bit arithmetic and word
//! boundaries are counted with `popcount`. Fully safe and portable — no
//! `std::arch`, no runtime feature detection, identical results on every
//! platform (see the `simd_matches_scalar` tests and property tests).

use crate::{estimate_from_whitespace_splits, TokenCounter};

const HIGH: u64 = 0x8080_8080_8080_8080;
const SEVENS: u64 = 0x7F7F_7F7F_7F7F_7F7F;
const ADD_5F: u64 = 0x5F5F_5F5F_5F5F_5F5F;

/// Per-byte non-whitespace mask: `0x80` in each byte lane whose byte is
/// **not** ≤ `0x20`.
///
/// Byte `c` is non-whitespace iff `c >= 0x80` (its high bit is set — this
/// covers all UTF-8 non-ASCII bytes) or `(c & 0x7F) >= 0x21`. The 7-bit
/// addition never carries across lanes (`0x7F + 0x5F = 0xDE`), so the
/// per-byte predicate is exact — no SWAR borrow artifacts.
#[inline(always)]
fn nonwhitespace_mask(word: u64) -> u64 {
    (word | ((word & SEVENS) + ADD_5F)) & HIGH
}

/// SIMD-style (SWAR) whitespace-split token estimator.
///
/// Splits on any byte ≤ `0x20` (covers space, tab, newline, CR, and other C0
/// controls) using 8-byte SWAR loads, then blends word count + character
/// based heuristics via [`estimate_from_whitespace_splits`] for a
/// BPE-approximate token count.
#[derive(Debug, Clone, Default)]
pub struct SimdWhitespaceTokenizer {
    _priv: (), // prevent direct construction — use ::new()
}

impl SimdWhitespaceTokenizer {
    /// Create a new tokenizer.
    pub fn new() -> Self {
        Self { _priv: () }
    }

    /// Count whitespace-delimited segments (maximal runs of non-whitespace
    /// bytes).
    ///
    /// Uses the SWAR path (8 bytes per step); the portable scalar reference
    /// is available as [`scalar_count_splits`]. Both produce identical
    /// results for every input.
    pub fn count_splits(&self, bytes: &[u8]) -> usize {
        count_splits_swar(bytes)
    }
}

impl TokenCounter for SimdWhitespaceTokenizer {
    fn count(&self, text: &str) -> usize {
        let bytes = text.as_bytes();
        let splits = count_splits_swar(bytes);
        estimate_from_whitespace_splits(splits, bytes.len())
    }

    fn backend_name(&self) -> &'static str {
        "simd-whitespace"
    }
}

/// SWAR whitespace-split counter: counts the first byte of every non-
/// whitespace run. A run's first byte is a non-whitespace byte whose
/// predecessor is whitespace (or start-of-input, modeled by seeding the
/// carry as "previous was whitespace").
fn count_splits_swar(data: &[u8]) -> usize {
    let mut count = 0usize;
    let mut prev_ws = true;

    let mut chunks = data.chunks_exact(8);
    for chunk in &mut chunks {
        let word = u64::from_le_bytes(chunk.try_into().expect("chunks_exact yields 8 bytes"));
        let nonws = nonwhitespace_mask(word); // 0x80 per non-whitespace byte
        let ws = !nonws & HIGH; // 0x80 per whitespace byte

        // Run-start lanes: non-whitespace byte whose predecessor lane is
        // whitespace. `ws << 8` moves each lane's flag to its successor;
        // lane 0 uses the incoming carry.
        let starts = nonws & ((ws << 8) | if prev_ws { 0x80 } else { 0 });
        count += (starts >> 7).count_ones() as usize;

        // Carry: whitespace-ness of the chunk's last byte (lane 7).
        prev_ws = ws & (0x80 << 56) != 0;
    }

    // Scalar tail (fewer than 8 bytes remain).
    for &b in chunks.remainder() {
        let is_ws = b <= 0x20;
        if !is_ws && prev_ws {
            count += 1;
        }
        prev_ws = is_ws;
    }

    count
}

/// Portable scalar reference implementation of [`SimdWhitespaceTokenizer::
/// count_splits`]. Public so callers (and tests) can cross-verify the SWAR
/// path byte-for-byte.
pub fn scalar_count_splits(data: &[u8]) -> usize {
    let mut in_word = false;
    let mut count = 0;
    for &b in data {
        let is_ws = b <= 0x20;
        if is_ws {
            if in_word {
                count += 1;
                in_word = false;
            }
        } else {
            in_word = true;
        }
    }
    if in_word {
        count += 1;
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- ported from clawdius simd_tokenizer.rs (scalar) -------------------

    #[test]
    fn test_scalar_empty() {
        assert_eq!(scalar_count_splits(b""), 0);
    }

    #[test]
    fn test_scalar_single_word() {
        assert_eq!(scalar_count_splits(b"hello"), 1);
    }

    #[test]
    fn test_scalar_two_words() {
        assert_eq!(scalar_count_splits(b"hello world"), 2);
    }

    #[test]
    fn test_scalar_whitespace_only() {
        assert_eq!(scalar_count_splits(b"   \t\n"), 0);
    }

    #[test]
    fn test_scalar_mixed_whitespace() {
        assert_eq!(scalar_count_splits(b"a b\tc\nd"), 4);
    }

    #[test]
    fn test_scalar_leading_trailing_ws() {
        assert_eq!(scalar_count_splits(b"  hello world  "), 2);
    }

    #[test]
    fn test_scalar_c0_controls() {
        assert_eq!(scalar_count_splits(b"a\x00b\x01c"), 3);
    }

    // --- ported from clawdius simd_tokenizer.rs (tokenizer) -----------------

    #[test]
    fn test_simd_tokenizer_empty() {
        let t = SimdWhitespaceTokenizer::new();
        assert_eq!(t.count(""), 0);
    }

    #[test]
    fn test_simd_tokenizer_single() {
        let t = SimdWhitespaceTokenizer::new();
        assert!(t.count("hello") >= 1);
    }

    #[test]
    fn test_simd_tokenizer_sentence() {
        let t = SimdWhitespaceTokenizer::new();
        let c = t.count("The quick brown fox jumps over the lazy dog.");
        assert!(c >= 5);
    }

    #[test]
    fn test_simd_tokenizer_code() {
        let t = SimdWhitespaceTokenizer::new();
        let code = "fn main() { let x = 42; }";
        let c = t.count(code);
        assert!(c >= 4);
    }

    #[test]
    fn test_simd_tokenizer_unicode() {
        let t = SimdWhitespaceTokenizer::new();
        let c = t.count("こんにちは 世界 hello");
        assert!(c >= 2);
    }

    // --- ported: SWAR-vs-scalar equivalence incl. alignment edges -----------

    #[test]
    fn test_simd_matches_scalar() {
        let t = SimdWhitespaceTokenizer::new();
        let cases: &[&str] = &[
            "",
            "a",
            "hello world",
            "  leading and trailing  ",
            "tabs\there\ttoo",
            "new\nlines\nhere",
        ];
        let long = "The quick brown fox jumps over the lazy dog. ".repeat(10);
        let mut all_cases: Vec<&str> = cases.to_vec();
        all_cases.push(&long);
        for &case in &all_cases {
            assert_eq!(
                t.count_splits(case.as_bytes()),
                scalar_count_splits(case.as_bytes()),
                "mismatch for input of {} bytes",
                case.len()
            );
        }
    }

    #[test]
    fn test_17_byte_alignment() {
        let data = b"12345678901234567";
        let t = SimdWhitespaceTokenizer::new();
        assert_eq!(t.count_splits(data), scalar_count_splits(data));
    }

    #[test]
    fn test_33_byte_alignment() {
        let data = b"123456789012345678901234567890123";
        let t = SimdWhitespaceTokenizer::new();
        assert_eq!(t.count_splits(data), scalar_count_splits(data));
    }

    #[test]
    fn test_exact_16_bytes() {
        let data = b"1234567890123456";
        let t = SimdWhitespaceTokenizer::new();
        assert_eq!(t.count_splits(data), scalar_count_splits(data));
    }

    #[test]
    fn test_exact_32_bytes() {
        let data = b"12345678901234567890123456789012";
        let t = SimdWhitespaceTokenizer::new();
        assert_eq!(t.count_splits(data), scalar_count_splits(data));
    }

    // --- ported from clawdius tokenizer/mod.rs -------------------------------

    #[test]
    fn test_empty_string() {
        let t = SimdWhitespaceTokenizer::new();
        assert_eq!(t.count(""), 0);
    }

    #[test]
    fn test_single_word() {
        let t = SimdWhitespaceTokenizer::new();
        let c = t.count("hello");
        assert!(c >= 1, "single word should yield >= 1 token, got {c}");
    }

    #[test]
    fn test_simple_sentence() {
        let t = SimdWhitespaceTokenizer::new();
        let c = t.count("The quick brown fox jumps over the lazy dog.");
        assert!(c >= 5, "sentence should yield >= 5 tokens, got {c}");
    }

    #[test]
    fn test_long_text() {
        let t = SimdWhitespaceTokenizer::new();
        let text = "The quick brown fox jumps over the lazy dog. ".repeat(100);
        let c = t.count(&text);
        assert!(c >= 100, "long text should yield >= 100 tokens, got {c}");
    }

    #[test]
    fn test_unicode_text() {
        let t = SimdWhitespaceTokenizer::new();
        let c = t.count("こんにちは世界 Hello 世界 this is unicode 🌍");
        assert!(c >= 3, "unicode text should yield >= 3 tokens, got {c}");
    }

    #[test]
    fn test_code_with_symbols() {
        let t = SimdWhitespaceTokenizer::new();
        let code = r#"fn main() {
    let x = 42;
    println!("Hello, world!");
    if x > 0 {
        println!("positive");
    }
}"#;
        let c = t.count(code);
        assert!(c >= 8, "code should yield >= 8 tokens, got {c}");
    }

    #[test]
    fn test_whitespace_only() {
        let t = SimdWhitespaceTokenizer::new();
        assert_eq!(t.count("   \t\n\r  "), 0);
    }

    #[test]
    fn test_single_character() {
        let t = SimdWhitespaceTokenizer::new();
        assert_eq!(t.count("a"), 1);
    }

    #[test]
    fn test_repeated_spaces() {
        let t = SimdWhitespaceTokenizer::new();
        let c = t.count("hello   world");
        assert_eq!(t.count("hello world"), c);
    }

    #[test]
    fn test_newlines_and_tabs() {
        let t = SimdWhitespaceTokenizer::new();
        let c1 = t.count("hello\nworld");
        let c2 = t.count("hello\tworld");
        let c3 = t.count("hello world");
        assert_eq!(c1, c2);
        assert_eq!(c2, c3);
    }

    #[test]
    fn test_backend_name() {
        let t = SimdWhitespaceTokenizer::new();
        assert_eq!(t.backend_name(), "simd-whitespace");
    }

    #[test]
    fn test_estimate_from_splits_empty() {
        assert_eq!(estimate_from_whitespace_splits(0, 0), 0);
    }

    #[test]
    fn test_estimate_from_splits_single() {
        let est = estimate_from_whitespace_splits(1, 5);
        assert!(est >= 1);
    }

    #[test]
    fn test_large_input_no_panic() {
        let t = SimdWhitespaceTokenizer::new();
        let text = "word ".repeat(100_000);
        let _ = t.count(&text);
    }

    #[test]
    fn test_emoji_heavy() {
        let t = SimdWhitespaceTokenizer::new();
        let c = t.count("😀😂🤣😃😄😁😆😅🤪😊😎");
        assert!(c >= 1);
    }
}
