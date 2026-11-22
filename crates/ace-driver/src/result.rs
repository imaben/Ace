use ace_core::Value;
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultColumn {
    pub name: String,
    pub table: Option<String>,
    pub database: Option<String>,
    pub data_type: String,
    pub nullable: bool,
    pub primary_key: bool,
    pub org_name: Option<String>,
}

pub type Row = Vec<Value>;

#[derive(Debug, Clone)]
pub struct QueryResult {
    pub columns: Vec<ResultColumn>,
    pub rows: Vec<Row>,
    pub affected_rows: u64,
    pub last_insert_id: Option<u64>,
    pub warnings: u64,
    pub info: String,
    pub duration_ms: u64,
}

impl QueryResult {
    pub fn empty() -> Self {
        Self {
            columns: Vec::new(),
            rows: Vec::new(),
            affected_rows: 0,
            last_insert_id: None,
            warnings: 0,
            info: String::new(),
            duration_ms: 0,
        }
    }

    pub fn is_result_set(&self) -> bool {
        !self.columns.is_empty()
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }
}

/// Streaming result for large tables — yields rows in chunks.
pub type ResultStream = BoxStream<'static, ace_core::AceResult<Row>>;

pub struct StreamingQueryResult {
    pub columns: Vec<ResultColumn>,
    pub stream: ResultStream,
    pub affected_rows: u64,
}