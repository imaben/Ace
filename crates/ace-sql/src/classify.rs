use ace_driver::query::StatementKind;
use regex::Regex;
use std::sync::OnceLock;

fn first_keyword(sql: &str) -> String {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r"(?is)^\s*(?:/\*.*?\*/\s*|--[^\n]*\n\s*|#.*\n\s*)*([a-zA-Z]+)").unwrap()
    });
    re.captures(sql)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_uppercase())
        .unwrap_or_default()
}

pub fn classify_sql(sql: &str) -> StatementKind {
    match first_keyword(sql).as_str() {
        "SELECT" | "WITH" | "TABLE" | "VALUES" => StatementKind::Select,
        "INSERT" => StatementKind::Insert,
        "UPDATE" => StatementKind::Update,
        "DELETE" => StatementKind::Delete,
        "REPLACE" => StatementKind::Replace,
        "CREATE" => StatementKind::Create,
        "ALTER" => StatementKind::Alter,
        "DROP" => StatementKind::Drop,
        "TRUNCATE" => StatementKind::Truncate,
        "SHOW" => StatementKind::Show,
        "DESCRIBE" | "DESC" => StatementKind::Describe,
        "EXPLAIN" | "ANALYZE" => StatementKind::Explain,
        "USE" => StatementKind::Use,
        "SET" => StatementKind::Set,
        "CALL" => StatementKind::Call,
        "BEGIN" | "START" | "COMMIT" | "ROLLBACK" | "SAVEPOINT" => StatementKind::Transaction,
        _ => StatementKind::Other,
    }
}

/// Heuristic: a SELECT that references exactly one base table (no JOIN) may be editable.
pub fn is_single_table_select(sql: &str) -> Option<String> {
    let kind = classify_sql(sql);
    if kind != StatementKind::Select {
        return None;
    }
    let upper = sql.to_uppercase();
    if upper.contains(" JOIN ") || upper.contains(" UNION ") {
        return None;
    }
    static RE: OnceLock<Regex> = OnceLock::new();
    let re =
        RE.get_or_init(|| Regex::new(r#"(?is)\bFROM\s+[`"\[]?([a-zA-Z0-9_$.]+)[`"\]]?"#).unwrap());
    re.captures(sql)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

pub fn is_read_only(sql: &str) -> bool {
    !classify_sql(sql).is_mutating()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_select() {
        assert_eq!(classify_sql("  select * from t"), StatementKind::Select);
        assert_eq!(
            classify_sql("WITH x AS (SELECT 1) SELECT * FROM x"),
            StatementKind::Select
        );
    }

    #[test]
    fn single_table() {
        assert_eq!(
            is_single_table_select("SELECT * FROM users").as_deref(),
            Some("users")
        );
        assert!(is_single_table_select("SELECT * FROM a JOIN b ON 1=1").is_none());
        assert!(is_single_table_select("SELECT * FROM a UNION SELECT * FROM b").is_none());
        assert!(is_single_table_select("DELETE FROM users").is_none());
    }

    #[test]
    fn classify_skips_leading_comments_and_covers_kinds() {
        assert_eq!(
            classify_sql("/* note */\nDELETE FROM t"),
            StatementKind::Delete
        );
        assert_eq!(classify_sql("EXPLAIN SELECT 1"), StatementKind::Explain);
        assert_eq!(classify_sql("USE app"), StatementKind::Use);
        assert_eq!(classify_sql("BEGIN"), StatementKind::Transaction);
        assert_eq!(classify_sql("SHOW TABLES"), StatementKind::Show);
        assert!(is_read_only("SELECT 1"));
        assert!(!is_read_only("UPDATE t SET a = 1"));
        assert_eq!(classify_sql(""), StatementKind::Other);
    }
}
