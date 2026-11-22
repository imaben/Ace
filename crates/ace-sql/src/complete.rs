use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompletionKind {
    Keyword,
    Database,
    Table,
    Column,
    Function,
    Snippet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionKind,
    pub detail: Option<String>,
    pub insert: String,
}

#[derive(Debug, Clone, Default)]
pub struct CompletionContext {
    pub databases: Vec<String>,
    pub tables: Vec<String>,
    pub columns: Vec<(String, String)>, // (table, column)
}

const KEYWORDS: &[&str] = &[
    "SELECT", "FROM", "WHERE", "AND", "OR", "NOT", "IN", "LIKE", "BETWEEN", "IS", "NULL",
    "ORDER", "BY", "GROUP", "HAVING", "LIMIT", "OFFSET", "JOIN", "LEFT", "RIGHT", "INNER",
    "OUTER", "ON", "AS", "DISTINCT", "INSERT", "INTO", "VALUES", "UPDATE", "SET", "DELETE",
    "CREATE", "TABLE", "INDEX", "VIEW", "DROP", "ALTER", "SHOW", "DESCRIBE", "EXPLAIN",
    "USE", "DATABASE", "SCHEMA", "PRIMARY", "KEY", "FOREIGN", "REFERENCES", "CASCADE",
    "CONSTRAINT", "UNIQUE", "DEFAULT", "AUTO_INCREMENT", "ENGINE", "CHARSET", "COLLATE",
    "BEGIN", "COMMIT", "ROLLBACK", "TRUNCATE", "REPLACE", "CALL", "WITH", "UNION", "ALL",
    "EXISTS", "CASE", "WHEN", "THEN", "ELSE", "END", "ASC", "DESC",
];

const FUNCTIONS: &[&str] = &[
    "COUNT", "SUM", "AVG", "MIN", "MAX", "NOW", "CURDATE", "CURTIME", "CONCAT", "IFNULL",
    "COALESCE", "IF", "NULLIF", "CAST", "CONVERT", "SUBSTRING", "LENGTH", "TRIM", "UPPER",
    "LOWER", "DATE_FORMAT", "UNIX_TIMESTAMP", "FROM_UNIXTIME", "JSON_EXTRACT", "GROUP_CONCAT",
];

pub fn complete(prefix: &str, ctx: &CompletionContext) -> Vec<CompletionItem> {
    let prefix_lower = prefix.to_lowercase();
    let mut items = Vec::new();

    for kw in KEYWORDS {
        if kw.to_lowercase().starts_with(&prefix_lower) {
            items.push(CompletionItem {
                label: kw.to_string(),
                kind: CompletionKind::Keyword,
                detail: Some("keyword".into()),
                insert: kw.to_string(),
            });
        }
    }
    for f in FUNCTIONS {
        if f.to_lowercase().starts_with(&prefix_lower) {
            items.push(CompletionItem {
                label: format!("{f}()"),
                kind: CompletionKind::Function,
                detail: Some("function".into()),
                insert: format!("{f}("),
            });
        }
    }
    for db in &ctx.databases {
        if db.to_lowercase().starts_with(&prefix_lower) {
            items.push(CompletionItem {
                label: db.clone(),
                kind: CompletionKind::Database,
                detail: Some("database".into()),
                insert: format!("`{db}`"),
            });
        }
    }
    for t in &ctx.tables {
        if t.to_lowercase().starts_with(&prefix_lower) {
            items.push(CompletionItem {
                label: t.clone(),
                kind: CompletionKind::Table,
                detail: Some("table".into()),
                insert: format!("`{t}`"),
            });
        }
    }
    for (table, col) in &ctx.columns {
        if col.to_lowercase().starts_with(&prefix_lower)
            || format!("{table}.{col}").to_lowercase().starts_with(&prefix_lower)
        {
            items.push(CompletionItem {
                label: col.clone(),
                kind: CompletionKind::Column,
                detail: Some(format!("column of {table}")),
                insert: format!("`{col}`"),
            });
        }
    }

    items.sort_by(|a, b| a.label.len().cmp(&b.label.len()).then(a.label.cmp(&b.label)));
    items.truncate(50);
    items
}

/// Extract the word being typed at `cursor` in `sql`.
pub fn word_at_cursor(sql: &str, cursor: usize) -> (usize, String) {
    let cursor = cursor.min(sql.len());
    let bytes = sql.as_bytes();
    let mut start = cursor;
    while start > 0 {
        let c = bytes[start - 1] as char;
        if c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '$' {
            start -= 1;
        } else {
            break;
        }
    }
    (start, sql[start..cursor].to_string())
}