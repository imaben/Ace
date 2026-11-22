use crate::connection::SqliteConnection;
use ace_core::{AceError, AceResult, ConnectionInfo, EngineKind};
use ace_driver::{Connection, Driver};
use async_trait::async_trait;
use std::sync::Arc;

#[derive(Debug, Default)]
pub struct SqliteDriver;

impl SqliteDriver {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Driver for SqliteDriver {
    fn name(&self) -> &str {
        "sqlite"
    }

    fn engines(&self) -> &[EngineKind] {
        &[EngineKind::Sqlite]
    }

    async fn connect(&self, info: &ConnectionInfo) -> AceResult<Arc<dyn Connection>> {
        let conn = SqliteConnection::connect(info)
            .await
            .map_err(|e| AceError::Connection(e.to_string()))?;
        Ok(Arc::new(conn))
    }
}