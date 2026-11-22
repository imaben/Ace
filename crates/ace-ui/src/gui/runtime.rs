//! Async Tokio runtime bridge, LoggedConnection (for SQL Console), SSH tunnel lifecycle,
//! metadata fetchers, export/import, and multi-DB driver dispatch.

use crate::gui::console::ConsoleLog;
use crate::gui::data_grid::GridCell;
use crate::gui::sql_complete::ColumnHint;
use ace_core::{
    parse_connection_string, AceError, AceResult, ConnectionInfo, ConnectionMethod, EngineKind,
};
use ace_driver::dialect::Dialect;
use ace_driver::meta::{
    ColumnMeta, DatabaseObject, ForeignKeyMeta, IndexMeta, ProcessInfo, ServerInfo, TableMeta,
    TriggerMeta, UserPrivilege,
};
use ace_driver::query::{CancelToken, QueryOptions};
use ace_driver::result::{QueryResult, StreamingQueryResult};
use ace_driver::{Connection, DriverRegistry};
use ace_io::{export_query_result, ExportFormat, ExportOptions};
use ace_net::SshTunnel;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, OnceLock};
use std::time::Instant;
use tokio::runtime::Runtime;
use tokio::sync::Mutex as TokioMutex;
use uuid::Uuid;

static TOKIO: OnceLock<Runtime> = OnceLock::new();

pub fn tokio_rt() -> &'static Runtime {
    TOKIO.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("ace-tokio")
            .build()
            .expect("tokio runtime")
    })
}

pub fn registry() -> DriverRegistry {
    let mut reg = DriverRegistry::new();
    reg.register(ace_mysql::driver());
    reg.register(ace_postgres::driver());
    reg.register(ace_sqlite::driver());
    reg
}

/// Connection wrapper that records executed queries into the session's SQL Console
/// and owns any SSH tunnel so it is torn down cleanly on disconnect.
pub struct LoggedConnection {
    inner: Arc<dyn Connection>,
    console: ConsoleLog,
    current_db: RwLock<String>,
    tunnel: TokioMutex<Option<SshTunnel>>,
}

impl LoggedConnection {
    pub fn new(
        inner: Arc<dyn Connection>,
        console: ConsoleLog,
        initial_db: String,
        tunnel: Option<SshTunnel>,
    ) -> Self {
        Self {
            inner,
            console,
            current_db: RwLock::new(initial_db),
            tunnel: TokioMutex::new(tunnel),
        }
    }

    pub fn set_current_db_label(&self, db: &str) {
        *self.current_db.write() = db.to_string();
    }

    fn db_label(&self) -> String {
        self.current_db.read().clone()
    }
}

#[async_trait::async_trait]
impl Connection for LoggedConnection {
    fn id(&self) -> Uuid {
        self.inner.id()
    }

    fn info(&self) -> &ConnectionInfo {
        self.inner.info()
    }

    fn dialect(&self) -> Arc<dyn Dialect> {
        self.inner.dialect()
    }

    async fn ping(&self) -> AceResult<()> {
        self.inner.ping().await
    }

    async fn close(&self) -> AceResult<()> {
        let res = self.inner.close().await;
        if let Some(tunnel) = self.tunnel.lock().await.take() {
            let _ = tunnel.close().await;
        }
        res
    }

    async fn server_info(&self) -> AceResult<ServerInfo> {
        self.inner.server_info().await
    }

    async fn list_databases(&self) -> AceResult<Vec<String>> {
        let sql = self.inner.dialect().show_databases_sql();
        let started = Instant::now();
        let res = self.inner.list_databases().await;
        self.console.push(
            self.db_label(),
            sql,
            started.elapsed().as_millis() as u64,
            res.as_ref().err().map(|e| e.to_string()),
        );
        res
    }

    async fn use_database(&self, name: &str) -> AceResult<()> {
        let sql = format!("USE {}", self.inner.dialect().quote_ident(name));
        let started = Instant::now();
        let res = self.inner.use_database(name).await;
        if res.is_ok() {
            self.set_current_db_label(name);
        }
        self.console.push(
            name.to_string(),
            sql,
            started.elapsed().as_millis() as u64,
            res.as_ref().err().map(|e| e.to_string()),
        );
        res
    }

    async fn current_database(&self) -> AceResult<Option<String>> {
        self.inner.current_database().await
    }

