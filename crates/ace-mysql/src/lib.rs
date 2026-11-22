//! MySQL / MariaDB driver backed by `mysql_async`.

mod connection;
mod convert;
mod driver;
mod meta_queries;

pub use connection::MysqlConnection;
pub use driver::MysqlDriver;

use ace_driver::Driver;
use std::sync::Arc;

pub fn driver() -> Arc<dyn Driver> {
    Arc::new(MysqlDriver::new())
}