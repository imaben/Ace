use crate::convert::{column_meta, row_to_values};
use crate::meta_queries;
use ace_core::{AceError, AceResult, ConnectionInfo};
use ace_driver::dialect::{Dialect, MysqlDialect};
use ace_driver::meta::*;
use ace_driver::query::{CancelToken, QueryOptions};
use ace_driver::result::{QueryResult, StreamingQueryResult};
use ace_driver::Connection;
use async_trait::async_trait;
use futures::stream;
use mysql_async::prelude::*;
use mysql_async::{Conn, Opts, OptsBuilder, Pool, Row};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct MysqlConnection {
    id: Uuid,
    info: ConnectionInfo,
    pool: Pool,
    dialect: Arc<dyn Dialect>,
    session: Mutex<Option<Conn>>,
}

impl MysqlConnection {
    pub async fn connect(info: &ConnectionInfo) -> AceResult<Self> {
        let opts = build_opts(info)?;
        let pool = Pool::new(opts);
        let mut conn = pool
            .get_conn()
            .await
            .map_err(|e| AceError::Connection(e.to_string()))?;

        if let Some(tz) = info.timezone.mysql_set_value() {
            let _ = conn
                .query_drop(format!("SET time_zone = '{}'", tz.replace('\'', "''")))
                .await;
        }
        if let Some(db) = &info.database {
            if !db.is_empty() {
                conn.query_drop(format!("USE `{}`", db.replace('`', "``")))
                    .await
                    .map_err(|e| AceError::Connection(e.to_string()))?;
            }
        }

        Ok(Self {
            id: Uuid::new_v4(),
            info: info.clone(),
            pool,
            dialect: Arc::new(MysqlDialect),
            session: Mutex::new(Some(conn)),
        })
    }

    async fn take_conn(&self) -> AceResult<Conn> {
        let mut guard = self.session.lock().await;
        if let Some(conn) = guard.take() {
            return Ok(conn);
        }
        self.pool
            .get_conn()
            .await
            .map_err(|e| AceError::Connection(e.to_string()))
    }

    async fn put_conn(&self, conn: Conn) {
        let mut guard = self.session.lock().await;
        *guard = Some(conn);
    }
}

fn build_opts(info: &ConnectionInfo) -> AceResult<Opts> {
    let mut builder = OptsBuilder::default()
        .user(Some(info.user.clone()))
        .pass(info.password.clone())
        .db_name(info.database.clone());

    match info.method {
        ace_core::ConnectionMethod::Socket => {
            let socket = info
                .socket_path
                .clone()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "/tmp/mysql.sock".into());
            builder = builder.socket(Some(socket)).prefer_socket(true);
        }
        _ => {
            builder = builder
                .ip_or_hostname(info.host.clone())
                .tcp_port(info.port)
                .prefer_socket(false);
        }
    }

    if info.use_compression {
        builder = builder.compression(mysql_async::Compression::fast());
    }

    if info.ssl.enabled {
        let mut ssl = mysql_async::SslOpts::default();
        if let Some(ca) = info.ssl.ca_cert_path.as_deref().filter(|s| !s.trim().is_empty()) {
            let expanded = expand_tilde(ca);
            ssl = ssl.with_root_certs(vec![std::path::PathBuf::from(expanded).into()]);
        }
        if !info.ssl.verify_server_cert {
            ssl = ssl
                .with_danger_accept_invalid_certs(true)
                .with_danger_skip_domain_validation(true);
        }
        builder = builder.ssl_opts(Some(ssl));
    }

    Ok(Opts::from(builder))
}

fn expand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return std::path::Path::new(&home).join(rest).display().to_string();
        }
    }
    path.to_string()
}

#[async_trait]
impl Connection for MysqlConnection {
    fn id(&self) -> Uuid {
        self.id
    }

    fn info(&self) -> &ConnectionInfo {
        &self.info
    }

    fn dialect(&self) -> Arc<dyn Dialect> {
        self.dialect.clone()
    }

