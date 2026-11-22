use thiserror::Error;

#[derive(Debug, Error)]
pub enum AceError {
    #[error("{0}")]
    Message(String),
    #[error("connection failed: {0}")]
    Connection(String),
    #[error("query failed: {0}")]
    Query(String),
    #[error("auth failed: {0}")]
    Auth(String),
    #[error("io error: {0}")]
    Io(String),
    #[error("not supported: {0}")]
    NotSupported(String),
    #[error("cancelled")]
    Cancelled,
}

impl AceError {
    pub fn msg(s: impl Into<String>) -> Self {
        Self::Message(s.into())
    }
}

pub type AceResult<T> = Result<T, AceError>;

impl From<std::io::Error> for AceError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}

impl From<serde_json::Error> for AceError {
    fn from(e: serde_json::Error) -> Self {
        Self::Message(e.to_string())
    }
}