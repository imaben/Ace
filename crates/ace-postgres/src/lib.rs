//! PostgreSQL driver (phase 7).

mod connection;
mod driver;

pub use connection::PostgresConnection;
pub use driver::PostgresDriver;

use ace_driver::Driver;
use std::sync::Arc;

pub fn driver() -> Arc<dyn Driver> {
    Arc::new(PostgresDriver::new())
}