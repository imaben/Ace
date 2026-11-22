use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterOp {
    Eq,
    Neq,
    Gt,
    Gte,
    Lt,
    Lte,
    Like,
    NotLike,
    IsNull,
    IsNotNull,
    In,
    NotIn,
    Between,
    Regexp,
    BeginsWith,
    EndsWith,
    Contains,
}

impl FilterOp {
    pub fn needs_value(self) -> bool {
        !matches!(self, Self::IsNull | Self::IsNotNull)
    }

    pub fn sql_op(self) -> &'static str {
        match self {
            Self::Eq => "=",
            Self::Neq => "!=",
            Self::Gt => ">",
            Self::Gte => ">=",
            Self::Lt => "<",
            Self::Lte => "<=",
            Self::Like | Self::BeginsWith | Self::EndsWith | Self::Contains => "LIKE",
            Self::NotLike => "NOT LIKE",
            Self::IsNull => "IS NULL",
            Self::IsNotNull => "IS NOT NULL",
            Self::In => "IN",
            Self::NotIn => "NOT IN",
            Self::Between => "BETWEEN",
            Self::Regexp => "REGEXP",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Conjunction {
    And,
    Or,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FilterNode {
    Group {
        conj: Conjunction,
        children: Vec<FilterNode>,
    },
    Rule {
        column: String,
        op: FilterOp,
        value: Option<String>,
        value2: Option<String>,
    },
}

impl FilterNode {
    pub fn rule(column: impl Into<String>, op: FilterOp, value: Option<String>) -> Self {
        Self::Rule {
            column: column.into(),
            op,
            value,
            value2: None,
        }
    }

    pub fn to_where_clause(&self, quote_ident: &dyn Fn(&str) -> String) -> String {
        match self {
            Self::Group { conj, children } => {
                let parts: Vec<String> = children
                    .iter()
                    .map(|c| c.to_where_clause(quote_ident))
                    .filter(|s| !s.is_empty())
                    .collect();
                if parts.is_empty() {
                    return String::new();
                }
                let joiner = match conj {
                    Conjunction::And => " AND ",
                    Conjunction::Or => " OR ",
                };
                if parts.len() == 1 {
                    parts.into_iter().next().unwrap()
                } else {
                    format!("({})", parts.join(joiner))
                }
            }
            Self::Rule {
                column,
                op,
                value,
                value2,
            } => {
                let col = quote_ident(column);
                match op {
                    FilterOp::IsNull | FilterOp::IsNotNull => {
                        format!("{col} {}", op.sql_op())
                    }
                    FilterOp::Between => {
                        let a = literal(value.as_deref().unwrap_or(""));
                        let b = literal(value2.as_deref().unwrap_or(""));
                        format!("{col} BETWEEN {a} AND {b}")
                    }
                    FilterOp::In | FilterOp::NotIn => {
                        let raw = value.as_deref().unwrap_or("");
                        let items: Vec<String> = raw
                            .split(',')
                            .map(|s| literal(s.trim()))
                            .collect();
                        format!("{col} {} ({})", op.sql_op(), items.join(", "))
                    }
                    FilterOp::BeginsWith => {
                        format!(
                            "{col} LIKE {}",
                            literal(&format!("{}%", value.as_deref().unwrap_or("")))
                        )
                    }
                    FilterOp::EndsWith => {
                        format!(
                            "{col} LIKE {}",
                            literal(&format!("%{}", value.as_deref().unwrap_or("")))
                        )
                    }
                    FilterOp::Contains => {
                        format!(
                            "{col} LIKE {}",
                            literal(&format!("%{}%", value.as_deref().unwrap_or("")))
                        )
                    }
                    _ => {
                        let v = literal(value.as_deref().unwrap_or(""));
                        format!("{col} {} {v}", op.sql_op())
                    }
                }
            }
        }
    }
}

fn literal(s: &str) -> String {
    if s.eq_ignore_ascii_case("null") {
        return "NULL".into();
    }
    if s.parse::<i64>().is_ok() || s.parse::<f64>().is_ok() {
        return s.to_string();
    }
    format!("'{}'", s.replace('\\', "\\\\").replace('\'', "''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_where() {
        let node = FilterNode::Group {
            conj: Conjunction::And,
            children: vec![
                FilterNode::rule("id", FilterOp::Gt, Some("10".into())),
                FilterNode::rule("name", FilterOp::Contains, Some("ace".into())),
            ],
        };
        let sql = node.to_where_clause(&|c| format!("`{c}`"));
        assert!(sql.contains("`id` > 10"));
        assert!(sql.contains("LIKE '%ace%'"));
    }
}