    async fn ping(&self) -> AceResult<()> {
        let mut conn = self.take_conn().await?;
        let result = conn
            .ping()
            .await
            .map_err(|e| AceError::Connection(e.to_string()));
        self.put_conn(conn).await;
        result
    }

    async fn close(&self) -> AceResult<()> {
        let mut guard = self.session.lock().await;
        if let Some(conn) = guard.take() {
            drop(conn);
        }
        let _ = self.pool.clone().disconnect().await;
        Ok(())
    }

    async fn server_info(&self) -> AceResult<ServerInfo> {
        let mut conn = self.take_conn().await?;
        let result = meta_queries::fetch_server_info(&mut conn, &self.info.host, self.info.port).await;
        self.put_conn(conn).await;
        result
    }

    async fn list_databases(&self) -> AceResult<Vec<String>> {
        let mut conn = self.take_conn().await?;
        let result = meta_queries::list_databases(&mut conn).await;
        self.put_conn(conn).await;
        result
    }

    async fn use_database(&self, name: &str) -> AceResult<()> {
        let sql = format!("USE `{}`", name.replace('`', "``"));
        let mut conn = self.take_conn().await?;
        let result = conn
            .query_drop(sql)
            .await
            .map_err(|e| AceError::Query(e.to_string()));
        self.put_conn(conn).await;
        result
    }

    async fn current_database(&self) -> AceResult<Option<String>> {
        let mut conn = self.take_conn().await?;
        let result = async {
            let row: Option<Row> = conn
                .query_first("SELECT DATABASE()")
                .await
                .map_err(|e| AceError::Query(e.to_string()))?;
            Ok(row.and_then(|r| {
                let v = row_to_values(r);
                match v.first() {
                    Some(ace_core::Value::Null) | None => None,
                    Some(val) => Some(val.to_string()),
                }
            }))
        }
        .await;
        self.put_conn(conn).await;
        result
    }

    async fn list_objects(&self, database: &str) -> AceResult<Vec<DatabaseObject>> {
        let mut conn = self.take_conn().await?;
        let result = meta_queries::list_objects(&mut conn, database).await;
        self.put_conn(conn).await;
        result
    }

    async fn table_meta(&self, database: &str, table: &str) -> AceResult<TableMeta> {
        let mut conn = self.take_conn().await?;
        let result = meta_queries::table_meta(&mut conn, database, table).await;
        self.put_conn(conn).await;
        result
    }

    async fn list_indexes(&self, database: &str, table: &str) -> AceResult<Vec<IndexMeta>> {
        let mut conn = self.take_conn().await?;
        let result = meta_queries::list_indexes(&mut conn, database, table).await;
        self.put_conn(conn).await;
        result
    }

    async fn list_foreign_keys(
        &self,
        database: &str,
        table: &str,
    ) -> AceResult<Vec<ForeignKeyMeta>> {
        let mut conn = self.take_conn().await?;
        let result = meta_queries::list_foreign_keys(&mut conn, database, table).await;
        self.put_conn(conn).await;
        result
    }

    async fn list_triggers(&self, database: &str, table: &str) -> AceResult<Vec<TriggerMeta>> {
        let mut conn = self.take_conn().await?;
        let result = meta_queries::list_triggers(&mut conn, database, table).await;
        self.put_conn(conn).await;
        result
    }

    async fn execute(
        &self,
        sql: &str,
        options: QueryOptions,
        cancel: CancelToken,
    ) -> AceResult<QueryResult> {
        if cancel.is_cancelled() {
            return Err(AceError::Cancelled);
        }
        let max_rows = options.max_rows;
        let started = Instant::now();
        let mut conn = self.take_conn().await?;

        let result = async {
            if cancel.is_cancelled() {
                return Err(AceError::Cancelled);
            }

            let mut qr = conn
                .query_iter(sql)
                .await
                .map_err(|e| AceError::Query(e.to_string()))?;

            let columns: Vec<_> = qr.columns_ref().iter().map(column_meta).collect();
            let mut rows = Vec::new();
            let raw_rows: Vec<Row> = qr
                .collect()
                .await
                .map_err(|e| AceError::Query(e.to_string()))?;

            for row in raw_rows {
                if let Some(max) = max_rows {
                    if rows.len() >= max {
                        break;
                    }
                }
                rows.push(row_to_values(row));
            }

            let affected_rows = conn.affected_rows();
            let last_insert_id = conn.last_insert_id().filter(|&x| x > 0);
            let warnings = conn.get_warnings() as u64;
            let info = conn.info().into_owned();

            Ok(QueryResult {
                columns,
                rows,
                affected_rows,
                last_insert_id,
                warnings,
                info,
                duration_ms: started.elapsed().as_millis() as u64,
            })
        }
        .await;

        self.put_conn(conn).await;
        result
    }