    async fn list_objects(&self, database: &str) -> AceResult<Vec<DatabaseObject>> {
        let sql = self.inner.dialect().show_tables_sql(database);
        let started = Instant::now();
        let res = self.inner.list_objects(database).await;
        self.console.push(
            database.to_string(),
            sql,
            started.elapsed().as_millis() as u64,
            res.as_ref().err().map(|e| e.to_string()),
        );
        res
    }

    async fn table_meta(&self, database: &str, table: &str) -> AceResult<TableMeta> {
        let sql = self.inner.dialect().describe_table_sql(database, table);
        let started = Instant::now();
        let res = self.inner.table_meta(database, table).await;
        self.console.push(
            database.to_string(),
            sql,
            started.elapsed().as_millis() as u64,
            res.as_ref().err().map(|e| e.to_string()),
        );
        res
    }

    async fn list_indexes(&self, database: &str, table: &str) -> AceResult<Vec<IndexMeta>> {
        self.inner.list_indexes(database, table).await
    }

    async fn list_foreign_keys(
        &self,
        database: &str,
        table: &str,
    ) -> AceResult<Vec<ForeignKeyMeta>> {
        self.inner.list_foreign_keys(database, table).await
    }

    async fn list_triggers(&self, database: &str, table: &str) -> AceResult<Vec<TriggerMeta>> {
        self.inner.list_triggers(database, table).await
    }

    async fn execute(
        &self,
        sql: &str,
        options: QueryOptions,
        cancel: CancelToken,
    ) -> AceResult<QueryResult> {
        let started = Instant::now();
        let res = self.inner.execute(sql, options, cancel).await;
        let ms = res
            .as_ref()
            .map(|r| r.duration_ms)
            .unwrap_or_else(|_| started.elapsed().as_millis() as u64);
        self.console.push(
            self.db_label(),
            sql.trim().to_string(),
            ms,
            res.as_ref().err().map(|e| e.to_string()),
        );
        res
    }

    async fn stream_query(
        &self,
        sql: &str,
        options: QueryOptions,
        cancel: CancelToken,
    ) -> AceResult<StreamingQueryResult> {
        self.inner.stream_query(sql, options, cancel).await
    }

    async fn process_list(&self) -> AceResult<Vec<ProcessInfo>> {
        let sql = self.inner.dialect().process_list_sql();
        let started = Instant::now();
        let res = self.inner.process_list().await;
        self.console.push(
            self.db_label(),
            sql,
            started.elapsed().as_millis() as u64,
            res.as_ref().err().map(|e| e.to_string()),
        );
        res
    }

    async fn kill_query(&self, process_id: u64) -> AceResult<()> {
        let sql = self.inner.dialect().kill_query_sql(process_id);
        let started = Instant::now();
        let res = self.inner.kill_query(process_id).await;
        self.console.push(
            self.db_label(),
            sql,
            started.elapsed().as_millis() as u64,
            res.as_ref().err().map(|e| e.to_string()),
        );
        res
    }

    async fn kill_connection(&self, process_id: u64) -> AceResult<()> {
        let sql = format!("KILL CONNECTION {process_id}");
        let started = Instant::now();
        let res = self.inner.kill_connection(process_id).await;
        self.console.push(
            self.db_label(),
            sql,
            started.elapsed().as_millis() as u64,
            res.as_ref().err().map(|e| e.to_string()),
        );
        res
    }

    async fn server_variables(&self) -> AceResult<HashMap<String, String>> {
        let started = Instant::now();
        let res = self.inner.server_variables().await;
        self.console.push(
            self.db_label(),
            "SHOW VARIABLES",
            started.elapsed().as_millis() as u64,
            res.as_ref().err().map(|e| e.to_string()),
        );
        res
    }

    async fn list_users(&self) -> AceResult<Vec<UserPrivilege>> {
        self.inner.list_users().await
    }

    async fn thread_id(&self) -> AceResult<Option<u64>> {
        self.inner.thread_id().await
    }
}

pub struct ConnectedBundle {
    pub conn: Arc<dyn Connection>,
    pub raw_conn: Arc<dyn Connection>,
    pub server_info: Option<ServerInfo>,
    pub databases: Vec<String>,
    pub current_db: String,
    pub objects: Vec<DatabaseObject>,
    pub status: String,
}

