use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatementKind {
    Select,
    Insert,
    Update,
    Delete,
    Replace,
    Create,
    Alter,
    Drop,
    Truncate,
    Show,
    Describe,
    Explain,
    Use,
    Set,
    Call,
    Transaction,
    Other,
}

impl StatementKind {
    pub fn is_mutating(self) -> bool {
        matches!(
            self,
            Self::Insert
                | Self::Update
                | Self::Delete
                | Self::Replace
                | Self::Create
                | Self::Alter
                | Self::Drop
                | Self::Truncate
                | Self::Call
        )
    }

    pub fn typically_returns_rows(self) -> bool {
        matches!(
            self,
            Self::Select | Self::Show | Self::Describe | Self::Explain
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedStatement {
    pub sql: String,
    pub kind: StatementKind,
    pub start_offset: usize,
    pub end_offset: usize,
}

#[derive(Debug, Clone, Default)]
pub struct QueryOptions {
    pub max_rows: Option<usize>,
    pub timeout_secs: Option<u64>,
    pub streaming: bool,
}

/// Cancellation token for in-flight queries.
#[derive(Debug, Clone, Default)]
pub struct CancelToken {
    inner: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl CancelToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.inner
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn child(&self) -> Self {
        self.clone()
    }
}