use ace_driver::query::ParsedStatement;
use crate::classify::classify_sql;

/// Split a SQL script into individual statements, respecting quotes and comments.
pub fn split_statements(sql: &str) -> Vec<ParsedStatement> {
    let mut out = Vec::new();
    let bytes = sql.as_bytes();
    let mut i = 0;
    let mut start = 0;
    let mut in_single = false;
    let mut in_double = false;
    let mut in_backtick = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;

    while i < bytes.len() {
        let c = bytes[i];
        let next = bytes.get(i + 1).copied().unwrap_or(0);

        if in_line_comment {
            if c == b'\n' {
                in_line_comment = false;
            }
            i += 1;
            continue;
        }
        if in_block_comment {
            if c == b'*' && next == b'/' {
                in_block_comment = false;
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }

        if !in_single && !in_double && !in_backtick {
            if c == b'-' && next == b'-' {
                in_line_comment = true;
                i += 2;
                continue;
            }
            if c == b'#' {
                in_line_comment = true;
                i += 1;
                continue;
            }
            if c == b'/' && next == b'*' {
                in_block_comment = true;
                i += 2;
                continue;
            }
            if c == b';' {
                let raw = &sql[start..i];
                let stmt = raw.trim();
                if !stmt.is_empty() {
                    let lead = raw.len() - raw.trim_start().len();
                    let trim_start = start + lead;
                    let trim_end = trim_start + stmt.len();
                    out.push(ParsedStatement {
                        kind: classify_sql(stmt),
                        sql: stmt.to_string(),
                        start_offset: trim_start,
                        end_offset: trim_end,
                    });
                }
                i += 1;
                start = i;
                continue;
            }
        }

        if c == b'\'' && !in_double && !in_backtick {
            // handle escaped ''
            if in_single && next == b'\'' {
                i += 2;
                continue;
            }
            in_single = !in_single;
        } else if c == b'"' && !in_single && !in_backtick {
            if in_double && next == b'"' {
                i += 2;
                continue;
            }
            in_double = !in_double;
        } else if c == b'`' && !in_single && !in_double {
            in_backtick = !in_backtick;
        }

        i += 1;
    }

    let raw = &sql[start..];
    let stmt = raw.trim();
    if !stmt.is_empty() {
        let lead = raw.len() - raw.trim_start().len();
        let trim_start = start + lead;
        let trim_end = trim_start + stmt.len();
        out.push(ParsedStatement {
            kind: classify_sql(stmt),
            sql: stmt.to_string(),
            start_offset: trim_start,
            end_offset: trim_end,
        });
    }
    out
}

/// Find the SQL statement at `cursor_byte` using Sequel Ace's `queryAtPosition:lookBehind:` rules:
/// 1. If cursor is inside a statement (or immediately after its trailing `;`), return that statement.
/// 2. If cursor is on leading indentation of a statement on the same line, return that statement.
/// 3. Otherwise look behind to the closest preceding statement; if none, return the next statement.
pub fn statement_at_cursor(sql: &str, cursor_byte: usize) -> Option<String> {
    let stmts = split_statements(sql);
    if stmts.is_empty() {
        return None;
    }
    let cursor = cursor_byte.min(sql.len());
    let bytes = sql.as_bytes();

    // Compute the semicolon-inclusive end for each statement
    let semi_end = |st: &ParsedStatement| -> usize {
        let mut idx = st.end_offset;
        while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
            idx += 1;
        }
        if idx < bytes.len() && bytes[idx] == b';' {
            idx + 1
        } else {
            st.end_offset
        }
    };

    // 1. Direct hit: cursor lies within [start_offset, semi_end]
    for st in &stmts {
        if cursor >= st.start_offset && cursor <= semi_end(st) {
            return Some(st.sql.clone());
        }
    }

    // 2. Cursor is in whitespace before a statement on the SAME line (e.g. `   |SELECT ...`)
    //    and no statement ended on this same line before cursor.
    let prev = stmts.iter().rev().find(|st| semi_end(st) < cursor);
    let next = stmts.iter().find(|st| st.start_offset > cursor);

    if let Some(nxt) = next {
        let same_line_as_next = !sql[cursor..nxt.start_offset].contains('\n');
        let prev_on_same_line = prev
            .map(|p| !sql[semi_end(p)..cursor].contains('\n'))
            .unwrap_or(false);
        if same_line_as_next && !prev_on_same_line {
            return Some(nxt.sql.clone());
        }
    }

    // 3. Sequel Ace lookBehind: prefer closest preceding statement, else next statement
    if let Some(p) = prev {
        return Some(p.sql.clone());
    }
    next.map(|n| n.sql.clone())
}

/// Lightweight streaming splitter for large SQL files.
pub struct StatementLexer {
    buf: String,
}

impl StatementLexer {
    pub fn new() -> Self {
        Self {
            buf: String::new(),
        }
    }

    pub fn push(&mut self, chunk: &str) -> Vec<String> {
        self.buf.push_str(chunk);
        let parts = split_statements(&self.buf);
        if parts.is_empty() {
            return Vec::new();
        }
        // Keep incomplete trailing statement in buffer.
        let last_complete = if self.buf.trim_end().ends_with(';') {
            parts.len()
        } else {
            parts.len().saturating_sub(1)
        };
        let mut out = Vec::new();
        for p in parts.iter().take(last_complete) {
            out.push(p.sql.clone());
        }
        if last_complete < parts.len() {
            self.buf = parts[last_complete].sql.clone();
        } else {
            self.buf.clear();
        }
        out
    }

    pub fn finish(self) -> Option<String> {
        let s = self.buf.trim().to_string();
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    }
}

impl Default for StatementLexer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_simple() {
        let stmts = split_statements("SELECT 1; SELECT 2");
        assert_eq!(stmts.len(), 2);
        assert_eq!(stmts[0].sql, "SELECT 1");
        assert_eq!(stmts[1].sql, "SELECT 2");
    }

    #[test]
    fn respects_quotes() {
        let stmts = split_statements("SELECT ';'; INSERT INTO t VALUES (1)");
        assert_eq!(stmts.len(), 2);
    }

    #[test]
    fn ignores_comments() {
        let stmts = split_statements("SELECT 1; -- comment;\nSELECT 2");
        assert_eq!(stmts.len(), 2);
    }

    #[test]
    fn cursor_selects_exact_statement() {
        let sql = "SELECT * FROM users;\n\n  SELECT * FROM orders;\nSELECT 3";
        // Inside first query
        assert_eq!(statement_at_cursor(sql, 5).as_deref(), Some("SELECT * FROM users"));
        // Immediately after first semicolon
        assert_eq!(statement_at_cursor(sql, 20).as_deref(), Some("SELECT * FROM users"));
        // On blank line between query 1 and 2 -> lookBehind picks query 1
        assert_eq!(statement_at_cursor(sql, 21).as_deref(), Some("SELECT * FROM users"));
        // Leading spaces before query 2 on line 3 -> picks query 2
        assert_eq!(statement_at_cursor(sql, 22).as_deref(), Some("SELECT * FROM orders"));
        // Inside query 2
        assert_eq!(statement_at_cursor(sql, 32).as_deref(), Some("SELECT * FROM orders"));
        // Immediately after query 2's semicolon
        assert_eq!(statement_at_cursor(sql, 45).as_deref(), Some("SELECT * FROM orders"));
        // Start of query 3
        assert_eq!(statement_at_cursor(sql, 46).as_deref(), Some("SELECT 3"));
        // End of query 3 (no semicolon)
        assert_eq!(statement_at_cursor(sql, sql.len()).as_deref(), Some("SELECT 3"));
    }
}