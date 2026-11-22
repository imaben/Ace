# Ace

Cross-platform MySQL / MariaDB (and later PostgreSQL / SQLite) database management tool —
a Rust remake of [Sequel Ace](https://github.com/Sequel-Ace/Sequel-Ace).

## Features (roadmap)

- Connection favorites, SSH tunnels, SSL/TLS, AWS RDS IAM, Vault
- Multi-window / multi-tab sessions
- Structure, Content, Relations, Triggers, Table Info, Custom Query views
- Import / export (SQL, CSV, XML, HTML, PDF, Dot)
- Bundle extensions, MCP server for AI agents
- macOS / Windows / Linux

## Build

```bash
cargo build -p ace-app
cargo run -p ace-app
```

## Workspace crates

| Crate | Role |
|-------|------|
| `ace-core` | Domain models, settings, events |
| `ace-driver` | Driver / Connection / Dialect traits |
| `ace-mysql` | MySQL & MariaDB driver |
| `ace-postgres` | PostgreSQL driver (phase 7) |
| `ace-sqlite` | SQLite driver (phase 7) |
| `ace-sql` | Split, classify, complete, format SQL |
| `ace-net` | SSH tunnel, TLS helpers, cloud auth |
| `ace-io` | Import / export, `.spf` sessions |
| `ace-bundle` | `.saBundle` runner |
| `ace-mcp` | Local MCP HTTP server |
| `ace-store` | Keyring, local SQLite history |
| `ace-ui` | UI components (GPUI-ready shell) |
| `ace-app` | Application binary |

## License

MIT
