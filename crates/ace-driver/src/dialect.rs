use crate::capability::Capabilities;
use ace_core::{EngineKind, Value};

/// SQL dialect helpers — UI never concatenates SQL by hand.
pub trait Dialect: Send + Sync {
    fn engine(&self) -> EngineKind;
    fn capabilities(&self) -> Capabilities;
    fn quote_ident(&self, name: &str) -> String;
    fn quote_string(&self, value: &str) -> String;

    fn format_value(&self, value: &Value) -> String {
        value.to_sql_literal()
    }

    fn qualified_name(&self, database: Option<&str>, table: &str) -> String {
        match database {
            Some(db) if !db.is_empty() => {
                format!("{}.{}", self.quote_ident(db), self.quote_ident(table))
            }
            _ => self.quote_ident(table),
        }
    }

    fn limit_offset(&self, limit: usize, offset: usize) -> String {
        format!("LIMIT {limit} OFFSET {offset}")
    }

    fn show_databases_sql(&self) -> String;
    fn show_tables_sql(&self, database: &str) -> String;
    fn describe_table_sql(&self, database: &str, table: &str) -> String;
    fn show_create_table_sql(&self, database: &str, table: &str) -> String;
    fn select_all_sql(&self, database: &str, table: &str, where_clause: Option<&str>) -> String {
        let mut sql = format!(
            "SELECT * FROM {}",
            self.qualified_name(Some(database), table)
        );
        if let Some(w) = where_clause {
            if !w.trim().is_empty() {
                sql.push_str(" WHERE ");
                sql.push_str(w);
            }
        }
        sql
    }

    fn count_sql(&self, database: &str, table: &str, where_clause: Option<&str>) -> String {
        let mut sql = format!(
            "SELECT COUNT(*) AS cnt FROM {}",
            self.qualified_name(Some(database), table)
        );
        if let Some(w) = where_clause {
            if !w.trim().is_empty() {
                sql.push_str(" WHERE ");
                sql.push_str(w);
            }
        }
        sql
    }

    fn explain_sql(&self, query: &str) -> String {
        format!("EXPLAIN {query}")
    }

    fn kill_query_sql(&self, process_id: u64) -> String {
        format!("KILL QUERY {process_id}")
    }

    fn process_list_sql(&self) -> String {
        "SHOW FULL PROCESSLIST".into()
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct MysqlDialect;

impl Dialect for MysqlDialect {
    fn engine(&self) -> EngineKind {
        EngineKind::Mysql
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::mysql_full()
    }

    fn quote_ident(&self, name: &str) -> String {
        format!("`{}`", name.replace('`', "``"))
    }

    fn quote_string(&self, value: &str) -> String {
        format!(
            "'{}'",
            value.replace('\\', "\\\\").replace('\'', "''")
        )
    }

    fn show_databases_sql(&self) -> String {
        "SHOW DATABASES".into()
    }

    fn show_tables_sql(&self, database: &str) -> String {
        format!(
            "SHOW FULL TABLES FROM {}",
            self.quote_ident(database)
        )
    }

    fn describe_table_sql(&self, database: &str, table: &str) -> String {
        format!(
            "SHOW FULL COLUMNS FROM {}",
            self.qualified_name(Some(database), table)
        )
    }

    fn show_create_table_sql(&self, database: &str, table: &str) -> String {
        format!(
            "SHOW CREATE TABLE {}",
            self.qualified_name(Some(database), table)
        )
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct PostgresDialect;

impl Dialect for PostgresDialect {
    fn engine(&self) -> EngineKind {
        EngineKind::Postgres
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::postgres_full()
    }

    fn quote_ident(&self, name: &str) -> String {
        format!("\"{}\"", name.replace('"', "\"\""))
    }

    fn quote_string(&self, value: &str) -> String {
        format!("'{}'", value.replace('\'', "''"))
    }

    fn show_databases_sql(&self) -> String {
        "SELECT datname FROM pg_database WHERE datistemplate = false ORDER BY 1".into()
    }

    fn show_tables_sql(&self, schema: &str) -> String {
        format!(
            "SELECT table_name, table_type FROM information_schema.tables \
             WHERE table_schema = {} ORDER BY 1",
            self.quote_string(schema)
        )
    }

    fn describe_table_sql(&self, schema: &str, table: &str) -> String {
        format!(
            "SELECT column_name, data_type, is_nullable, column_default \
             FROM information_schema.columns \
             WHERE table_schema = {} AND table_name = {} ORDER BY ordinal_position",
            self.quote_string(schema),
            self.quote_string(table)
        )
    }

    fn show_create_table_sql(&self, schema: &str, table: &str) -> String {
        // PostgreSQL has no SHOW CREATE; callers use pg_dump-style reconstruction.
        format!(
            "SELECT '/* use information_schema for {}.{} */'",
            schema, table
        )
    }

    fn process_list_sql(&self) -> String {
        "SELECT pid, usename, client_addr::text, datname, state, query, \
         EXTRACT(EPOCH FROM (now() - query_start))::bigint AS time \
         FROM pg_stat_activity"
            .into()
    }

    fn kill_query_sql(&self, process_id: u64) -> String {
        format!("SELECT pg_cancel_backend({process_id})")
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SqliteDialect;

impl Dialect for SqliteDialect {
    fn engine(&self) -> EngineKind {
        EngineKind::Sqlite
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::sqlite_full()
    }

    fn quote_ident(&self, name: &str) -> String {
        format!("\"{}\"", name.replace('"', "\"\""))
    }

    fn quote_string(&self, value: &str) -> String {
        format!("'{}'", value.replace('\'', "''"))
    }

    fn qualified_name(&self, _database: Option<&str>, table: &str) -> String {
        // SQLite attachment names like "main" are fine; file paths must never be
        // interpolated as schema qualifiers.
        self.quote_ident(table)
    }

    fn show_databases_sql(&self) -> String {
        "PRAGMA database_list".into()
    }

    fn show_tables_sql(&self, _database: &str) -> String {
        "SELECT name, type FROM sqlite_master WHERE type IN ('table','view') \
         AND name NOT LIKE 'sqlite_%' ORDER BY name"
            .into()
    }

    fn describe_table_sql(&self, _database: &str, table: &str) -> String {
        format!("PRAGMA table_info({})", self.quote_ident(table))
    }

    fn show_create_table_sql(&self, _database: &str, table: &str) -> String {
        format!(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name={}",
            self.quote_string(table)
        )
    }
}