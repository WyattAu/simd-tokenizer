//! tiktoken-rs wrapper implementing [`TokenCounter`].
//!
//! Only compiled when `feature = "tiktoken"` is enabled **and** the target
//! is not `wasm32` (tiktoken-rs's BPE tables are memory-heavy on WASM).

use crate::TokenCounter;

/// Error returned when a tiktoken encoding cannot be initialised.
#[derive(Debug, thiserror::Error)]
#[error("failed to initialise tiktoken encoding: {0}")]
pub struct TiktokenError(String);

/// Exact token counter backed by tiktoken-rs.
///
/// Two encodings are available:
///
/// - [`TiktokenCounter::new`] — `cl100k_base` (GPT-4 / ChatGPT).
/// - [`TiktokenCounter::o200k`] — `o200k_base` (GPT-4o / o-series), with
///   substantially cheaper multilingual and emoji tokenization.
pub struct TiktokenCounter {
    bpe: tiktoken_rs::CoreBPE,
    backend_name: &'static str,
}

impl std::fmt::Debug for TiktokenCounter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // `tiktoken_rs::CoreBPE` does not implement `Debug`.
        f.debug_struct("TiktokenCounter")
            .field("backend", &self.backend_name)
            .finish_non_exhaustive()
    }
}

impl TiktokenCounter {
    /// Initialise with the `cl100k_base` encoding (GPT-4 / ChatGPT).
    pub fn new() -> Result<Self, TiktokenError> {
        Self::from_encoding("tiktoken(cl100k_base)", tiktoken_rs::cl100k_base())
    }

    /// Initialise with the `o200k_base` encoding (GPT-4o / o-series).
    ///
    /// # Examples
    ///
    /// ```
    /// use simd_tokenizer::{TiktokenCounter, TokenCounter};
    ///
    /// let t = TiktokenCounter::o200k().expect("o200k_base loads");
    /// assert_eq!(t.count("hello world"), 2);
    /// assert_eq!(t.backend_name(), "tiktoken(o200k_base)");
    /// ```
    pub fn o200k() -> Result<Self, TiktokenError> {
        Self::from_encoding("tiktoken(o200k_base)", tiktoken_rs::o200k_base())
    }

    /// Shared constructor: `init` is the tiktoken-rs loader for the encoding
    /// (`Result<CoreBPE, E>` where `E: Display`, so the concrete error type
    /// stays tiktoken-rs's business).
    fn from_encoding<E: std::fmt::Display>(
        backend_name: &'static str,
        init: Result<tiktoken_rs::CoreBPE, E>,
    ) -> Result<Self, TiktokenError> {
        Ok(Self {
            bpe: init.map_err(|e| TiktokenError(e.to_string()))?,
            backend_name,
        })
    }
}

impl TokenCounter for TiktokenCounter {
    /// Exact token count via `CoreBPE::encode_ordinary`.
    ///
    /// Ordinary encoding never expands special-token strings (`<|endoftext|>`
    /// and friends): text that merely *looks like* a special token is counted
    /// as the ordinary text it is, so counts are stable for arbitrary input.
    fn count(&self, text: &str) -> usize {
        self.bpe.encode_ordinary(text).len()
    }

    fn backend_name(&self) -> &'static str {
        self.backend_name
    }
}

#[cfg(test)]
// Test code: expect is the idiomatic way to assert setup success.
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    // --- ported from clawdius tiktoken_backend.rs ---------------------------

    #[test]
    fn test_tiktoken_empty() {
        let t = TiktokenCounter::new().expect("tiktoken init");
        assert_eq!(t.count(""), 0);
    }

    #[test]
    fn test_tiktoken_hello() {
        let t = TiktokenCounter::new().expect("tiktoken init");
        let c = t.count("hello world");
        assert!(c >= 2, "expected >= 2, got {c}");
    }

    #[test]
    fn test_tiktoken_code() {
        let t = TiktokenCounter::new().expect("tiktoken init");
        let code = r#"fn main() { println!("Hello"); }"#;
        let c = t.count(code);
        assert!(c >= 5, "expected >= 5, got {c}");
    }

    #[test]
    fn test_backend_name() {
        let t = TiktokenCounter::new().expect("tiktoken init");
        assert_eq!(t.backend_name(), "tiktoken(cl100k_base)");
    }

    #[test]
    fn test_deterministic() {
        let t = TiktokenCounter::new().expect("tiktoken init");
        let text = "The quick brown fox jumps over the lazy dog.";
        assert_eq!(t.count(text), t.count(text));
    }

    // --- o200k_base ----------------------------------------------------------

    #[test]
    fn test_o200k_empty() {
        let t = TiktokenCounter::o200k().expect("o200k init");
        assert_eq!(t.count(""), 0);
    }

    #[test]
    fn test_o200k_known_counts() {
        // Exact counts pinned against tiktoken-rs 0.12 o200k_base.
        let t = TiktokenCounter::o200k().expect("o200k init");
        assert_eq!(t.count("hello world"), 2);
        assert_eq!(t.count("Hello, world!"), 4);
        assert_eq!(t.count("The quick brown fox jumps over the lazy dog."), 10);
        assert_eq!(t.count(r#"fn main() { println!("Hello"); }"#), 9);
    }

    #[test]
    fn test_o200k_backend_name() {
        let t = TiktokenCounter::o200k().expect("o200k init");
        assert_eq!(t.backend_name(), "tiktoken(o200k_base)");
    }

    #[test]
    fn test_o200k_monotone_in_length() {
        let t = TiktokenCounter::o200k().expect("o200k init");
        let mut text = String::from("hello world");
        let mut prev = t.count(&text);
        for word in ["and", "the", "rest", "of", "the", "corpus", "follows"] {
            text.push(' ');
            text.push_str(word);
            let now = t.count(&text);
            assert!(now >= prev, "count decreased after append: {prev} -> {now}");
            prev = now;
        }
    }

    #[test]
    fn test_o200k_special_token_string_is_ordinary() {
        // `encode_ordinary` counts `<|endoftext|>` as plain text (7 tokens),
        // not as the single special token.
        let t = TiktokenCounter::o200k().expect("o200k init");
        assert_eq!(t.count("<|endoftext|>"), 7);
    }

    #[test]
    fn test_o200k_differs_from_cl100k_on_emoji_and_multilingual() {
        // The two BPE vocabularies tokenize emoji and non-Latin scripts
        // differently (o200k is more efficient there); pin the divergence so
        // a backend mix-up cannot pass silently.
        let cl = TiktokenCounter::new().expect("cl100k init");
        let o2 = TiktokenCounter::o200k().expect("o200k init");
        for text in ["🌍", "hello 🌍 world", "こんにちは世界", "Привет, мир!"] {
            assert_ne!(
                cl.count(text),
                o2.count(text),
                "cl100k and o200k unexpectedly agree on {text:?}"
            );
        }
    }

    #[test]
    fn test_o200k_deterministic() {
        let t = TiktokenCounter::o200k().expect("o200k init");
        let text = "こんにちは世界 🌍 Привет";
        assert_eq!(t.count(text), t.count(text));
    }

    #[test]
    fn test_o200k_debug_does_not_panic() {
        let t = TiktokenCounter::o200k().expect("o200k init");
        let rendered = format!("{t:?}");
        assert!(rendered.contains("o200k"));
    }
}
