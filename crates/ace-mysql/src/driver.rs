use crate::connection::MysqlConnection;
use ace_core::{AceResult, ConnectionInfo, EngineKind};
use ace_driver::{Connection, Driver};
use async_trait::async_trait;
use std::sync::Arc;

#[derive(Debug, Default)]
pub struct MysqlDriver;

impl MysqlDriver {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Driver for MysqlDriver {
    fn name(&self) -> &str {
        "mysql"
    }

    fn engines(&self) -> &[EngineKind] {
        &[EngineKind::Mysql, EngineKind::MariaDb]
    }

    async fn connect(&self, info: &ConnectionInfo) -> AceResult<Arc<dyn Connection>> {
        let conn = MysqlConnection::connect(info).await?;
        Ok(Arc::new(conn))
    }
}