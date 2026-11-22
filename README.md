# Ace

[简体中文](README.zh-CN.md)

Ace is a cross-platform desktop database client for **MySQL**, **MariaDB**,
**PostgreSQL**, and **SQLite**. It is an independent Rust remake of
[Sequel Ace](https://github.com/Sequel-Ace/Sequel-Ace), with a native GUI
built on [GPUI Kit](https://crates.io/crates/gpui-kit) 0.10.0.

The current tree version is **0.1.0**. Ace is early-stage software: useful
features already exist, but many Sequel Ace workflows are still incomplete.

## Project status

- There are **no GitHub Releases**, installers, or package-manager recipes
  published from this repository yet. The supported way to try Ace is to
  build it from source.
- This README does not include screenshots (none are kept in the repo today).
- `FEATURES.md` and `ARCHITECTURE.md` are working notes for development.
  They can lag behind the code; treat the source and this README as the
  description of what is actually present.

## Features

What you can do in the current tree:

- Desktop GUI (default) plus a CLI for queries, exports, an interactive
  shell, and a local MCP server.
- Saved connection favorites, including import/export of Sequel Ace / Sequel
  Pro favorites plists.
- Connection methods: TCP/IP, socket, SSH tunnel, AWS RDS IAM, and Vault
  (the Vault OIDC browser-callback flow is still a stub).
- SSL/TLS options, including CA and client certificates, plus a request for
  the server RSA public key when using `caching_sha2_password` without SSL.
- Browse structure, content, relations, triggers, and table info; run custom
  SQL with highlighting and completion helpers.
- A bundled demo SQLite database (`demo/demo.db`) so you can open the app
  without a server.
- CLI export of query results as SQL, CSV, XML, HTML, DOT, or JSON.
- English and Simplified Chinese UI strings.
- `ace update-check` can query GitHub's latest release tag. It does **not**
  download or install updates, and there is no release to check against yet.

## Known limitations

- Many Sequel Ace features are not done yet (multi-window sessions, visual
  filter builders, a full preferences UI, user/process management, a complete
  bundle system, and others). See `FEATURES.md` for the living checklist.
- Saved passwords go to a local `credentials.json` file, not the OS
  keychain / credential manager.
- Vault OIDC login is not fully implemented.
- Keep-alive, disconnect detection, and automatic reconnect are not
  implemented.
- Import / export coverage in the GUI is narrower than the CLI helpers.
- The SQL editor is still a work in progress compared with Sequel Ace.

## Platform support

Ace is written to build on Windows, macOS, and Linux. That is not the same
as a published, tested product on every platform.

| Platform | What has been verified | What is not claimed |
|----------|------------------------|---------------------|
| Windows (x64, MSVC) | Local development and compilation of this tree | No published installer, portable ZIP, or Microsoft Store package |
| macOS | GitHub Actions compiles on `macos-latest`; `packaging/build-macos-app.sh` can build an `.app` / optional DMG | No published `.dmg`, Homebrew cask, or signed distribution |
| Linux | GitHub Actions compiles on `ubuntu-latest` after installing the GPUI system libraries listed in CI | No published AppImage, `.deb`, Flathub, or distro package |

CI currently builds and tests on `windows-latest`, `macos-latest`, and
`ubuntu-latest`. A green compile in CI is not a substitute for a packaged
release.

## Build from source

### Requirements

- Rust **1.100.0** (see `rust-toolchain.toml`). `rustup` will install the
  pinned toolchain when you enter this directory.
- **Windows:** MSVC build tools and a Windows 10 or later SDK (`rc.exe`
  embeds the application icon).
- **macOS:** Xcode command line tools.
- **Linux:** development packages used in CI, including `pkg-config`,
  `libfontconfig1-dev`, `libfreetype-dev`, `libxkbcommon-dev`,
  `libxkbcommon-x11-dev`, `libxcb1-dev`, `libxcb-xkb-dev`,
  `libxcb-render0-dev`, `libxcb-shape0-dev`, `libxcb-xfixes0-dev`,
  `libx11-dev`, `libwayland-dev`, `libvulkan-dev`, `libssl-dev`, and `cmake`.

### Commands

```bash
git clone https://github.com/imaben/Ace.git
cd Ace
cargo build -p ace-app
cargo run -p ace-app
```

`cargo run -p ace-app` opens the GUI. You can also run the same binary with
subcommands:

```bash
# Print version and driver list
cargo run -p ace-app -- info

# One-shot query (example; use your own URL)
cargo run -p ace-app -- query 'sqlite://demo/demo.db' 'SELECT 1 AS ok'

# Interactive CLI shell
cargo run -p ace-app -- connect 'sqlite://demo/demo.db'
```

Release-profile builds are available with `cargo build -p ace-app --release`.
On macOS, `packaging/build-macos-app.sh` wraps a release binary in
`Ace.app`. The `packaging/packager.toml` file sketches Windows NSIS and
Linux AppImage/deb packaging, but those products are not shipped from CI
today.

## Configuration, data, and uninstall

Ace stores settings and local data with the `directories` crate,
`ProjectDirs::from("com", "Ace", "Ace")`:

| File | Typical Windows location | Typical macOS location | Typical Linux location |
|------|--------------------------|------------------------|------------------------|
| `settings.toml` | `%APPDATA%\Ace\Ace\settings.toml` | `~/Library/Application Support/com.Ace.Ace/settings.toml` | `~/.config/ace/settings.toml` |
| `favorites.json` | `%APPDATA%\Ace\Ace\favorites.json` | `~/Library/Application Support/com.Ace.Ace/favorites.json` | `~/.local/share/ace/favorites.json` |
| `credentials.json` | `%APPDATA%\Ace\Ace\credentials.json` | `~/Library/Application Support/com.Ace.Ace/credentials.json` | `~/.local/share/ace/credentials.json` |
| `history.sqlite3` | `%APPDATA%\Ace\Ace\history.sqlite3` | `~/Library/Application Support/com.Ace.Ace/history.sqlite3` | `~/.local/share/ace/history.sqlite3` |

If the platform directories cannot be resolved, Ace falls back to a local
`.ace/` directory.

To uninstall a source build, delete the `ace` / `ace.exe` binary (and
`Ace.app` on macOS if you used the packaging script). Then delete the
directories above if you also want to remove favorites, passwords, and
query history. There is no installer-driven uninstaller yet, and the app
does not delete these files by itself.

Treat `credentials.json` as sensitive. It is a local JSON password store.

## Workspace crates

| Crate | Role |
|-------|------|
| `ace-core` | Domain models, connection info, settings |
| `ace-driver` | Driver / connection / dialect traits |
| `ace-mysql` | MySQL and MariaDB driver |
| `ace-postgres` | PostgreSQL driver |
| `ace-sqlite` | SQLite driver |
| `ace-sql` | Split, classify, complete, and format SQL |
| `ace-net` | SSH tunnel, AWS IAM, Vault helpers |
| `ace-io` | Import / export and session formats |
| `ace-bundle` | `.saBundle` runner |
| `ace-mcp` | Local MCP HTTP server |
| `ace-store` | Settings, favorites, credentials, query history |
| `ace-ui` | GPUI desktop UI |
| `ace-app` | Application binary (`ace`) |

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for build conventions, tests, and
pull-request expectations. Bug and feature reports are welcome via GitHub
Issues.

## Security

Please read [SECURITY.md](SECURITY.md) before reporting a vulnerability.
Do not attach real connection secrets to public issues.

## License

Ace is licensed under the [MIT License](LICENSE).

This is an independent remake. It is not affiliated with Sequel Ace, and it
is not a fork of the Sequel Ace source tree. Sequel Ace itself is a separate
project with its own license.

Third-party Rust crates and their licenses are summarized in
[THIRD_PARTY.md](THIRD_PARTY.md), with a generated inventory in
[THIRD_PARTY_LICENSES.txt](THIRD_PARTY_LICENSES.txt).

## Acknowledgements

- [Sequel Ace](https://github.com/Sequel-Ace/Sequel-Ace) for the product
  inspiration and the favorites / session formats Ace aims to be compatible
  with.
- [GPUI](https://github.com/zed-industries/zed) and
  [GPUI Kit](https://crates.io/crates/gpui-kit) for the desktop UI toolkit.
