//! Database driver abstraction layer.

pub mod capability;
pub mod connection;
pub mod dialect;
pub mod meta;
pub mod query;
pub mod result;

pub use capability::*;
pub use connection::*;
pub use dialect::*;
pub use meta::*;
pub use query::*;
pub use result::*;

use ace_core::{AceResult, ConnectionInfo, EngineKind};
use async_trait::async_trait;
use std::sync::Arc;

/// Factory that can open connections for a given engine.
#[async_trait]
pub trait Driver: Send + Sync {
    fn name(&self) -> &str;
    fn engines(&self) -> &[EngineKind];
    fn supports(&self, engine: EngineKind) -> bool {
        self.engines().contains(&engine)
    }

    async fn connect(&self, info: &ConnectionInfo) -> AceResult<Arc<dyn Connection>>;
}

/// Registry of available drivers.
#[derive(Default)]
pub struct DriverRegistry {
    drivers: Vec<Arc<dyn Driver>>,
}

impl DriverRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, driver: Arc<dyn Driver>) {
        self.drivers.push(driver);
    }

    pub fn for_engine(&self, engine: EngineKind) -> Option<Arc<dyn Driver>> {
        self.drivers
            .iter()
            .find(|d| d.supports(engine))
            .cloned()
    }

    pub async fn open(&self, info: &ConnectionInfo) -> AceResult<Arc<dyn Connection>> {
        let driver = self
            .for_engine(info.engine)
            .ok_or_else(|| ace_core::AceError::NotSupported(format!("{:?}", info.engine)))?;
        driver.connect(info).await
    }

    pub fn drivers(&self) -> &[Arc<dyn Driver>] {
        &self.drivers
    }
}