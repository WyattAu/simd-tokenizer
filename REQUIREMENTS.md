# Requirements — simd-tokenizer

Numbered, testable requirements. Every requirement maps to at least one named
test; every security-relevant test cites at least one requirement. Threat
IDs reference `THREAT-MODEL.md`.

Scope note: `simd-tokenizer` provides a safe SWAR-accelerated whitespace
token estimator (`SimdWhitespaceTokenizer`, `count_splits`) with an optional
exact `tiktoken` backend (`cl100k_base`, `o200k_base`) behind the `tiktoken`
feature, plus the `TokenEstimator` dispatch enum.

## Functional

| ID | Requirement | Priority |
|----|-------------|----------|
| REQ-ST-001 | `count_splits` counts whitespace-separated tokens for any input, including non-UTF-8 boundaries, trailing/leading whitespace, and empty input | MUST |
| REQ-ST-002 | The SWAR path and the scalar reference (`scalar_count_splits`) agree on every input | MUST |
| REQ-ST-003 | `TokenEstimator::new` selects the best available backend (tiktoken exact when the feature is on, whitespace estimate otherwise) and `backend_name` reports which | MUST |
| REQ-ST-004 | `TiktokenCounter::new`/`o200k` produce exact `cl100k_base`/`o200k_base` counts; special-token strings are treated as ordinary text | SHOULD |
| REQ-ST-005 | All counters are deterministic: the same input yields the same count on every call | MUST |
| REQ-ST-006 | Counters are monotone: longer inputs never produce smaller counts | SHOULD |

## Security

| ID | Requirement | Priority |
|----|-------------|----------|
| REQ-ST-100 | No panic on arbitrary or hostile input — empty strings, emoji-heavy text, control characters, truncated UTF-8, and oversized inputs return counts (T1, T3) | MUST |
| REQ-ST-101 | SWAR must never diverge from scalar semantics — divergence would misclassify token budgets (T2); pinned by property test | MUST |
| REQ-ST-102 | Estimate-vs-exact cannot be silently swapped: `backend_name` makes the active backend observable so callers can assert exactness (T4) | SHOULD |
| REQ-ST-103 | Construction failure of the tiktoken tables surfaces as `TiktokenError`, never a panic (T5) | MUST |
| REQ-ST-104 | Byte- and word-boundary alignment (16/32/33-byte inputs) must not change results (alignment edge cases in the SWAR reader) | MUST |

## Robustness

| ID | Requirement | Priority |
|----|-------------|----------|
| REQ-ST-200 | `TokenCounter` is usable as a trait object and `TokenEstimator` is `Send + Sync` | SHOULD |
| REQ-ST-201 | Debug formatting of counters and estimators never panics | SHOULD |

## Traceability Matrix

| Requirement | Test (fn, file) | Property class |
|-------------|-----------------|----------------|
| REQ-ST-001 | `test_empty_string`, `test_whitespace_only`, `test_single_word`, `test_repeated_spaces`, `test_newlines_and_tabs`, `test_unicode_text` (`src/swar.rs`, `tests/*.rs`) | unit |
| REQ-ST-002 | `test_simd_matches_scalar`, `test_scalar_c0_controls`, `test_scalar_mixed_whitespace`, `test_scalar_leading_trailing_ws` (`src/swar.rs`) | unit/property |
| REQ-ST-003 | `test_estimator_default`, `test_estimator_prefers_tiktoken_with_feature`, `test_backend_name`, `test_o200k_backend_name` | unit |
| REQ-ST-004 | `test_tiktoken_exact_counts`, `test_o200k_known_counts`, `test_o200k_special_token_string_is_ordinary`, `test_o200k_matches_cl100k_on_english` | unit |
| REQ-ST-005 | `test_deterministic`, `test_o200k_deterministic` | unit |
| REQ-ST-006 | `test_estimate_blend_monotone_in_splits_and_len`, `test_o200k_monotone_in_length` | unit |
| REQ-ST-100 | `test_o200k_never_panics_on_unicode`, `test_tiktoken_never_panics_on_unicode`, `test_large_input_no_panic`, `test_emoji_heavy`, `test_scalar_c0_controls` | unit |
| REQ-ST-101 | `test_simd_matches_scalar` (`src/swar.rs`) | property |
| REQ-ST-102 | `test_backend_name`, `test_o200k_backend_name` | unit |
| REQ-ST-103 | `TiktokenError` construction path (`src/tiktoken_backend.rs`), `test_backend_name` | unit |
| REQ-ST-104 | `test_exact_16_bytes`, `test_exact_32_bytes`, `test_17_byte_alignment`, `test_33_byte_alignment` | unit |
| REQ-ST-200 | `test_trait_object_usage`, `test_o200k_trait_object_usage`, `test_estimator_send_sync` | unit |
| REQ-ST-201 | `test_o200k_debug_does_not_panic` | unit |

## Test Count

- 62 `#[test]` functions across unit and integration suites.
- All-features suite passes with 0 failures; no-default-features suite passes.