async fn open_raw_connection(
    info: &ConnectionInfo,
) -> AceResult<(Arc<dyn Connection>, Option<SshTunnel>)> {
    let mut info = info.clone();
    if info.method == ConnectionMethod::AwsIam {
        info.aws.enabled = true;
        info.ssl.enabled = true;
    }
    if info.method == ConnectionMethod::Vault {
        info.vault.enabled = true;
    }
    if info.aws.enabled {
        let token =
            ace_net::resolve_aws_token(&info.aws, &info.host, info.port, &info.user).await?;
        info.password = Some(token);
    }
    let mut tunnel = None;
    if info.method == ConnectionMethod::SshTunnel {
        if let Some(ssh) = info.ssh.clone() {
            let t = SshTunnel::open(&ssh, &info.host, info.port, "ssh").await?;
            info.host = "127.0.0.1".into();
            info.port = t.local_port;
            tunnel = Some(t);
        }
    }
    match registry().open(&info).await {
        Ok(conn) => Ok((conn, tunnel)),
        Err(e) => {
            if let Some(t) = tunnel {
                let _ = t.close().await;
            }
            Err(e)
        }
    }
}

pub async fn connect_session(
    info: &ConnectionInfo,
    console: ConsoleLog,
) -> AceResult<ConnectedBundle> {
    let (raw_conn, tunnel) = open_raw_connection(info).await?;
    let initial_db = info.database.clone().unwrap_or_default();
    let logged = Arc::new(LoggedConnection::new(
        raw_conn.clone(),
        console,
        initial_db,
        tunnel,
    ));
    let conn: Arc<dyn Connection> = logged.clone();

    let server_info = conn.server_info().await.ok();
    let databases = conn.list_databases().await.unwrap_or_default();
    let current_db = if info.engine == EngineKind::Sqlite {
        "main".to_string()
    } else {
        info.database
            .clone()
            .filter(|s| !s.trim().is_empty())
            .or_else(|| {
                server_info
                    .as_ref()
                    .and_then(|s| s.current_database.clone())
                    .filter(|s| !s.trim().is_empty())
            })
            .unwrap_or_else(|| {
                databases
                    .iter()
                    .find(|d| {
                        !matches!(
                            d.as_str(),
                            "information_schema" | "performance_schema" | "mysql" | "sys"
                        )
                    })
                    .or_else(|| databases.first())
                    .cloned()
                    .unwrap_or_default()
            })
    };

    if !current_db.is_empty() && info.engine != EngineKind::Sqlite {
        let _ = conn.use_database(&current_db).await;
    } else {
        logged.set_current_db_label(&current_db);
    }

    let objects = if current_db.is_empty() {
        Vec::new()
    } else {
        conn.list_objects(&current_db).await.unwrap_or_default()
    };

    let status = match &server_info {
        Some(s) => format!("{} {}", s.engine_name, s.version),
        None => format!("Connected to {}", info.display_host()),
    };

    Ok(ConnectedBundle {
        conn,
        raw_conn,
        server_info,
        databases,
        current_db,
        objects,
        status,
    })
}

