use ace_core::{AceError, AceResult, ConnectionInfo, Value};
use ace_driver::dialect::{Dialect, PostgresDialect};
use ace_driver::meta::*;
use ace_driver::query::{CancelToken, QueryOptions};
use ace_driver::result::{QueryResult, ResultColumn, StreamingQueryResult};
use ace_driver::Connection;
use async_trait::async_trait;
use futures::stream;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use tokio_postgres::{Client, NoTls, Row};
use uuid::Uuid;

pub struct PostgresConnection {
    id: Uuid,
    info: ConnectionInfo,
    client: Mutex<Client>,
    dialect: Arc<dyn Dialect>,
}

impl PostgresConnection {
    pub async fn connect(info: &ConnectionInfo) -> AceResult<Self> {
        let db = info.database.as_deref().unwrap_or("postgres");
        let conn_str = format!(
            "host={} port={} user={} password={} dbname={}",
            info.host,
            info.port,
            info.user,
            info.password.as_deref().unwrap_or(""),
            db
        );
        let (client, connection) = tokio_postgres::connect(&conn_str, NoTls)
            .await
            .map_err(|e| AceError::Connection(e.to_string()))?;
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                tracing::error!("postgres connection error: {e}");
            }
        });
        Ok(Self {
            id: Uuid::new_v4(),
            info: info.clone(),
            client: Mutex::new(client),
            dialect: Arc::new(PostgresDialect),
        })
    }
}

fn row_values(row: &Row) -> Vec<Value> {
    let mut out = Vec::with_capacity(row.len());
    for i in 0..row.len() {
        if let Ok(v) = row.try_get::<_, Option<String>>(i) {
            out.push(match v {
                None => Value::Null,
                Some(s) => Value::String(s),
            });
            continue;
        }
        if let Ok(v) = row.try_get::<_, Option<i64>>(i) {
            out.push(match v {
                None => Value::Null,
                Some(n) => Value::I64(n),
            });
            continue;
        }
        if let Ok(v) = row.try_get::<_, Option<f64>>(i) {
            out.push(match v {
                None => Value::Null,
                Some(n) => Value::F64(n),
            });
            continue;
        }
        if let Ok(v) = row.try_get::<_, Option<bool>>(i) {
            out.push(match v {
                None => Value::Null,
                Some(b) => Value::Bool(b),
            });
            continue;
        }
        out.push(Value::String(format!("<col {i}>")));
    }
    out
}

