//! SIMD-style token counting without unsafe code.
//!
//! Provides a pure-Rust, SWAR-boosted token estimator — extracted from
//! [clawdius](https://github.com/WyattAu/clawdius) — for environments where
//! `tiktoken-rs` is unavailable (WASM builds, minimal builds, or when the
//! C/C++/Python tokenizer runtimes cannot be linked), or where a fast
//! *estimate* beats a slow *exact* count.
//!
//! # Architecture
//!
//! | Type | Description |
//! | --- | --- |
//! | [`TokenCounter`] | Trait abstracting token counting backends. |
//! | [`SimdWhitespaceTokenizer`] | SWAR byte-scan for whitespace boundaries. |
//! | [`TokenEstimator`] | Enum dispatching to the best available backend. |
//! | [`TiktokenCounter`] | Exact `cl100k_base` counting (feature `tiktoken`). |
//!
//! # The "SIMD" in the name, honestly
//!
//! clawdius's original scanner used `std::arch` SSE2/NEON intrinsics behind
//! `unsafe`. This port replaces it with **SWAR** ("SIMD Within A Register"):
//! 8 bytes are loaded per iteration into a `u64` and classified with
//! branch-free bit tricks, then run boundaries are counted with `popcount`.
//! It is fully portable (no `cfg(target_feature)`, no runtime detection) and
//! lets the whole crate declare `#![forbid(unsafe_code)]` while staying on
//! stable Rust.
//!
//! Performance characteristics, measured honestly:
//!
//! - Several times faster than a naive branchy per-byte loop on large inputs,
//!   and much faster than `str::split_whitespace().count()` (which allocates
//!   UTF-8-boundary-aware iterators).
//! - Not as fast as hand-written SSE2/AVX2 for huge buffers; SWAR moves 8
//!   bytes per instruction-stream step vs 16-32 for vector ISAs.
//! - For token *estimation* granularity (the result feeds a heuristic blend,
//!   not an exact invoice) this trade is usually invisible end-to-end.
//!
//! # Estimate semantics
//!
//! [`SimdWhitespaceTokenizer`] splits on any byte ≤ `0x20` (space, tab,
//! newline, CR, and other C0 controls), then blends the word count with a
//! ~4 chars/token heuristic and a punctuation overhead term — see
//! [`estimate_from_whitespace_splits`]. It approximates BPE behavior for
//! budgeting and rate-limit decisions; it is **not** a tokenizer table.
//!
//! # Feature flags
//!
//! - `tiktoken` (default **off**) — adds [`TiktokenCounter`] for exact
//!   `cl100k_base` counts and makes [`TokenEstimator::new`] prefer it.
//!
//! # Guarantees
//!
//! - `#![forbid(unsafe_code)]` — including the SIMD-style scan.
//! - `#![deny(missing_docs)]` — the entire public API is documented.
//! - Counting is deterministic, allocation-free, and never panics on any
//!   input (verified by property tests over arbitrary byte strings,
//!   including non-UTF-8 and truncated UTF-8 sequences).

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]

mod swar;

#[cfg(all(not(target_arch = "wasm32"), feature = "tiktoken"))]
mod tiktoken_backend;

pub use swar::{scalar_count_splits, SimdWhitespaceTokenizer};
#[cfg(all(not(target_arch = "wasm32"), feature = "tiktoken"))]
pub use tiktoken_backend::{TiktokenCounter, TiktokenError};

/// Abstraction over token counting backends.
pub trait TokenCounter: Send + Sync {
    /// Return an estimated (or exact) token count for `text`.
    fn count(&self, text: &str) -> usize;

    /// Return a human-readable name for this backend.
    fn backend_name(&self) -> &'static str;
}

/// Estimate a BPE-approximate token count from whitespace split statistics.
///
/// Deliberately kept simple: split on whitespace, add a small overhead for
/// punctuation that would normally become separate tokens in BPE, and apply
/// a ~4 chars/token heuristic for whitespace-dense text. The blend trusts
/// word count when there are clear word boundaries and falls back to the
/// character-based estimate for dense text.
pub fn estimate_from_whitespace_splits(splits: usize, byte_len: usize) -> usize {
    if splits == 0 {
        return 0;
    }
    let word_tokens = splits;
    let punct_overhead = (byte_len / 40).max(1);
    let char_based = byte_len.div_ceil(4);
    // Weighted blend of both estimates.
    let blended = (word_tokens + punct_overhead + char_based) / 2;
    blended.max(1)
}

/// Enum that dispatches to the best available token counter.
///
/// With the `tiktoken` feature enabled (on non-WASM targets) this wraps a
/// [`TiktokenCounter`]; otherwise it uses [`SimdWhitespaceTokenizer`].
#[derive(Debug)]
pub enum TokenEstimator {
    /// Exact counting via tiktoken-rs (cl100k_base).
    #[cfg(all(not(target_arch = "wasm32"), feature = "tiktoken"))]
    Tiktoken(TiktokenCounter),
    /// Fast SWAR-accelerated approximation.
    Simd(SimdWhitespaceTokenizer),
}

impl TokenEstimator {
    /// Create the best estimator available for the current configuration.
    ///
    /// # Panics
    ///
    /// With the `tiktoken` feature enabled, panics if the `cl100k_base`
    /// encoding fails to load. Use [`TokenEstimator::try_new_tiktoken`] for
    /// fallible construction.
    pub fn new() -> Self {
        #[cfg(all(not(target_arch = "wasm32"), feature = "tiktoken"))]
        {
            Self::Tiktoken(
                TiktokenCounter::new().expect("failed to initialise cl100k_base tokenizer"),
            )
        }
        #[cfg(not(all(not(target_arch = "wasm32"), feature = "tiktoken")))]
        {
            Self::Simd(SimdWhitespaceTokenizer::new())
        }
    }

    /// Fallibly construct the tiktoken-backed estimator (only available with
    /// the `tiktoken` feature on non-WASM targets).
    #[cfg(all(not(target_arch = "wasm32"), feature = "tiktoken"))]
    pub fn try_new_tiktoken() -> Result<Self, TiktokenError> {
        Ok(Self::Tiktoken(TiktokenCounter::new()?))
    }
}

impl Default for TokenEstimator {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenCounter for TokenEstimator {
    fn count(&self, text: &str) -> usize {
        match self {
            #[cfg(all(not(target_arch = "wasm32"), feature = "tiktoken"))]
            Self::Tiktoken(t) => t.count(text),
            Self::Simd(s) => s.count(text),
        }
    }

    fn backend_name(&self) -> &'static str {
        match self {
            #[cfg(all(not(target_arch = "wasm32"), feature = "tiktoken"))]
            Self::Tiktoken(_) => "tiktoken(cl100k_base)",
            Self::Simd(_) => "simd-whitespace",
        }
    }
}