/// Silently fetch all `(table -> columns)` for autocomplete without cluttering the SQL Console.
pub async fn fetch_database_columns(
    raw_conn: &Arc<dyn Connection>,
    engine: EngineKind,
    database: &str,
    table_names: &[String],
) -> HashMap<String, Vec<ColumnHint>> {
    let mut map: HashMap<String, Vec<ColumnHint>> = HashMap::new();
    if database.is_empty() {
        return map;
    }

    match engine {
        EngineKind::Mysql | EngineKind::MariaDb => {
            let escaped = database.replace('\\', "\\\\").replace('\'', "\\'");
            let sql = format!(
                "SELECT TABLE_NAME, COLUMN_NAME, COLUMN_TYPE, COLUMN_KEY, IS_NULLABLE \
                 FROM information_schema.COLUMNS \
                 WHERE TABLE_SCHEMA = '{escaped}' \
                 ORDER BY TABLE_NAME, ORDINAL_POSITION"
            );
            if let Ok(res) = raw_conn
                .execute(
                    &sql,
                    QueryOptions {
                        max_rows: Some(50_000),
                        ..Default::default()
                    },
                    CancelToken::new(),
                )
                .await
            {
                for row in res.rows {
                    let tbl = row.first().map(|v| v.to_string()).unwrap_or_default();
                    let col = row.get(1).map(|v| v.to_string()).unwrap_or_default();
                    if tbl.is_empty() || col.is_empty() {
                        continue;
                    }
                    let dtype = row.get(2).map(|v| v.to_string()).unwrap_or_default();
                    let key = row.get(3).map(|v| v.to_string()).unwrap_or_default();
                    let nullable = row
                        .get(4)
                        .map(|v| v.to_string().eq_ignore_ascii_case("YES"))
                        .unwrap_or(true);
                    map.entry(tbl).or_default().push(ColumnHint {
                        name: col,
                        data_type: dtype,
                        primary_key: key.contains("PRI"),
                        nullable,
                    });
                }
            }
        }
        EngineKind::Postgres => {
            let escaped = database.replace('\'', "''");
            let sql = format!(
                "SELECT table_name, column_name, data_type, is_nullable \
                 FROM information_schema.columns \
                 WHERE table_schema = '{escaped}' \
                 ORDER BY table_name, ordinal_position"
            );
            if let Ok(res) = raw_conn
                .execute(
                    &sql,
                    QueryOptions {
                        max_rows: Some(50_000),
                        ..Default::default()
                    },
                    CancelToken::new(),
                )
                .await
            {
                for row in res.rows {
                    let tbl = row.first().map(|v| v.to_string()).unwrap_or_default();
                    let col = row.get(1).map(|v| v.to_string()).unwrap_or_default();
                    if tbl.is_empty() || col.is_empty() {
                        continue;
                    }
                    let dtype = row.get(2).map(|v| v.to_string()).unwrap_or_default();
                    let nullable = row
                        .get(3)
                        .map(|v| v.to_string().eq_ignore_ascii_case("YES"))
                        .unwrap_or(true);
                    map.entry(tbl).or_default().push(ColumnHint {
                        name: col,
                        data_type: dtype,
                        primary_key: false,
                        nullable,
                    });
                }
            }
        }
        EngineKind::Sqlite => {
            for tbl in table_names.iter().take(200) {
                if let Ok(meta) = raw_conn.table_meta(database, tbl).await {
                    let cols = meta.columns.iter().map(ColumnHint::from).collect();
                    map.insert(tbl.clone(), cols);
                }
            }
        }
    }

    map
}

pub async fn test_connection(info: &ConnectionInfo) -> AceResult<String> {
    let (conn, tunnel) = open_raw_connection(info).await?;
    let ping_res = conn.ping().await;
    let ver = conn
        .server_info()
        .await
        .ok()
        .map(|s| format!("{} {}", s.engine_name, s.version));
    let _ = conn.close().await;
    if let Some(t) = tunnel {
        let _ = t.close().await;
    }
    ping_res?;
    Ok(ver.unwrap_or_else(|| "Connection succeeded".into()))
}

pub fn demo_sqlite_path() -> String {
    let mut path = std::env::current_dir().unwrap_or_else(|_| ".".into());
    path.push("demo");
    path.push("demo.db");
    if !path.exists() {
        let fallback = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("demo")
            .join("demo.db");
        if fallback.exists() {
            path = fallback;
        }
    }
    path.display().to_string()
}

pub fn demo_connection() -> ConnectionInfo {
    let path = demo_sqlite_path();
    let mut info = parse_connection_string(&format!("sqlite://{path}")).unwrap_or_default();
    info.name = "Demo SQLite".into();
    info.engine = EngineKind::Sqlite;
    info.color_index = 4;
    info.color = Some("#76b9e8".into());
    info
}

pub struct TablePageData {
    pub columns: Vec<String>,
    pub cells: Vec<Vec<GridCell>>,
    pub raw_result: QueryResult,
    pub total_count: Option<u64>,
}

pub async fn fetch_table_page(
    conn: &Arc<dyn Connection>,
    database: &str,
    table: &str,
    limit: usize,
    offset: usize,
    order_by: Option<&str>,
    where_clause: Option<&str>,
) -> AceResult<TablePageData> {
    let raw_result = conn
        .fetch_page(
            database,
            table,
            where_clause,
            order_by,
            limit,
            offset,
            CancelToken::new(),
        )
        .await?;
    let total_count = conn
        .count_rows(database, table, where_clause, CancelToken::new())
        .await
        .ok();
    let columns: Vec<String> = raw_result.columns.iter().map(|c| c.name.clone()).collect();
    let cells: Vec<Vec<GridCell>> = raw_result
        .rows
        .iter()
        .map(|r| r.iter().map(GridCell::from_value).collect())
        .collect();
    Ok(TablePageData {
        columns,
        cells,
        raw_result,
        total_count,
    })
}