#[async_trait]
impl Connection for PostgresConnection {
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
        let client = self.client.lock().await;
        client
            .simple_query("SELECT 1")
            .await
            .map_err(|e| AceError::Connection(e.to_string()))?;
        Ok(())
    }

    async fn close(&self) -> AceResult<()> {
        Ok(())
    }

    async fn server_info(&self) -> AceResult<ServerInfo> {
        let client = self.client.lock().await;
        let row = client
            .query_one("SELECT version(), current_user, current_database()", &[])
            .await
            .map_err(|e| AceError::Query(e.to_string()))?;
        Ok(ServerInfo {
            version: row.get(0),
            version_comment: String::new(),
            host: self.info.host.clone(),
            port: self.info.port,
            current_user: row.get(1),
            current_database: row.get(2),
            charset: "UTF8".into(),
            engine_name: "PostgreSQL".into(),
        })
    }

    async fn list_databases(&self) -> AceResult<Vec<String>> {
        let client = self.client.lock().await;
        let rows = client
            .query(
                "SELECT datname FROM pg_database WHERE datistemplate = false ORDER BY 1",
                &[],
            )
            .await
            .map_err(|e| AceError::Query(e.to_string()))?;
        Ok(rows.iter().map(|r| r.get(0)).collect())
    }

    async fn use_database(&self, _name: &str) -> AceResult<()> {
        Err(AceError::NotSupported(
            "PostgreSQL requires a new connection to change database".into(),
        ))
    }

    async fn current_database(&self) -> AceResult<Option<String>> {
        let client = self.client.lock().await;
        let row = client
            .query_one("SELECT current_database()", &[])
            .await
            .map_err(|e| AceError::Query(e.to_string()))?;
        Ok(Some(row.get(0)))
    }

    async fn list_objects(&self, schema: &str) -> AceResult<Vec<DatabaseObject>> {
        let client = self.client.lock().await;
        let rows = client
            .query(
                "SELECT table_name, table_type FROM information_schema.tables \
                 WHERE table_schema = $1 ORDER BY 1",
                &[&schema],
            )
            .await
            .map_err(|e| AceError::Query(e.to_string()))?;
        Ok(rows
            .iter()
            .map(|r| {
                let name: String = r.get(0);
                let t: String = r.get(1);
                DatabaseObject {
                    name,
                    kind: if t.contains("VIEW") {
                        ObjectKind::View
                    } else {
                        ObjectKind::Table
                    },
                    engine: None,
                    rows_estimate: None,
                    data_length: None,
                    comment: None,
                    pinned: false,
                }
            })
            .collect())
    }

    async fn table_meta(&self, schema: &str, table: &str) -> AceResult<TableMeta> {
        let client = self.client.lock().await;
        let rows = client
            .query(
                "SELECT column_name, data_type, is_nullable, column_default, ordinal_position \
                 FROM information_schema.columns \
                 WHERE table_schema = $1 AND table_name = $2 ORDER BY ordinal_position",
                &[&schema, &table],
            )
            .await
            .map_err(|e| AceError::Query(e.to_string()))?;
        let columns = rows
            .iter()
            .map(|r| {
                let nullable: String = r.get(2);
                ColumnMeta {
                    name: r.get(0),
                    data_type: r.get(1),
                    full_type: r.get(1),
                    nullable: nullable.eq_ignore_ascii_case("YES"),
                    primary_key: false,
                    auto_increment: false,
                    default: r.get(3),
                    extra: None,
                    comment: None,
                    charset: None,
                    collation: None,
                    unsigned: false,
                    zerofill: false,
                    character_maximum_length: None,
                    numeric_precision: None,
                    numeric_scale: None,
                    ordinal_position: {
                        let n: i32 = r.get(4);
                        n as u32
                    },
                }
            })
            .collect();
        Ok(TableMeta {
            name: table.to_string(),
            database: schema.to_string(),
            engine: None,
            charset: None,
            collation: None,
            comment: None,
            auto_increment: None,
            create_options: None,
            columns,
            indexes: Vec::new(),
            foreign_keys: Vec::new(),
            create_sql: None,
        })
    }

    async fn list_indexes(&self, _database: &str, _table: &str) -> AceResult<Vec<IndexMeta>> {
        Ok(Vec::new())
    }
    async fn list_foreign_keys(
        &self,
        _database: &str,
        _table: &str,
    ) -> AceResult<Vec<ForeignKeyMeta>> {
        Ok(Vec::new())
    }
    async fn list_triggers(&self, _database: &str, _table: &str) -> AceResult<Vec<TriggerMeta>> {
        Ok(Vec::new())
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
        let started = Instant::now();
        let client = self.client.lock().await;
        let rows = client
            .query(sql, &[])
            .await
            .map_err(|e| AceError::Query(e.to_string()))?;
        let columns = if let Some(first) = rows.first() {
            (0..first.columns().len())
                .map(|i| {
                    let col = &first.columns()[i];
                    ResultColumn {
                        name: col.name().to_string(),
                        table: None,
                        database: None,
                        data_type: format!("{:?}", col.type_()),
                        nullable: true,
                        primary_key: false,
                        org_name: None,
                    }
                })
                .collect()
        } else {
            Vec::new()
        };
        let max = options.max_rows.unwrap_or(usize::MAX);
        let data: Vec<_> = rows.iter().take(max).map(row_values).collect();
        Ok(QueryResult {
            columns,
            rows: data,
            affected_rows: 0,
            last_insert_id: None,
            warnings: 0,
            info: String::new(),
            duration_ms: started.elapsed().as_millis() as u64,
        })
    }

    async fn stream_query(
        &self,
        sql: &str,
        options: QueryOptions,
        cancel: CancelToken,
    ) -> AceResult<StreamingQueryResult> {
        let result = self.execute(sql, options, cancel).await?;
        Ok(StreamingQueryResult {
            columns: result.columns.clone(),
            stream: Box::pin(stream::iter(result.rows.into_iter().map(Ok))),
            affected_rows: result.affected_rows,
        })
    }

    async fn process_list(&self) -> AceResult<Vec<ProcessInfo>> {
        let client = self.client.lock().await;
        let rows = client
            .query(
                "SELECT pid, usename, coalesce(client_addr::text,''), datname, state, \
                 coalesce(EXTRACT(EPOCH FROM (now() - query_start))::bigint,0), query \
                 FROM pg_stat_activity",
                &[],
            )
            .await
            .map_err(|e| AceError::Query(e.to_string()))?;
        Ok(rows
            .iter()
            .map(|r| ProcessInfo {
                id: r.get::<_, i32>(0) as u64,
                user: r.get(1),
                host: r.get(2),
                database: r.get(3),
                command: r.get::<_, Option<String>>(4).unwrap_or_default(),
                time_secs: r.get::<_, i64>(5) as u64,
                state: r.get(4),
                info: r.get(6),
            })
            .collect())
    }

    async fn kill_query(&self, process_id: u64) -> AceResult<()> {
        let client = self.client.lock().await;
        client
            .execute("SELECT pg_cancel_backend($1)", &[&(process_id as i32)])
            .await
            .map_err(|e| AceError::Query(e.to_string()))?;
        Ok(())
    }

    async fn kill_connection(&self, process_id: u64) -> AceResult<()> {
        let client = self.client.lock().await;
        client
            .execute("SELECT pg_terminate_backend($1)", &[&(process_id as i32)])
            .await
            .map_err(|e| AceError::Query(e.to_string()))?;
        Ok(())
    }

    async fn server_variables(&self) -> AceResult<HashMap<String, String>> {
        let client = self.client.lock().await;
        let rows = client
            .query("SHOW ALL", &[])
            .await
            .map_err(|e| AceError::Query(e.to_string()))?;
        let mut map = HashMap::new();
        for r in rows {
            map.insert(r.get(0), r.get(1));
        }
        Ok(map)
    }

    async fn list_users(&self) -> AceResult<Vec<UserPrivilege>> {
        let client = self.client.lock().await;
        let rows = client
            .query("SELECT usename FROM pg_user ORDER BY 1", &[])
            .await
            .map_err(|e| AceError::Query(e.to_string()))?;
        Ok(rows
            .iter()
            .map(|r| UserPrivilege {
                user: r.get(0),
                host: String::new(),
                global_privileges: Vec::new(),
                database_privileges: Vec::new(),
                max_questions: None,
                max_updates: None,
                max_connections: None,
                max_user_connections: None,
            })
            .collect())
    }
}