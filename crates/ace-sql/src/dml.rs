//! Helpers for Content-view row edits (Sequel Ace style UPDATE/INSERT/DELETE).

use ace_driver::dialect::Dialect;
use std::sync::Arc;

pub fn sql_literal(raw: &str, is_null: bool) -> String {
    if is_null || raw.eq_ignore_ascii_case("NULL") && raw.len() == 4 {
        return "NULL".into();
    }
    // Numeric-looking values stay unquoted for convenience; otherwise quote as string.
    if looks_numeric(raw) {
        return raw.to_string();
    }
    format!("'{}'", raw.replace('\'', "''"))
}

fn looks_numeric(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let bytes = s.as_bytes();
    let start = if bytes[0] == b'-' {
        if bytes.len() == 1 {
            return false;
        }
        1
    } else {
        0
    };
    let rest = &bytes[start..];
    // Preserve leading-zero codes like "00123" as strings (only allow "0" or "0.xxx")
    if rest.len() > 1 && rest[0] == b'0' && rest[1] != b'.' {
        return false;
    }
    let mut seen_dot = false;
    let mut seen_digit = false;
    for &b in rest {
        if b == b'.' {
            if seen_dot {
                return false;
            }
            seen_dot = true;
        } else if b.is_ascii_digit() {
            seen_digit = true;
        } else {
            return false;
        }
    }
    seen_digit
}

pub fn build_update_row(
    dialect: &Arc<dyn Dialect>,
    database: &str,
    table: &str,
    set_column: &str,
    new_value: &str,
    set_null: bool,
    pk_columns: &[(String, String, bool)],
    fallback_all_columns: &[(String, String, bool)],
) -> Result<String, String> {
    let target = dialect.qualified_name(Some(database), table);
    let set = format!(
        "{} = {}",
        dialect.quote_ident(set_column),
        sql_literal(new_value, set_null)
    );
    let (keys, add_limit) = if !pk_columns.is_empty() {
        (pk_columns, false)
    } else if !fallback_all_columns.is_empty() {
        (fallback_all_columns, dialect.engine() == ace_core::EngineKind::Mysql || dialect.engine() == ace_core::EngineKind::MariaDb)
    } else {
        return Err("Cannot update: unable to identify target row".into());
    };
    let where_parts: Vec<String> = keys
        .iter()
        .map(|(col, val, is_null)| {
            if *is_null {
                format!("{} IS NULL", dialect.quote_ident(col))
            } else {
                format!(
                    "{} = {}",
                    dialect.quote_ident(col),
                    sql_literal(val, false)
                )
            }
        })
        .collect();
    let limit_suffix = if add_limit { " LIMIT 1" } else { "" };
    Ok(format!(
        "UPDATE {target} SET {set} WHERE {}{limit_suffix}",
        where_parts.join(" AND ")
    ))
}

pub fn build_delete_row(
    dialect: &Arc<dyn Dialect>,
    database: &str,
    table: &str,
    pk_columns: &[(String, String, bool)],
    fallback_all_columns: &[(String, String, bool)],
) -> Result<String, String> {
    let target = dialect.qualified_name(Some(database), table);
    let (keys, add_limit) = if !pk_columns.is_empty() {
        (pk_columns, false)
    } else if !fallback_all_columns.is_empty() {
        (fallback_all_columns, dialect.engine() == ace_core::EngineKind::Mysql || dialect.engine() == ace_core::EngineKind::MariaDb)
    } else {
        return Err("Cannot delete: unable to identify target row".into());
    };
    let where_parts: Vec<String> = keys
        .iter()
        .map(|(col, val, is_null)| {
            if *is_null {
                format!("{} IS NULL", dialect.quote_ident(col))
            } else {
                format!(
                    "{} = {}",
                    dialect.quote_ident(col),
                    sql_literal(val, false)
                )
            }
        })
        .collect();
    let limit_suffix = if add_limit { " LIMIT 1" } else { "" };
    Ok(format!(
        "DELETE FROM {target} WHERE {}{limit_suffix}",
        where_parts.join(" AND ")
    ))
}

pub fn build_insert_typed(
    dialect: &Arc<dyn Dialect>,
    database: &str,
    table: &str,
    entries: &[(String, String, bool)],
) -> Result<String, String> {
    if entries.is_empty() {
        return Err("No columns specified for insert".into());
    }
    let target = dialect.qualified_name(Some(database), table);
    let cols = entries
        .iter()
        .map(|(c, _, _)| dialect.quote_ident(c))
        .collect::<Vec<_>>()
        .join(", ");
    let vals = entries
        .iter()
        .map(|(_, v, is_null)| sql_literal(v, *is_null))
        .collect::<Vec<_>>()
        .join(", ");
    Ok(format!("INSERT INTO {target} ({cols}) VALUES ({vals})"))
}

