# Contributing to Ace

Thanks for helping improve Ace. This document covers how to work on the
repository locally. Product behavior should stay the same unless a change is
intentional and described in the pull request.

English is the primary language for code, commit messages, and review
comments. Issue and discussion text may be in English or Chinese.

## Development setup

1. Install Rust **1.100.0**, including `rustfmt` and `clippy`. The repo
   contains `rust-toolchain.toml`, so `rustup` will select this toolchain when
   you are in the project directory.
2. On Windows, install the MSVC build tools and a Windows 10 (or later) SDK
   (`rc.exe` is used to embed the application icon).
3. On Linux, install the system libraries listed in
   [`.github/workflows/ci.yml`](.github/workflows/ci.yml) (fontconfig, xkbcommon,
   xcb, Wayland, Vulkan, and related development packages).
4. On macOS, install the Xcode command line tools. The optional
   `packaging/build-macos-app.sh` script also needs `codesign` (and `hdiutil`
   if you build a disk image).

## Useful commands

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets
cargo fmt --all
cargo run -p ace-app
```

The default binary is `ace`. Running it with no subcommand opens the desktop
GUI.

## Code style

- Format **files you touch** with `cargo fmt` before opening a pull request.
  The existing tree is not yet uniformly rustfmt-clean; please do not reformat
  unrelated files just to satisfy `cargo fmt --all`.
- Clippy runs in CI. A small set of existing lints is currently allowed in
  the workflow so a full-repo cleanup is not required. Do not add new
  warnings if you can reasonably avoid them.
- Prefer tests around connection parsing, SQL helpers, and configuration
  I/O over screenshots or manual-only checks when the behavior is
  deterministic.

## Tests

- Unit tests should use temporary directories and clearly fake values. Never
  read or write the real user config directory in tests.
- Do not put real passwords, private keys, or production connection strings
  into tests, fixtures, or logs.
- The GitHub Actions `mysql-integration` job is a smoke test against a local
  MySQL 8 service. It is optional to run that exact flow on your machine.

## Pull requests

- Use the pull request template and link any related issue.
- Describe what you changed and how you verified it. If a check could not be
  run (for example, you are on Windows and did not exercise the macOS app
  script), say so instead of claiming it passed.
- Do not include `target/`, editor settings, logs, `favorites.json`,
  `credentials.json`, or other local user data.
- Do not rewrite git history to rotate secrets; if you discover a real
  credential, report it through the process in [SECURITY.md](SECURITY.md)
  and stop short of committing the value.

## License

By contributing, you agree that your contribution is licensed under the MIT
License in [`LICENSE`](LICENSE).
