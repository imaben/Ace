//! Lightweight, zero-dependency SQL syntax highlighter for GPUI EditorState.

use gpui_kit::component::input::{
    EditorState, FoldRange, HighlightStyleResolver, InputEdit, InputHighlighter, Rope,
};
use gpui_kit::*;
use std::ops::Range;

#[derive(Clone, Copy, PartialEq, Eq)]
enum TokenKind {
    Keyword,
    Function,
    Type,
    String,
    Number,
    Comment,
    Ident,
    Operator,
}

impl TokenKind {
    fn highlight_name(self) -> &'static str {
        match self {
            Self::Keyword => "keyword",
            Self::Function => "function",
            Self::Type => "type",
            Self::String => "string",
            Self::Number => "number",
            Self::Comment => "comment",
            Self::Ident => "variable",
            Self::Operator => "operator",
        }
    }

    fn fallback_color(self) -> Hsla {
        match self {
            Self::Keyword => rgb(0x569cd6).into(),
            Self::Function => rgb(0xdcdcaa).into(),
            Self::Type => rgb(0x4ec9b0).into(),
            Self::String => rgb(0xce9178).into(),
            Self::Number => rgb(0xb5cea8).into(),
            Self::Comment => rgb(0x6a9955).into(),
            Self::Ident => rgb(0x9cdcfe).into(),
            Self::Operator => rgb(0xd4d4d4).into(),
        }
    }
}

pub struct SqlHighlighter {
    tokens: Vec<(Range<usize>, TokenKind)>,
}

impl SqlHighlighter {
    pub fn new() -> Self {
        Self { tokens: Vec::new() }
    }

    fn reparse(&mut self, text: &str) {
        self.tokens.clear();
        let bytes = text.as_bytes();
        let len = bytes.len();
        let mut i = 0;

        while i < len {
            let b = bytes[i];

            // Whitespace
            if b.is_ascii_whitespace() {
                i += 1;
                continue;
            }

            // Line comment: -- or #
            if (b == b'-' && i + 1 < len && bytes[i + 1] == b'-') || b == b'#' {
                let start = i;
                while i < len && bytes[i] != b'\n' {
                    i += 1;
                }
                self.tokens.push((start..i, TokenKind::Comment));
                continue;
            }

            // Block comment: /* ... */
            if b == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
                let start = i;
                i += 2;
                while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                i = (i + 2).min(len);
                self.tokens.push((start..i, TokenKind::Comment));
                continue;
            }

            // Single-quoted or double-quoted string
            if b == b'\'' || b == b'"' {
                let quote = b;
                let start = i;
                i += 1;
                while i < len {
                    if bytes[i] == b'\\' && i + 1 < len {
                        i += 2;
                        continue;
                    }
                    if bytes[i] == quote {
                        i += 1;
                        if i < len && bytes[i] == quote {
                            i += 1;
                            continue;
                        }
                        break;
                    }
                    i += 1;
                }
                self.tokens.push((start..i, TokenKind::String));
                continue;
            }

            // Backtick-quoted MySQL identifier
            if b == b'`' {
                let start = i;
                i += 1;
                while i < len {
                    if bytes[i] == b'`' {
                        i += 1;
                        if i < len && bytes[i] == b'`' {
                            i += 1;
                            continue;
                        }
                        break;
                    }
                    i += 1;
                }
                self.tokens.push((start..i, TokenKind::Ident));
                continue;
            }

            // Numbers
            if b.is_ascii_digit() {
                let start = i;
                i += 1;
                while i < len
                    && (bytes[i].is_ascii_digit()
                        || bytes[i] == b'.'
                        || bytes[i] == b'e'
                        || bytes[i] == b'E'
                        || bytes[i] == b'x'
                        || bytes[i].is_ascii_hexdigit())
                {
                    i += 1;
                }
                self.tokens.push((start..i, TokenKind::Number));
                continue;
            }

            // Word (keyword / type / function / identifier)
            if b.is_ascii_alphabetic() || b == b'_' || b == b'@' {
                let start = i;
                i += 1;
                while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'$') {
                    i += 1;
                }
                let word = &text[start..i];
                let upper = word.to_ascii_uppercase();
                // Peek for '(' for function calls
                let mut peek = i;
                while peek < len && bytes[peek].is_ascii_whitespace() {
                    peek += 1;
                }
                let is_call = peek < len && bytes[peek] == b'(';

                if is_sql_keyword(&upper) {
                    self.tokens.push((start..i, TokenKind::Keyword));
                } else if is_sql_type(&upper) {
                    self.tokens.push((start..i, TokenKind::Type));
                } else if is_call {
                    self.tokens.push((start..i, TokenKind::Function));
                }
                continue;
            }

            // Operators
            if matches!(b, b'=' | b'<' | b'>' | b'!' | b'+' | b'-' | b'*' | b'/' | b'%' | b';') {
                let start = i;
                i += 1;
                self.tokens.push((start..i, TokenKind::Operator));
                continue;
            }

            i += 1;
        }
    }
}

impl InputHighlighter for SqlHighlighter {
    fn language(&self) -> SharedString {
        "sql".into()
    }

    fn update(
        &mut self,
        _edit: Option<InputEdit>,
        text: &Rope,
        _folding: bool,
        _window: &mut Window,
        _cx: &mut Context<EditorState>,
    ) {
        self.reparse(&text.to_string());
    }

