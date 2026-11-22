//! Context-aware SQL autocomplete provider for GPUI EditorState.
//!
//! Provides live suggestions for tables, views, columns (including `table.col` and `alias.col`),
//! databases, SQL keywords, and built-in functions using the session's cached `SchemaCatalog`.

use ace_driver::meta::{ColumnMeta, DatabaseObject, ObjectKind};
use gpui_kit::component::input::{CompletionProvider, Rope, RopeExt as _};
use gpui_kit::{App, AppContext as _, Task, Window};
use lsp_types::{
    CompletionContext, CompletionItem, CompletionItemKind, CompletionResponse, CompletionTextEdit,
    CompletionTriggerKind, TextEdit,
};
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ColumnHint {
    pub name: String,
    pub data_type: String,
    pub primary_key: bool,
    pub nullable: bool,
}

impl From<&ColumnMeta> for ColumnHint {
    fn from(c: &ColumnMeta) -> Self {
        Self {
            name: c.name.clone(),
            data_type: if c.full_type.is_empty() {
                c.data_type.clone()
            } else {
                c.full_type.clone()
            },
            primary_key: c.primary_key,
            nullable: c.nullable,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SchemaSnapshot {
    pub current_db: String,
    pub databases: Vec<String>,
    pub tables: Vec<(String, ObjectKind)>,
    /// table_name -> columns
    pub columns: HashMap<String, Vec<ColumnHint>>,
}

#[derive(Clone, Default)]
pub struct SchemaCatalog {
    inner: Arc<RwLock<SchemaSnapshot>>,
}

impl SchemaCatalog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_databases(&self, databases: Vec<String>) {
        self.inner.write().databases = databases;
    }

    pub fn set_objects(&self, current_db: String, objects: &[DatabaseObject]) {
        let mut g = self.inner.write();
        if g.current_db != current_db {
            g.columns.clear();
        }
        g.current_db = current_db;
        g.tables = objects.iter().map(|o| (o.name.clone(), o.kind)).collect();
    }

    pub fn set_all_columns(&self, columns: HashMap<String, Vec<ColumnHint>>) {
        self.inner.write().columns = columns;
    }

    pub fn set_table_columns(&self, table: &str, cols: Vec<ColumnHint>) {
        self.inner.write().columns.insert(table.to_string(), cols);
    }

    pub fn snapshot(&self) -> SchemaSnapshot {
        self.inner.read().clone()
    }
}

pub struct SqlCompletionProvider {
    catalog: SchemaCatalog,
}

impl SqlCompletionProvider {
    pub fn new(catalog: SchemaCatalog) -> Self {
        Self { catalog }
    }
}

impl CompletionProvider for SqlCompletionProvider {
    fn completions(
        &self,
        rope: &Rope,
        offset: usize,
        trigger: CompletionContext,
        _window: &mut Window,
        cx: &mut App,
    ) -> Task<anyhow::Result<CompletionResponse>> {
        let full_text = rope.to_string();
        let offset = offset.min(full_text.len());
        let explicit = trigger.trigger_kind == CompletionTriggerKind::INVOKED;

        let (frag_start, qualifier, fragment) = extract_token_context(&full_text, offset);

        // Don't pop up noisily on empty fragment unless after `.` or explicitly invoked (Ctrl+Space)
        if qualifier.is_none() && fragment.is_empty() && !explicit {
            return Task::ready(Ok(CompletionResponse::Array(Vec::new())));
        }

        let start_pos = rope.offset_to_position(frag_start);
        let end_pos = rope.offset_to_position(offset);
        let replace_range = lsp_types::Range {
            start: start_pos,
            end: end_pos,
        };
        let snapshot = self.catalog.snapshot();

        cx.background_spawn(async move {
            let items = compute_completions(
                &full_text,
                offset,
                frag_start,
                qualifier.as_deref(),
                &fragment,
                replace_range,
                &snapshot,
            );
            Ok(CompletionResponse::Array(items))
        })
    }

    fn is_completion_trigger(&self, _offset: usize, new_text: &str, _cx: &mut App) -> bool {
        if new_text.is_empty() {
            return false;
        }
        new_text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '`')
    }
}

/// Extract `(fragment_start_byte, optional_qualifier, fragment)` at `offset`.
/// Example: `SELECT u.em|` -> `(frag_start at 'e', Some("u"), "em")`
/// Example: `SELECT us|` -> `(frag_start at 'u', None, "us")`
/// Example: `SELECT users.|` -> `(frag_start at offset, Some("users"), "")`
fn extract_token_context(sql: &str, offset: usize) -> (usize, Option<String>, String) {
    let bytes = sql.as_bytes();
    let mut i = offset;
    while i > 0 {
        let b = bytes[i - 1];
        if b.is_ascii_alphanumeric() || b == b'_' || b == b'$' {
            i -= 1;
        } else {
            break;
        }
    }
    let frag_start = i;
    let fragment = sql[frag_start..offset].to_string();

    // Check if preceded by a dot (possibly with a backtick before fragment)
    let mut dot_pos = frag_start;
    if dot_pos > 0 && bytes[dot_pos - 1] == b'`' {
        dot_pos -= 1;
    }
    if dot_pos > 0 && bytes[dot_pos - 1] == b'.' {
        let mut q_end = dot_pos - 1;
        if q_end > 0 && bytes[q_end - 1] == b'`' {
            q_end -= 1;
        }
        let mut q_start = q_end;
        while q_start > 0 {
            let b = bytes[q_start - 1];
            if b.is_ascii_alphanumeric() || b == b'_' || b == b'$' {
                q_start -= 1;
            } else {
                break;
            }
        }
        if q_start < q_end {
            let qualifier = sql[q_start..q_end].to_string();
            return (frag_start, Some(qualifier), fragment);
        }
    }

    (frag_start, None, fragment)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SqlClauseContext {
    TablesFirst,
    ColumnsFirst,
    DatabasesFirst,
    Mixed,
}

fn detect_clause_context(sql: &str, frag_start: usize) -> SqlClauseContext {
    let before = &sql[..frag_start.min(sql.len())];
    // Scan backwards for the nearest meaningful SQL word
    let tokens: Vec<String> = tokenize_words(before);
    let last = tokens.last().map(|s| s.as_str()).unwrap_or("");
    let prev2 = if tokens.len() >= 2 {
        tokens[tokens.len() - 2].as_str()
    } else {
        ""
    };

    match last {
        "FROM" | "JOIN" | "INTO" | "UPDATE" | "TABLE" | "TRUNCATE" | "DESCRIBE" | "DESC" => {
            SqlClauseContext::TablesFirst
        }
        "USE" | "DATABASE" | "SCHEMA" => SqlClauseContext::DatabasesFirst,
        "SELECT" | "WHERE" | "AND" | "OR" | "ON" | "SET" | "HAVING" | "DISTINCT" => {
            SqlClauseContext::ColumnsFirst
        }
        "BY" if prev2 == "ORDER" || prev2 == "GROUP" => SqlClauseContext::ColumnsFirst,
        _ => {
            // Check if the last non-whitespace char is `,` or `(` or operator
            let last_char = before.trim_end().chars().last();
            if matches!(last_char, Some(',' | '=' | '<' | '>' | '+' | '-' | '*' | '/' | '(')) {
                // If after FROM t1, -> still TablesFirst; otherwise ColumnsFirst
                for tok in tokens.iter().rev() {
                    match tok.as_str() {
                        "SELECT" | "WHERE" | "ON" | "SET" | "HAVING" | "ORDER" | "GROUP" => {
                            return SqlClauseContext::ColumnsFirst;
                        }
                        "FROM" | "JOIN" | "UPDATE" | "INTO" => {
                            return if last_char == Some(',') {
                                SqlClauseContext::TablesFirst
                            } else {
                                SqlClauseContext::ColumnsFirst
                            };
                        }
                        _ => {}
                    }
                }
            }
            SqlClauseContext::Mixed
        }
    }
}

/// Parse table names and aliases referenced in the current statement:
/// `FROM users u`, `FROM `users` AS u`, `JOIN orders o`, `UPDATE users`, `INTO users`.
fn extract_referenced_tables(
    stmt: &str,
    known_tables: &[(String, ObjectKind)],
) -> (Vec<String>, HashMap<String, String>) {
    let known_lower: HashMap<String, String> = known_tables
        .iter()
        .map(|(n, _)| (n.to_lowercase(), n.clone()))
        .collect();

    let raw_tokens = split_sql_ident_tokens(stmt);
    let mut tables = Vec::new();
    let mut aliases = HashMap::new();
    let mut i = 0;

    while i < raw_tokens.len() {
        let up = raw_tokens[i].to_ascii_uppercase();
        if matches!(up.as_str(), "FROM" | "JOIN" | "UPDATE" | "INTO") {
            if let Some(tbl_tok) = raw_tokens.get(i + 1) {
                let clean = tbl_tok
                    .rsplit('.')
                    .next()
                    .unwrap_or(tbl_tok)
                    .trim_matches('`')
                    .trim_matches('"');
                if let Some(real_table) = known_lower.get(&clean.to_lowercase()) {
                    if !tables.contains(real_table) {
                        tables.push(real_table.clone());
                    }
                    // Check optional `[AS] <alias>`
                    if let Some(next_tok) = raw_tokens.get(i + 2) {
                        let next_up = next_tok.to_ascii_uppercase();
                        if next_up == "AS" {
                            if let Some(al) = raw_tokens.get(i + 3) {
                                let al_clean = al.trim_matches('`').trim_matches('"');
                                if !is_clause_keyword(&al_clean.to_ascii_uppercase()) {
                                    aliases.insert(al_clean.to_lowercase(), real_table.clone());
                                }
                            }
                        } else if !is_clause_keyword(&next_up)
                            && next_tok
                                .chars()
                                .next()
                                .map(|c| c.is_ascii_alphabetic() || c == '_')
                                .unwrap_or(false)
                        {
                            let al_clean = next_tok.trim_matches('`').trim_matches('"');
                            aliases.insert(al_clean.to_lowercase(), real_table.clone());
                        }
                    }
                }
            }
        }
        i += 1;
    }

    (tables, aliases)
}

fn compute_completions(
    full_sql: &str,
    offset: usize,
    frag_start: usize,
    qualifier: Option<&str>,
    fragment: &str,
    replace_range: lsp_types::Range,
    schema: &SchemaSnapshot,
) -> Vec<CompletionItem> {
    let frag_lower = fragment.to_lowercase();
    let current_stmt = ace_sql::statement_at_cursor(full_sql, offset).unwrap_or_default();
    let (ref_tables, aliases) = extract_referenced_tables(&current_stmt, &schema.tables);

    let mut out: Vec<CompletionItem> = Vec::new();
    let mut seen: HashSet<(String, String)> = HashSet::new();

    let mut push_item = |tier: u8,
                         label: String,
                         insert: String,
                         kind: CompletionItemKind,
                         detail: String| {
        let lbl_lower = label.to_lowercase();
        let match_rank = if frag_lower.is_empty() {
            0u8
        } else if lbl_lower.starts_with(&frag_lower) {
            0u8
        } else if lbl_lower.contains(&frag_lower) {
            1u8
        } else {
            return;
        };

        let dedup_key = (label.to_lowercase(), detail.clone());
        if !seen.insert(dedup_key) {
            return;
        }

        let sort_text = format!("{tier}_{match_rank}_{:03}_{lbl_lower}", label.len().min(999));
        out.push(CompletionItem {
            label: label.clone(),
            kind: Some(kind),
            detail: Some(detail),
            sort_text: Some(sort_text),
            filter_text: Some(label),
            text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                range: replace_range,
                new_text: insert,
            })),
            ..Default::default()
        });
    };

    // Case 1: Qualified dot completion (`u.` / `users.` / `mydb.`)
    if let Some(qual) = qualifier {
        let qual_lower = qual.to_lowercase();

        // Check alias first, then exact table name (case-insensitive)
        let resolved_table = aliases.get(&qual_lower).cloned().or_else(|| {
            schema
                .tables
                .iter()
                .find(|(t, _)| t.to_lowercase() == qual_lower)
                .map(|(t, _)| t.clone())
        });

        if let Some(tbl) = resolved_table {
            if let Some(cols) = schema.columns.get(&tbl) {
                for c in cols {
                    let detail = format_col_detail(None, c);
                    push_item(0, c.name.clone(), c.name.clone(), CompletionItemKind::FIELD, detail);
                }
            }
        } else if schema
            .databases
            .iter()
            .any(|d| d.to_lowercase() == qual_lower)
        {
            for (tbl, kind) in &schema.tables {
                let detail = object_kind_label(*kind).to_string();
                push_item(
                    0,
                    tbl.clone(),
                    tbl.clone(),
                    CompletionItemKind::CLASS,
                    detail,
                );
            }
        }

        out.sort_by(|a, b| a.sort_text.cmp(&b.sort_text));
        out.truncate(60);
        return out;
    }

    // Case 2: Unqualified completion — rank by SQL clause context
    let clause = detect_clause_context(full_sql, frag_start);

    let (t_ref_col, t_table, t_other_col, t_db, t_fn, t_kw) = match clause {
        SqlClauseContext::TablesFirst => (3, 0, 4, 1, 5, 2),
        SqlClauseContext::ColumnsFirst => (0, 2, 1, 5, 3, 4),
        SqlClauseContext::DatabasesFirst => (4, 2, 5, 0, 5, 1),
        SqlClauseContext::Mixed => (0, 1, 2, 4, 3, 3),
    };

    // Columns of tables referenced in current query
    let mut ref_set: HashSet<&str> = HashSet::new();
    for tbl in &ref_tables {
        ref_set.insert(tbl.as_str());
        if let Some(cols) = schema.columns.get(tbl) {
            for c in cols {
                let detail = format_col_detail(Some(tbl), c);
                push_item(
                    t_ref_col,
                    c.name.clone(),
                    c.name.clone(),
                    CompletionItemKind::FIELD,
                    detail,
                );
            }
        }
    }

    // Aliases defined in current query
    for (al, tbl) in &aliases {
        push_item(
            t_ref_col,
            al.clone(),
            al.clone(),
            CompletionItemKind::VARIABLE,
            format!("alias → {tbl}"),
        );
    }

    // Tables & Views in current database
    for (tbl, kind) in &schema.tables {
        let detail = object_kind_label(*kind).to_string();
        push_item(
            t_table,
            tbl.clone(),
            tbl.clone(),
            CompletionItemKind::CLASS,
            detail,
        );
    }

    // Columns of all other tables (when not exclusively in TablesFirst / DatabasesFirst)
    if !matches!(
        clause,
        SqlClauseContext::TablesFirst | SqlClauseContext::DatabasesFirst
    ) {
        for (tbl, cols) in &schema.columns {
            if ref_set.contains(tbl.as_str()) {
                continue;
            }
            for c in cols {
                let detail = format_col_detail(Some(tbl), c);
                push_item(
                    t_other_col,
                    c.name.clone(),
                    c.name.clone(),
                    CompletionItemKind::FIELD,
                    detail,
                );
            }
        }
    }

    // Databases
    for db in &schema.databases {
        push_item(
            t_db,
            db.clone(),
            db.clone(),
            CompletionItemKind::MODULE,
            "database".into(),
        );
    }

    // SQL Functions
    for func in SQL_FUNCTIONS {
        push_item(
            t_fn,
            format!("{func}()"),
            format!("{func}("),
            CompletionItemKind::FUNCTION,
            "function".into(),
        );
    }

    // SQL Keywords
    for kw in SQL_KEYWORDS {
        push_item(
            t_kw,
            (*kw).to_string(),
            (*kw).to_string(),
            CompletionItemKind::KEYWORD,
            "keyword".into(),
        );
    }

    out.sort_by(|a, b| a.sort_text.cmp(&b.sort_text));
    out.truncate(60);
    out
}

