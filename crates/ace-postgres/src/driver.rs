use crate::connection::PostgresConnection;
use ace_core::{AceError, AceResult, ConnectionInfo, EngineKind};
use ace_driver::{Connection, Driver};
use async_trait::async_trait;
use std::sync::Arc;

#[derive(Debug, Default)]
pub struct PostgresDriver;

impl PostgresDriver {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Driver for PostgresDriver {
    fn name(&self) -> &str {
        "postgres"
    }

    fn engines(&self) -> &[EngineKind] {
        &[EngineKind::Postgres]
    }

    async fn connect(&self, info: &ConnectionInfo) -> AceResult<Arc<dyn Connection>> {
        let conn = PostgresConnection::connect(info)
            .await
            .map_err(|e| AceError::Connection(e.to_string()))?;
        Ok(Arc::new(conn))
    }
}