    fn styles(
        &self,
        range: &Range<usize>,
        resolver: &dyn HighlightStyleResolver,
    ) -> Vec<(Range<usize>, HighlightStyle)> {
        let mut out = Vec::new();
        let mut cursor = range.start;

        let start_idx = self.tokens.partition_point(|(r, _)| r.end <= range.start);
        for (tok_range, kind) in &self.tokens[start_idx..] {
            if tok_range.start >= range.end {
                break;
            }
            let s = tok_range.start.max(range.start);
            let e = tok_range.end.min(range.end);
            if s > cursor {
                out.push((cursor..s, HighlightStyle::default()));
            }
            if e > s {
                let mut style = resolver
                    .style(kind.highlight_name())
                    .unwrap_or_else(|| HighlightStyle {
                        color: Some(kind.fallback_color()),
                        ..Default::default()
                    });
                if matches!(kind, TokenKind::Keyword) {
                    style.font_weight = Some(FontWeight::MEDIUM);
                }
                if matches!(kind, TokenKind::Comment) {
                    style.font_style = Some(FontStyle::Italic);
                }
                out.push((s..e, style));
                cursor = e;
            }
        }
        if cursor < range.end {
            out.push((cursor..range.end, HighlightStyle::default()));
        }
        out
    }

    fn fold_ranges(&self, _text: &Rope) -> Vec<FoldRange> {
        Vec::new()
    }
}

pub fn sql_highlighter_factory(
) -> gpui_kit::component::input::InputHighlighterFactory {
    std::rc::Rc::new(|lang: &str| {
        if lang.eq_ignore_ascii_case("sql") {
            Some(Box::new(SqlHighlighter::new()) as Box<dyn InputHighlighter>)
        } else {
            None
        }
    })
}

fn is_sql_keyword(w: &str) -> bool {
    matches!(
        w,
        "SELECT" | "FROM" | "WHERE" | "AND" | "OR" | "NOT" | "IN" | "IS" | "NULL"
            | "AS" | "ON" | "JOIN" | "LEFT" | "RIGHT" | "INNER" | "OUTER" | "CROSS" | "FULL"
            | "GROUP" | "BY" | "ORDER" | "ASC" | "DESC" | "HAVING" | "LIMIT" | "OFFSET"
            | "DISTINCT" | "ALL" | "UNION" | "INTERSECT" | "EXCEPT" | "CASE" | "WHEN"
            | "THEN" | "ELSE" | "END" | "EXISTS" | "BETWEEN" | "LIKE" | "RLIKE" | "REGEXP"
            | "INSERT" | "INTO" | "VALUES" | "UPDATE" | "SET" | "DELETE" | "REPLACE"
            | "CREATE" | "ALTER" | "DROP" | "TRUNCATE" | "RENAME" | "TABLE" | "DATABASE"
            | "SCHEMA" | "INDEX" | "VIEW" | "TRIGGER" | "PROCEDURE" | "FUNCTION" | "EVENT"
            | "PRIMARY" | "KEY" | "FOREIGN" | "REFERENCES" | "CONSTRAINT" | "UNIQUE"
            | "DEFAULT" | "AUTO_INCREMENT" | "AUTOINCREMENT" | "COMMENT" | "ENGINE"
            | "CHARSET" | "CHARACTER" | "COLLATE" | "UNSIGNED" | "ZEROFILL" | "BINARY"
            | "IF" | "SHOW" | "DESCRIBE" | "EXPLAIN" | "USE" | "BEGIN" | "COMMIT"
            | "ROLLBACK" | "START" | "TRANSACTION" | "GRANT" | "REVOKE" | "FLUSH" | "KILL"
            | "OPTIMIZE" | "ANALYZE" | "CHECK" | "REPAIR" | "ADD" | "MODIFY" | "CHANGE"
            | "COLUMN" | "BEFORE" | "AFTER" | "FOR" | "EACH" | "ROW" | "TRUE" | "FALSE"
            | "WITH" | "RECURSIVE" | "OVER" | "PARTITION" | "WINDOW" | "RETURNING"
            | "CASCADE" | "RESTRICT" | "NO" | "ACTION" | "DUPLICATE" | "IGNORE"
            | "TEMPORARY" | "FULLTEXT" | "SPATIAL" | "STATUS" | "VARIABLES" | "PROCESSLIST"
            | "DATABASES" | "TABLES" | "COLUMNS" | "INDEXES" | "TRIGGERS" | "GRANTS"
    )
}

fn is_sql_type(w: &str) -> bool {
    matches!(
        w,
        "INT" | "INTEGER" | "BIGINT" | "SMALLINT" | "TINYINT" | "MEDIUMINT"
            | "DECIMAL" | "NUMERIC" | "FLOAT" | "DOUBLE" | "REAL" | "BIT" | "BOOL" | "BOOLEAN"
            | "CHAR" | "VARCHAR" | "TEXT" | "TINYTEXT" | "MEDIUMTEXT" | "LONGTEXT"
            | "BLOB" | "TINYBLOB" | "MEDIUMBLOB" | "LONGBLOB" | "VARBINARY"
            | "DATE" | "TIME" | "DATETIME" | "TIMESTAMP" | "YEAR"
            | "JSON" | "ENUM" | "UUID" | "GEOMETRY" | "POINT"
    )
}
