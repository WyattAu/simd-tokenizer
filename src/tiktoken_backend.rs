//! tiktoken-rs wrapper implementing [`TokenCounter`].
//!
//! Only compiled when `feature = "tiktoken"` is enabled **and** the target
//! is not `wasm32` (tiktoken-rs's BPE tables are memory-heavy on WASM).

use crate::TokenCounter;

/// Error returned when the tiktoken encoding cannot be initialised.
#[derive(Debug, thiserror::Error)]
#[error("failed to initialise tiktoken cl100k_base encoding: {0}")]
pub struct TiktokenError(String);

/// Exact token counter backed by tiktoken-rs `cl100k_base`.
pub struct TiktokenCounter {
    bpe: tiktoken_rs::CoreBPE,
}

impl std::fmt::Debug for TiktokenCounter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // `tiktoken_rs::CoreBPE` does not implement `Debug`.
        f.debug_struct("TiktokenCounter").finish_non_exhaustive()
    }
}

impl TiktokenCounter {
    /// Initialise with the `cl100k_base` encoding (GPT-4 / ChatGPT).
    pub fn new() -> Result<Self, TiktokenError> {
        Ok(Self {
            bpe: tiktoken_rs::cl100k_base().map_err(|e| TiktokenError(e.to_string()))?,
        })
    }
}

impl TokenCounter for TiktokenCounter {
    fn count(&self, text: &str) -> usize {
        self.bpe.encode_with_special_tokens(text).len()
    }

    fn backend_name(&self) -> &'static str {
        "tiktoken(cl100k_base)"
    }
}

#[cfg(test)]
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
}