pub fn build_update(
    dialect: &Arc<dyn Dialect>,
    database: &str,
    table: &str,
    set_column: &str,
    new_value: &str,
    set_null: bool,
    pk_columns: &[(String, String)],
) -> Result<String, String> {
    if pk_columns.is_empty() {
        return Err("Cannot update: table has no primary key".into());
    }
    let target = dialect.qualified_name(Some(database), table);
    let set = format!(
        "{} = {}",
        dialect.quote_ident(set_column),
        sql_literal(new_value, set_null)
    );
    let where_parts: Vec<String> = pk_columns
        .iter()
        .map(|(col, val)| {
            format!(
                "{} = {}",
                dialect.quote_ident(col),
                sql_literal(val, val.eq_ignore_ascii_case("NULL") || val.is_empty())
            )
        })
        .collect();
    Ok(format!(
        "UPDATE {target} SET {set} WHERE {}",
        where_parts.join(" AND ")
    ))
}

pub fn build_delete(
    dialect: &Arc<dyn Dialect>,
    database: &str,
    table: &str,
    pk_columns: &[(String, String)],
) -> Result<String, String> {
    if pk_columns.is_empty() {
        return Err("Cannot delete: table has no primary key".into());
    }
    let target = dialect.qualified_name(Some(database), table);
    let where_parts: Vec<String> = pk_columns
        .iter()
        .map(|(col, val)| {
            format!(
                "{} = {}",
                dialect.quote_ident(col),
                sql_literal(val, val.eq_ignore_ascii_case("NULL") || val.is_empty())
            )
        })
        .collect();
    Ok(format!(
        "DELETE FROM {target} WHERE {}",
        where_parts.join(" AND ")
    ))
}

pub fn build_insert(
    dialect: &Arc<dyn Dialect>,
    database: &str,
    table: &str,
    columns: &[String],
    values: &[String],
) -> Result<String, String> {
    if columns.is_empty() || columns.len() != values.len() {
        return Err("Column/value count mismatch".into());
    }
    let target = dialect.qualified_name(Some(database), table);
    let cols = columns
        .iter()
        .map(|c| dialect.quote_ident(c))
        .collect::<Vec<_>>()
        .join(", ");
    let vals = values
        .iter()
        .map(|v| sql_literal(v, v.is_empty() || v.eq_ignore_ascii_case("NULL")))
        .collect::<Vec<_>>()
        .join(", ");
    Ok(format!("INSERT INTO {target} ({cols}) VALUES ({vals})"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ace_driver::dialect::MysqlDialect;
    use std::sync::Arc;

    #[test]
    fn update_sql() {
        let d: Arc<dyn Dialect> = Arc::new(MysqlDialect);
        let sql = build_update(
            &d,
            "app",
            "users",
            "name",
            "Ada",
            false,
            &[("id".into(), "1".into())],
        )
        .unwrap();
        assert_eq!(sql, "UPDATE `app`.`users` SET `name` = 'Ada' WHERE `id` = 1");
    }

    #[test]
    fn preserves_leading_zero_strings() {
        assert_eq!(sql_literal("00123", false), "'00123'");
        assert_eq!(sql_literal("-", false), "'-'");
        assert_eq!(sql_literal("0", false), "0");
        assert_eq!(sql_literal("0.5", false), "0.5");
        assert_eq!(sql_literal("-42", false), "-42");
    }

    #[test]
    fn update_and_delete_fallback_without_pk() {
        let d: Arc<dyn Dialect> = Arc::new(MysqlDialect);
        let all = vec![
            ("name".into(), "Ada".into(), false),
            ("bio".into(), String::new(), true),
        ];
        let u = build_update_row(&d, "app", "notes", "name", "Grace", false, &[], &all).unwrap();
        assert_eq!(
            u,
            "UPDATE `app`.`notes` SET `name` = 'Grace' WHERE `name` = 'Ada' AND `bio` IS NULL LIMIT 1"
        );
        let del = build_delete_row(&d, "app", "notes", &[], &all).unwrap();
        assert_eq!(
            del,
            "DELETE FROM `app`.`notes` WHERE `name` = 'Ada' AND `bio` IS NULL LIMIT 1"
        );
    }
}
