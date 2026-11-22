//! SQLite driver (phase 7).

mod connection;
mod driver;

pub use connection::SqliteConnection;
pub use driver::SqliteDriver;

use ace_driver::Driver;
use std::sync::Arc;

pub fn driver() -> Arc<dyn Driver> {
    Arc::new(SqliteDriver::new())
}