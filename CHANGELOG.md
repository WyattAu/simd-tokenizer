# Changelog

All notable changes to this project are documented here. Format: [Keep a
Changelog](https://keepachangelog.com/) — versions follow [semver](https://semver.org).

## [Unreleased]

## [0.1.1] - 2026-09-08

### Added
- `o200k_base` encoding behind the existing `tiktoken` feature:
  `TiktokenCounter::o200k()` and
  `TokenEstimator::try_new_tiktoken_o200k()`. `TiktokenCounter::new` still
  loads `cl100k_base`.
- Feature-gated `cl100k_base` / `o200k_base` benchmark groups in the
  criterion bench (`tiktoken/cl100k_*`, `tiktoken/o200k_*`).
- `no_std` support (carried over from `Unreleased`): the SWAR estimator
  builds core-only with `--no-default-features` (thumbv7em-none-eabihf
  passes `cargo check`). New `std` feature (no-op today); the `tiktoken`
  feature implies `std` since tiktoken-rs is std-bound.

### Changed
- `TiktokenCounter::count` now uses `CoreBPE::encode_ordinary` instead of
  `encode_with_special_tokens`: special-token strings (`<|endoftext|>` and
  friends) in the input are counted as ordinary text. Counts for text
  without special-token strings are unchanged.
- `TokenEstimator::backend_name` now delegates to the wrapped counter, so an
  o200k-backed estimator reports `tiktoken(o200k_base)`.

### Fixed
- `cargo test` / `cargo clippy --all-targets` with default features failed
  to compile the `swar` unit tests under `no_std` (CI never caught it because
  it builds with `--all-features`).

## [0.1.0] - 2026-09-04

### Added
- SWAR-accelerated whitespace token estimator with exact-count fallback.
- Published to crates.io (2026-09-04).