    async fn stream_query(
        &self,
        sql: &str,
        options: QueryOptions,
        cancel: CancelToken,
    ) -> AceResult<StreamingQueryResult> {
        let result = self.execute(sql, options, cancel).await?;
        let columns = result.columns.clone();
        let affected = result.affected_rows;
        let rows = result.rows;
        let stream = stream::iter(rows.into_iter().map(Ok));
        Ok(StreamingQueryResult {
            columns,
            stream: Box::pin(stream),
            affected_rows: affected,
        })
    }

    async fn process_list(&self) -> AceResult<Vec<ProcessInfo>> {
        let mut conn = self.take_conn().await?;
        let result = meta_queries::process_list(&mut conn).await;
        self.put_conn(conn).await;
        result
    }

    async fn kill_query(&self, process_id: u64) -> AceResult<()> {
        // Use a separate pool connection so we can kill while the session is busy.
        let mut conn = self
            .pool
            .get_conn()
            .await
            .map_err(|e| AceError::Connection(e.to_string()))?;
        conn.query_drop(format!("KILL QUERY {process_id}"))
            .await
            .map_err(|e| AceError::Query(e.to_string()))
    }

    async fn kill_connection(&self, process_id: u64) -> AceResult<()> {
        let mut conn = self
            .pool
            .get_conn()
            .await
            .map_err(|e| AceError::Connection(e.to_string()))?;
        conn.query_drop(format!("KILL CONNECTION {process_id}"))
            .await
            .map_err(|e| AceError::Query(e.to_string()))
    }

    async fn server_variables(&self) -> AceResult<HashMap<String, String>> {
        let mut conn = self.take_conn().await?;
        let result = meta_queries::server_variables(&mut conn).await;
        self.put_conn(conn).await;
        result
    }

    async fn list_users(&self) -> AceResult<Vec<UserPrivilege>> {
        let mut conn = self.take_conn().await?;
        let result = async {
            let rows: Vec<Row> = conn
                .query("SELECT User, Host FROM mysql.user ORDER BY User, Host")
                .await
                .map_err(|e| AceError::Query(e.to_string()))?;
            Ok(rows
                .into_iter()
                .map(|r| {
                    let v = row_to_values(r);
                    UserPrivilege {
                        user: v.first().map(|x| x.to_string()).unwrap_or_default(),
                        host: v.get(1).map(|x| x.to_string()).unwrap_or_default(),
                        global_privileges: Vec::new(),
                        database_privileges: Vec::new(),
                        max_questions: None,
                        max_updates: None,
                        max_connections: None,
                        max_user_connections: None,
                    }
                })
                .collect())
        }
        .await;
        self.put_conn(conn).await;
        result
    }

    async fn thread_id(&self) -> AceResult<Option<u64>> {
        let mut conn = self.take_conn().await?;
        let result = async {
            let row: Option<Row> = conn
                .query_first("SELECT CONNECTION_ID()")
                .await
                .map_err(|e| AceError::Query(e.to_string()))?;
            Ok(row.and_then(|r| {
                let v = row_to_values(r);
                match v.first() {
                    Some(ace_core::Value::I64(i)) => Some(*i as u64),
                    Some(ace_core::Value::U64(u)) => Some(*u),
                    Some(ace_core::Value::String(s)) => s.parse().ok(),
                    _ => None,
                }
            }))
        }
        .await;
        self.put_conn(conn).await;
        result
    }
}