fn format_col_detail(table: Option<&str>, c: &ColumnHint) -> String {
    let pk = if c.primary_key { " • PK" } else { "" };
    match table {
        Some(t) if !c.data_type.is_empty() => format!("{t} • {}{pk}", c.data_type),
        Some(t) => format!("{t}{pk}"),
        None if !c.data_type.is_empty() => format!("{}{pk}", c.data_type),
        None => "column".into(),
    }
}

fn object_kind_label(kind: ObjectKind) -> &'static str {
    match kind {
        ObjectKind::Table => "table",
        ObjectKind::View => "view",
        ObjectKind::Procedure => "procedure",
        ObjectKind::Function => "function",
        ObjectKind::Trigger => "trigger",
        ObjectKind::Event => "event",
    }
}

fn tokenize_words(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            cur.push(ch.to_ascii_uppercase());
        } else if !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn split_sql_ident_tokens(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '`' | '"' | '$') {
            cur.push(ch);
        } else if !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn is_clause_keyword(w: &str) -> bool {
    matches!(
        w,
        "WHERE" | "SET" | "JOIN" | "LEFT" | "RIGHT" | "INNER" | "OUTER" | "CROSS" | "FULL"
            | "ON" | "GROUP" | "ORDER" | "HAVING" | "LIMIT" | "OFFSET" | "UNION" | "VALUES"
            | "SELECT" | "FROM" | "INTO" | "AND" | "OR" | "USING"
    )
}

