# Ace Architecture

```
ace-app  →  ace-ui (view-models + GPUI shell hook)
         →  ace-mcp / ace-bundle / ace-io / ace-store / ace-net
         →  ace-driver ← ace-mysql | ace-postgres | ace-sqlite
         →  ace-sql / ace-core
```

## Session model

Each workspace tab owns a `SessionTab` (UI) backed by `Arc<dyn Connection>`.
All DB I/O runs on Tokio; the shell / future GPUI layer only mutates view state.

## Driver contract

`ace-driver::Connection` covers:

- metadata (databases, tables, columns, indexes, FKs, triggers)
- execute / stream / cancel
- process list, variables, users
- capability flags via `Dialect`

UI never concatenates SQL; it asks `Dialect` or `ace-sql::ddl`.

## Compatibility

- Favorites JSON (native) + Sequel Ace/Pro plist import (`ace-io`)
- Session `.ace.json` + `.spf` import
- Bundle `.saBundle` / `command.plist`
- Editor themes (TOML + `.spTheme` stub)

## Current UI surface

Interactive CLI shell (`ace connect <url>`) drives the same `AppState` as the
planned GPUI window. See `ace-ui::gpui_shell`.
