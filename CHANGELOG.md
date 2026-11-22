# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

There are no tagged GitHub Releases yet. The crate version in the tree is
`0.1.0`. Older history is summarized only where the existing git commits
already record it; this file does not invent past release dates.

## [Unreleased]

### Added

- Open-source project files: English and Chinese READMEs, contributor and
  security policies, this changelog, issue forms, and a pull request template.
- `LICENSE` for the MIT terms already declared in `Cargo.toml`.
- `rust-toolchain.toml` pinning Rust 1.100.0 with `rustfmt` and `clippy`.
- A reproducible third-party license inventory and the script used to generate
  it.
- Additional unit tests for connection-string parsing, SQL classification, and
  settings file I/O.

### Changed

- CI now pins the same Rust 1.100.0 toolchain, runs `rustfmt` (non-blocking
  until the tree is fully formatted), and runs Clippy as a required check with
  a documented allowlist for existing lints.
- Workspace package metadata (`authors`, `repository`, `rust-version`) is now
  inherited by member crates so the MSRV pin is actually enforced.