const SQL_KEYWORDS: &[&str] = &[
    "SELECT", "FROM", "WHERE", "AND", "OR", "NOT", "IN", "IS", "NULL", "NOT NULL", "LIKE",
    "BETWEEN", "EXISTS", "AS", "ON", "JOIN", "LEFT JOIN", "RIGHT JOIN", "INNER JOIN",
    "GROUP BY", "ORDER BY", "HAVING", "LIMIT", "OFFSET", "DISTINCT", "ASC", "DESC",
    "INSERT INTO", "VALUES", "UPDATE", "SET", "DELETE FROM", "REPLACE INTO",
    "CREATE TABLE", "ALTER TABLE", "DROP TABLE", "TRUNCATE TABLE", "SHOW TABLES",
    "SHOW DATABASES", "SHOW CREATE TABLE", "DESCRIBE", "EXPLAIN", "USE", "UNION", "UNION ALL",
    "CASE", "WHEN", "THEN", "ELSE", "END", "WITH", "BEGIN", "COMMIT", "ROLLBACK",
];

const SQL_FUNCTIONS: &[&str] = &[
    "COUNT", "SUM", "AVG", "MIN", "MAX", "COALESCE", "IFNULL", "NULLIF", "IF", "CONCAT",
    "CONCAT_WS", "SUBSTRING", "LENGTH", "CHAR_LENGTH", "TRIM", "LOWER", "UPPER", "REPLACE",
    "NOW", "CURDATE", "CURTIME", "DATE_FORMAT", "DATEDIFF", "DATE_ADD", "DATE_SUB",
    "UNIX_TIMESTAMP", "FROM_UNIXTIME", "CAST", "CONVERT", "ROUND", "FLOOR", "CEIL", "ABS",
    "JSON_EXTRACT", "JSON_UNQUOTE", "JSON_OBJECT", "JSON_ARRAY", "GROUP_CONCAT",
    "ROW_NUMBER", "RANK", "DENSE_RANK",
];

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_schema() -> SchemaSnapshot {
        let mut cols = HashMap::new();
        cols.insert(
            "users".into(),
            vec![
                ColumnHint {
                    name: "id".into(),
                    data_type: "bigint".into(),
                    primary_key: true,
                    nullable: false,
                },
                ColumnHint {
                    name: "email".into(),
                    data_type: "varchar(255)".into(),
                    primary_key: false,
                    nullable: true,
                },
                ColumnHint {
                    name: "username".into(),
                    data_type: "varchar(64)".into(),
                    primary_key: false,
                    nullable: false,
                },
            ],
        );
        cols.insert(
            "orders".into(),
            vec![
                ColumnHint {
                    name: "id".into(),
                    data_type: "bigint".into(),
                    primary_key: true,
                    nullable: false,
                },
                ColumnHint {
                    name: "user_id".into(),
                    data_type: "bigint".into(),
                    primary_key: false,
                    nullable: false,
                },
                ColumnHint {
                    name: "total_amount".into(),
                    data_type: "decimal(10,2)".into(),
                    primary_key: false,
                    nullable: false,
                },
            ],
        );
        SchemaSnapshot {
            current_db: "shop".into(),
            databases: vec!["shop".into(), "analytics".into()],
            tables: vec![
                ("users".into(), ObjectKind::Table),
                ("orders".into(), ObjectKind::Table),
            ],
            columns: cols,
        }
    }

    #[test]
    fn suggests_tables_after_from() {
        let schema = sample_schema();
        let sql = "SELECT * FROM us";
        let (fs, qual, frag) = extract_token_context(sql, sql.len());
        assert_eq!(qual, None);
        assert_eq!(frag, "us");
        let items = compute_completions(
            sql,
            sql.len(),
            fs,
            qual.as_deref(),
            &frag,
            lsp_types::Range::default(),
            &schema,
        );
        assert_eq!(items.first().map(|i| i.label.as_str()), Some("users"));
    }

    #[test]
    fn suggests_columns_via_alias_dot() {
        let schema = sample_schema();
        let sql = "SELECT u.em FROM users u";
        let offset = 11; // right after `u.em`
        let (fs, qual, frag) = extract_token_context(sql, offset);
        assert_eq!(qual.as_deref(), Some("u"));
        assert_eq!(frag, "em");
        let items = compute_completions(
            sql,
            offset,
            fs,
            qual.as_deref(),
            &frag,
            lsp_types::Range::default(),
            &schema,
        );
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].label, "email");
    }

    #[test]
    fn prioritizes_referenced_table_columns_in_where() {
        let schema = sample_schema();
        let sql = "SELECT * FROM orders WHERE tot";
        let (fs, qual, frag) = extract_token_context(sql, sql.len());
        let items = compute_completions(
            sql,
            sql.len(),
            fs,
            qual.as_deref(),
            &frag,
            lsp_types::Range::default(),
            &schema,
        );
        assert_eq!(items.first().map(|i| i.label.as_str()), Some("total_amount"));
    }
}
