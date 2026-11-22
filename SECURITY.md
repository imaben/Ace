# Security Policy

## Supported versions

Ace is in early development (version `0.1.0` in the tree, with no tagged
GitHub Release at the time this policy was written). Security fixes will be
applied on the default branch. There is no separate long-term support release.

## Reporting a vulnerability

Please do **not** open a public issue for a vulnerability, and do not include
database passwords, connection strings, private keys, tokens, or dumps of
local config files in any report.

Use GitHub's private vulnerability reporting if it is enabled for this
repository:

https://github.com/imaben/Ace/security/advisories/new

If that form is not available, open a public issue that only says you want to
report a security issue and wait for a maintainer to share a private channel.
Do not attach exploits or sensitive data to that issue.

There is no separate security email for this project. Please do not send
credentials or production data to personal git-commit addresses.

## Local data you should treat as sensitive

Ace stores user data outside the application binary. On a typical install
these files can contain connection details or passwords:

- `settings.toml` — app preferences; may include an optional MCP auth token
- `favorites.json` — saved connections (passwords are intended to be stored
  separately)
- `credentials.json` — saved passwords in a local JSON file (Unix file mode
  `0600` is applied when supported; Windows ACLs are not specially hardened)
- `history.sqlite3` — query history

See the README for the exact directories on each platform. When filing any
report, redact these files.

## Scope notes

- `ace update-check` only queries the GitHub Releases API. It does not
  download or install updates.
- The CI workflow uses a local MySQL service with a well-known test password
  (`root` / `root`) bound to the runner. That value is not a production
  secret.
