# Changelog

All notable changes to this project are documented here. Format: [Keep a
Changelog](https://keepachangelog.com/) — versions follow [semver](https://semver.org).

## [Unreleased]

### Added
- `no_std` support: the SWAR estimator builds core-only with
  `--no-default-features` (thumbv7em-none-eabihf passes `cargo check`).
  New `std` feature (no-op today); the `tiktoken` feature implies `std`
  since tiktoken-rs is std-bound.

## [0.1.0] - 2026-09-04

### Added
- SWAR-accelerated whitespace token estimator with exact-count fallback.
- Published to crates.io (2026-09-04).