pub async fn fetch_structure(
    conn: &Arc<dyn Connection>,
    database: &str,
    table: &str,
) -> AceResult<(TableMeta, Vec<TriggerMeta>)> {
    let meta = conn.table_meta(database, table).await?;
    let triggers = conn.list_triggers(database, table).await.unwrap_or_default();
    Ok((meta, triggers))
}

pub fn structure_column_rows(columns: &[ColumnMeta]) -> (Vec<String>, Vec<Vec<GridCell>>) {
    let headers = vec![
        "Field".into(),
        "Type".into(),
        "Length".into(),
        "Unsigned".into(),
        "Zerofill".into(),
        "Binary".into(),
        "Allow Null".into(),
        "Key".into(),
        "Default".into(),
        "Extra".into(),
        "Encoding".into(),
        "Collation".into(),
        "Comment".into(),
    ];
    let rows = columns
        .iter()
        .map(|c| {
            let length = c
                .character_maximum_length
                .map(|n| n.to_string())
                .or_else(|| {
                    c.numeric_precision.map(|p| match c.numeric_scale {
                        Some(s) if s > 0 => format!("{p},{s}"),
                        _ => p.to_string(),
                    })
                })
                .unwrap_or_default();
            let key = if c.primary_key { "PRI" } else { "" };
            let binary = c.full_type.to_lowercase().contains("binary")
                || c.full_type.to_lowercase().contains("blob")
                || c.collation
                    .as_deref()
                    .map(|x| x.to_lowercase().contains("bin"))
                    .unwrap_or(false);
            let default_cell = match &c.default {
                Some(d) => GridCell::text(d.clone()),
                None if c.nullable => GridCell {
                    display: "NULL".into(),
                    is_null: true,
                    is_numeric: false,
                },
                None => GridCell::text(""),
            };
            vec![
                GridCell::text(c.name.clone()),
                GridCell::text(c.full_type.clone()),
                GridCell::text(length),
                GridCell::text(yn(c.unsigned)),
                GridCell::text(yn(c.zerofill)),
                GridCell::text(yn(binary)),
                GridCell::text(if c.nullable { "YES" } else { "NO" }),
                GridCell::text(key),
                default_cell,
                GridCell::text(c.extra.clone().unwrap_or_else(|| {
                    if c.auto_increment {
                        "auto_increment".into()
                    } else {
                        String::new()
                    }
                })),
                GridCell::text(c.charset.clone().unwrap_or_default()),
                GridCell::text(c.collation.clone().unwrap_or_default()),
                GridCell::text(c.comment.clone().unwrap_or_default()),
            ]
        })
        .collect();
    (headers, rows)
}

pub fn index_rows(indexes: &[IndexMeta]) -> (Vec<String>, Vec<Vec<GridCell>>) {
    let headers = vec![
        "Key Name".into(),
        "Type".into(),
        "Unique".into(),
        "Columns".into(),
        "Comment".into(),
    ];
    let rows = indexes
        .iter()
        .map(|i| {
            let cols = i
                .columns
                .iter()
                .map(|c| match c.sub_part {
                    Some(n) => format!("{}({n})", c.name),
                    None => c.name.clone(),
                })
                .collect::<Vec<_>>()
                .join(", ");
            vec![
                GridCell::text(i.name.clone()),
                GridCell::text(i.index_type.clone()),
                GridCell::text(if i.unique || i.primary { "YES" } else { "NO" }),
                GridCell::text(cols),
                GridCell::text(i.comment.clone().unwrap_or_default()),
            ]
        })
        .collect();
    (headers, rows)
}

pub fn relation_rows(fks: &[ForeignKeyMeta]) -> (Vec<String>, Vec<Vec<GridCell>>) {
    let headers = vec![
        "Name".into(),
        "Columns".into(),
        "References".into(),
        "On Update".into(),
        "On Delete".into(),
    ];
    let rows = fks
        .iter()
        .map(|fk| {
            let refs = match fk.ref_database.as_deref().filter(|s| !s.is_empty()) {
                Some(db) => format!("{db}.{} ({})", fk.ref_table, fk.ref_columns.join(", ")),
                None => format!("{} ({})", fk.ref_table, fk.ref_columns.join(", ")),
            };
            vec![
                GridCell::text(fk.name.clone()),
                GridCell::text(fk.columns.join(", ")),
                GridCell::text(refs),
                GridCell::text(fk.on_update.clone()),
                GridCell::text(fk.on_delete.clone()),
            ]
        })
        .collect();
    (headers, rows)
}

