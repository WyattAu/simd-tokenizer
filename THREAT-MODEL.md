# Threat Model — simd-tokenizer

Reference: STRIDE. Scope: the crate's public API surface (`TokenEstimator`,
`SimdWhitespaceTokenizer`, `TiktokenCounter` (feature `tiktoken`),
`estimate_from_whitespace_splits`, `TokenCounter`) as used by a downstream
service. Trust boundaries: (1) untrusted text entering `count`, (2) the
feature-selected backend (estimate vs exact), (3) the dependency tree
(tiktoken-rs when enabled).

This crate has a **minimal security-sensitive surface**: it is a pure,
allocation-free-in-the-hot-path counting function over `&str`. It holds no
secrets, performs no I/O, and mutates nothing. The credible threats are
availability (panic/DoS on hostile input) and *semantic* (estimate
under/over-count feeding bad decisions).

## Assets

| ID | Asset | Example |
|----|-------|---------|
| A1 | Availability of the caller (count never aborts) | Hostile/truncated UTF-8 input panicking a request worker |
| A2 | Soundness of decisions fed by counts | Rate limiter or budget heuristic trusting wildly wrong estimates |

## STRIDE Analysis

| # | Threat | Category | Surface | Mitigation | Verifying test |
|---|--------|----------|---------|------------|----------------|
| T1 | Panic on arbitrary/hostile input (non-UTF-8, truncated sequences, empty) | DoS | `SimdWhitespaceTokenizer::count`, `estimate_from_whitespace_splits` | `#![forbid(unsafe_code)]`; SWAR byte classification over `&str` with no slicing on non-boundaries; property tests assert no panic over arbitrary bytes and arbitrary text; `splits == 0` short-circuits | `prop_count_never_panics_on_arbitrary_bytes`, `prop_count_never_panics_on_arbitrary_text` (`tests/properties.rs`), `test_large_input_no_panic`, `test_empty_string`, `test_simd_tokenizer_empty` |
| T2 | SWAR scan diverging from scalar semantics | Tampering | `count_splits_swar` | Property test pins SWAR output to the scalar reference implementation for arbitrary inputs — a bit-trick regression cannot ship silently | `prop_swar_matches_scalar`, `prop_splits_at_most_one_per_byte`, `scalar_count_splits` reference |
| T3 | tiktoken backend panic on unicode edge cases | DoS | `TiktokenCounter::count` (feature `tiktoken`) | Backed by maintained tiktoken-rs; explicit tests assert no panic on unicode and determinism; fallible constructors (`try_new_tiktoken`, `try_new_tiktoken_o200k`) exist for callers avoiding the documented `new()` panic | `test_tiktoken_never_panics_on_unicode`, `test_o200k_never_panics_on_unicode`, `test_tiktoken_exact_counts`, `test_o200k_known_counts` |
| T4 | Backend selection surprise (estimate silently used where exact expected) | Spoofing | `TokenEstimator::new` dispatch | Backend is pinned by feature flag and reported via `backend_name()`; dispatch is exhaustive over the enum, so a caller can always know which backend answered | `test_estimator_prefers_tiktoken_with_feature`, `test_estimator_default`, `test_estimator_send_sync`, `test_o200k_estimator_dispatch` |
| T5 | Construction panic on tokenizer-table load failure | DoS | `TokenEstimator::new` (tiktoken feature) | Documented, deliberate: `new()` expects the cl100k load; fallible path is `try_new_tiktoken`. Not a runtime-input condition (startup-only) | Panic contract documented on `new()`; `try_new_tiktoken` error path type-tested via `TiktokenError` |
| T6 | Determinism loss (same input, different counts) | Tampering | all counters | Pure functions over the input; property tests assert determinism for both backends | `prop_count_is_deterministic`, `test_deterministic`, `test_o200k_deterministic` |

## Repudiation

Not applicable — stateless counting, no logs, no identity.

## Out of Scope

- **Estimate fitness for billing or security decisions:** the whitespace
  estimator is a heuristic (~4 chars/token blend). A rate limiter keyed on
  it can be gamed with whitespace-dense or punctuation-dense text; token
  counts are cost approximations, not invoices.
- Secret-handling: token *counts* of secret text inherently leak length
  statistics to whoever reads them; that is inherent to exposing a counter.
- tiktoken-rs internals (BPE tables, merge rules).

## Residual Risks

- **R1 (Low, accepted):** Estimation error is unbounded adversarially (A2):
  an attacker shaping input can push the estimate arbitrarily far from the
  true BPE count (up to ~4× under on whitespace-dense payloads). Acceptable
  because the documented use is budgeting/rate-limit heuristics; use
  `TiktokenCounter` where the number must be exact.
- **R2 (Low, accepted):** `punct_overhead = (byte_len / 40).max(1)` floors
  at 1 for any non-empty input — irrelevant at security scale, noted only
  because it makes `estimate ≥ 1` an invariant callers may rely on.
- **R3 (Low, accepted):** With the `tiktoken` feature, `TokenEstimator`
  embeds ~200+ bytes of BPE tables per instance (documented
  `large_enum_variant`); constructing one per request is wasteful but not a
  security issue. Share the estimator.
- **R4 (Low, accepted):** Dependency risk in tiktoken-rs (fuzzed upstream,
  but no in-repo `cargo audit` gate for it or serde/transitive deps).
