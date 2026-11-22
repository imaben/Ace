# Third-party licenses

Ace itself is licensed under the MIT License in [`LICENSE`](LICENSE). That
matches the `license = "MIT"` field already declared in the workspace
`Cargo.toml`. This review did **not** relicense the project.

## How to regenerate the inventory

From the repository root, with the same Rust toolchain the project pins:

```bash
python scripts/list-third-party-licenses.py > THIRD_PARTY_LICENSES.txt
```

The script reads `cargo metadata --format-version 1 --offline` (it will retry
without `--offline` if the local metadata cache is incomplete) and lists
every package in the resolved dependency graph: name, version, license
expression from crate metadata, and repository URL when present.

`THIRD_PARTY_LICENSES.txt` is a snapshot. Re-run the script after changing
dependencies. The script reports crate metadata; it does not vendor the full
text of every upstream `LICENSE` file.

## Review notes (current tree)

A `cargo metadata` pass over this tree found on the order of 900 packages
in the resolved graph (including platform-specific and duplicate versions).
License expressions were overwhelmingly MIT, Apache-2.0, or typical
permissive combinations (BSD, ISC, Zlib, Unicode-3.0, CC0, and similar).

Notable exceptions and compatibility notes:

- **GPUI Kit 0.10.0** and the related `gpui-*` crates are Apache-2.0, which
  is compatible with distributing Ace under MIT.
- **MPL-2.0** crates currently in the graph include `cbindgen`, `dwrote`,
  and `option-ext`. They are used as unmodified dependencies. MPL is a
  file-level weak copyleft; if you modify those crates in-tree, follow the
  MPL terms.
- **`self_cell`** is dual-licensed `Apache-2.0 OR GPL-2.0-only`. Ace relies
  on the Apache-2.0 option and does not require the project to become GPL.
- **`webpki` 0.22.4** currently exposes `license-file` metadata rather than a
  SPDX `license` field, so the generated inventory may show it as unknown.
  Consult that crate's own license file; do not guess a different license
  for Ace.
- **`r-efi`** is `MIT OR Apache-2.0 OR LGPL-2.1-or-later`. The MIT or
  Apache-2.0 option can be selected.

No in-repository font files were found under `assets/`. UI fonts and other
toolkit assets come from `gpui-kit` / `gpui-kit-assets` and should be
attributed according to those crates.

Application icons under `assets/icons/` and the small theme / sample bundle
under `assets/themes/` and `assets/bundles/` appear, from the current tree
and git history, to be project files rather than vendored Sequel Ace
sources. The sample `Format SQL.saBundle` is a minimal plist (`cat` as the
command). This is not a legal opinion, and icon authorship was not verified
outside this repository.

## Sequel Ace and format compatibility

[Sequel Ace](https://github.com/Sequel-Ace/Sequel-Ace) is a separate project
licensed under the MIT License, including copyright notices for Moballo, LLC
(2020) and the Sequel Pro & CocoaMySQL Teams (2002-2019). See the official
file: https://raw.githubusercontent.com/Sequel-Ace/Sequel-Ace/main/LICENSE

Ace is an independent Rust remake, not a fork of that repository.
Compatibility with favorites plists and related file formats does not by
itself copy Sequel Ace's source.

If source or assets under an incompatible license are later vendored into
this tree, the MIT declaration must be revisited. **Do not silently
relicense Ace** to "fix" such a conflict; document the finding and decide
explicitly.

## Limitations of this review

- Based on crate metadata and files in the current tree plus reachable git
  history of this repository, not a full reading of every upstream license
  file or font notice inside compiled assets.
- Transitive system libraries (for example MSVC, Windows SDK, Xcode, or
  Linux distribution packages) are not listed here.
- Historical commits were not rewritten. License metadata can change when
  dependencies are upgraded.