pub fn trigger_rows(triggers: &[TriggerMeta]) -> (Vec<String>, Vec<Vec<GridCell>>) {
    let headers = vec![
        "Name".into(),
        "Timing".into(),
        "Event".into(),
        "Statement".into(),
        "Definer".into(),
    ];
    let rows = triggers
        .iter()
        .map(|t| {
            vec![
                GridCell::text(t.name.clone()),
                GridCell::text(t.timing.clone()),
                GridCell::text(t.event.clone()),
                GridCell::text(t.statement.clone()),
                GridCell::text(t.definer.clone().unwrap_or_default()),
            ]
        })
        .collect();
    (headers, rows)
}

fn yn(v: bool) -> &'static str {
    if v {
        "YES"
    } else {
        ""
    }
}

pub struct MultiQueryOutput {
    pub results: Vec<QueryResult>,
    pub total_affected: u64,
    pub total_duration_ms: u64,
    pub statement_count: usize,
}

pub async fn run_sql_multi(
    conn: &Arc<dyn Connection>,
    sql: &str,
) -> AceResult<MultiQueryOutput> {
    let statements: Vec<String> = ace_sql::split_statements(sql)
        .into_iter()
        .map(|s| s.sql)
        .filter(|s| !s.trim().is_empty())
        .collect();
    if statements.is_empty() {
        return Ok(MultiQueryOutput {
            results: Vec::new(),
            total_affected: 0,
            total_duration_ms: 0,
            statement_count: 0,
        });
    }
    let count = statements.len();
    let results = conn
        .execute_many(
            &statements,
            QueryOptions {
                max_rows: Some(2000),
                ..Default::default()
            },
            CancelToken::new(),
        )
        .await?;
    let total_affected = results.iter().map(|r| r.affected_rows).sum();
    let total_duration_ms = results.iter().map(|r| r.duration_ms).sum();
    Ok(MultiQueryOutput {
        results,
        total_affected,
        total_duration_ms,
        statement_count: count,
    })
}

pub async fn exec_single_sql(conn: &Arc<dyn Connection>, sql: &str) -> AceResult<QueryResult> {
    conn.execute(sql, QueryOptions::default(), CancelToken::new())
        .await
}

pub async fn export_data(
    path: &str,
    result: &QueryResult,
    table: Option<&str>,
    format: ExportFormat,
    gzip: bool,
) -> AceResult<u64> {
    let path_str = expand_home(path);
    let opts = ExportOptions {
        format,
        gzip,
        ..Default::default()
    };
    let result = result.clone();
    let table = table.map(|s| s.to_string());
    tokio::task::spawn_blocking(move || {
        export_query_result(Path::new(&path_str), &result, table.as_deref(), &opts)
    })
    .await
    .map_err(|e| AceError::Io(e.to_string()))?
}

pub async fn import_sql_file(conn: &Arc<dyn Connection>, path: &str) -> AceResult<(usize, u64)> {
    let path_str = expand_home(path);
    let content = tokio::fs::read_to_string(&path_str)
        .await
        .map_err(|e| AceError::Io(format!("Failed to read {path_str}: {e}")))?;
    let statements: Vec<String> = ace_sql::split_statements(&content)
        .into_iter()
        .map(|s| s.sql)
        .filter(|s| !s.trim().is_empty())
        .collect();
    let started = Instant::now();
    let mut executed = 0usize;
    for stmt in &statements {
        conn.execute(stmt, QueryOptions::default(), CancelToken::new())
            .await?;
        executed += 1;
    }
    Ok((executed, started.elapsed().as_millis() as u64))
}

pub fn expand_home(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return std::path::Path::new(&home).join(rest).display().to_string();
        }
    }
    path.to_string()
}

pub fn query_result_to_cells(res: &QueryResult) -> (Vec<String>, Vec<Vec<GridCell>>) {
    let cols = res.columns.iter().map(|c| c.name.clone()).collect();
    let rows = res
        .rows
        .iter()
        .map(|r| r.iter().map(GridCell::from_value).collect())
        .collect();
    (cols, rows)
}
