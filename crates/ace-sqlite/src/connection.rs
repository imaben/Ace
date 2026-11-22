use ace_core::{AceError, AceResult, ConnectionInfo, Value};
use ace_driver::dialect::{Dialect, SqliteDialect};
use ace_driver::meta::*;
use ace_driver::query::{CancelToken, QueryOptions};
use ace_driver::result::{QueryResult, ResultColumn, StreamingQueryResult};
use ace_driver::Connection;
use async_trait::async_trait;
use futures::stream;
use rusqlite::types::ValueRef;
use rusqlite::Connection as RuConnection;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use uuid::Uuid;

pub struct SqliteConnection {
    id: Uuid,
    info: ConnectionInfo,
    conn: Mutex<RuConnection>,
    dialect: Arc<dyn Dialect>,
}

impl SqliteConnection {
    pub async fn connect(info: &ConnectionInfo) -> AceResult<Self> {
        let path = info.database.as_deref().unwrap_or(":memory:");
        let path = path.to_string();
        let conn = tokio::task::spawn_blocking(move || RuConnection::open(path))
            .await
            .map_err(|e| AceError::Connection(e.to_string()))?
            .map_err(|e| AceError::Connection(e.to_string()))?;
        Ok(Self {
            id: Uuid::new_v4(),
            info: info.clone(),
            conn: Mutex::new(conn),
            dialect: Arc::new(SqliteDialect),
        })
    }
}

fn value_ref_to_ace(v: ValueRef<'_>) -> Value {
    match v {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(i) => Value::I64(i),
        ValueRef::Real(f) => Value::F64(f),
        ValueRef::Text(t) => Value::String(String::from_utf8_lossy(t).into_owned()),
        ValueRef::Blob(b) => Value::Bytes(b.to_vec()),
    }
}

fn query_sqlite_indexes(conn: &RuConnection, table: &str) -> AceResult<Vec<IndexMeta>> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA index_list(\"{}\")", table.replace('"', "\"\"")))
        .map_err(|e| AceError::Query(e.to_string()))?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, String>(3).unwrap_or_default(),
            ))
        })
        .map_err(|e| AceError::Query(e.to_string()))?;
    let mut list = Vec::new();
    for r in rows {
        list.push(r.map_err(|e| AceError::Query(e.to_string()))?);
    }
    let mut out = Vec::new();
    for (name, unique, origin) in list {
        let mut cols = Vec::new();
        if let Ok(mut cs) = conn.prepare(&format!("PRAGMA index_info(\"{}\")", name.replace('"', "\"\""))) {
            if let Ok(crows) = cs.query_map([], |r| r.get::<_, String>(2)) {
                for cr in crows.flatten() {
                    cols.push(IndexColumn {
                        name: cr,
                        sub_part: None,
                        descending: false,
                    });
                }
            }
        }
        out.push(IndexMeta {
            name,
            columns: cols,
            unique: unique != 0,
            primary: origin == "pk",
            index_type: "BTREE".into(),
            comment: None,
        });
    }
    Ok(out)
}

fn query_sqlite_foreign_keys(conn: &RuConnection, table: &str) -> AceResult<Vec<ForeignKeyMeta>> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA foreign_key_list(\"{}\")", table.replace('"', "\"\"")))
        .map_err(|e| AceError::Query(e.to_string()))?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, String>(6)?,
            ))
        })
        .map_err(|e| AceError::Query(e.to_string()))?;
    let mut out: Vec<ForeignKeyMeta> = Vec::new();
    for r in rows {
        let (id, ref_table, col, ref_col, on_update, on_delete) =
            r.map_err(|e| AceError::Query(e.to_string()))?;
        let name = format!("fk_{table}_{id}");
        if let Some(existing) = out.iter_mut().find(|f| f.name == name) {
            existing.columns.push(col);
            existing.ref_columns.push(ref_col);
        } else {
            out.push(ForeignKeyMeta {
                name,
                columns: vec![col],
                ref_database: Some("main".into()),
                ref_table,
                ref_columns: vec![ref_col],
                on_update,
                on_delete,
            });
        }
    }
    Ok(out)
}

#[async_trait]
impl Connection for SqliteConnection {
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
        let conn = self.conn.lock().map_err(|e| AceError::Connection(e.to_string()))?;
        conn.execute_batch("SELECT 1")
            .map_err(|e| AceError::Connection(e.to_string()))
    }

    async fn close(&self) -> AceResult<()> {
        Ok(())
    }

    async fn server_info(&self) -> AceResult<ServerInfo> {
        let conn = self.conn.lock().map_err(|e| AceError::Connection(e.to_string()))?;
        let version: String = conn
            .query_row("SELECT sqlite_version()", [], |r| r.get(0))
            .map_err(|e| AceError::Query(e.to_string()))?;
        Ok(ServerInfo {
            version,
            version_comment: String::new(),
            host: String::new(),
            port: 0,
            current_user: String::new(),
            current_database: self.info.database.clone(),
            charset: "UTF-8".into(),
            engine_name: "SQLite".into(),
        })
    }

    async fn list_databases(&self) -> AceResult<Vec<String>> {
        let conn = self.conn.lock().map_err(|e| AceError::Connection(e.to_string()))?;
        let mut stmt = conn
            .prepare("PRAGMA database_list")
            .map_err(|e| AceError::Query(e.to_string()))?;
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(1))
            .map_err(|e| AceError::Query(e.to_string()))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| AceError::Query(e.to_string()))?);
        }
        Ok(out)
    }

    async fn use_database(&self, _name: &str) -> AceResult<()> {
        Ok(())
    }

    async fn current_database(&self) -> AceResult<Option<String>> {
        Ok(Some("main".into()))
    }

    async fn list_objects(&self, _database: &str) -> AceResult<Vec<DatabaseObject>> {
        let conn = self.conn.lock().map_err(|e| AceError::Connection(e.to_string()))?;
        let mut stmt = conn
            .prepare(
                "SELECT name, type FROM sqlite_master WHERE type IN ('table','view') \
                 AND name NOT LIKE 'sqlite_%' ORDER BY name",
            )
            .map_err(|e| AceError::Query(e.to_string()))?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
            .map_err(|e| AceError::Query(e.to_string()))?;
        let mut out = Vec::new();
        for r in rows {
            let (name, kind) = r.map_err(|e| AceError::Query(e.to_string()))?;
            out.push(DatabaseObject {
                name,
                kind: if kind == "view" {
                    ObjectKind::View
                } else {
                    ObjectKind::Table
                },
                engine: Some("SQLite".into()),
                rows_estimate: None,
                data_length: None,
                comment: None,
                pinned: false,
            });
        }
        Ok(out)
    }

    async fn table_meta(&self, database: &str, table: &str) -> AceResult<TableMeta> {
        let conn = self.conn.lock().map_err(|e| AceError::Connection(e.to_string()))?;
        let mut stmt = conn
            .prepare(&format!("PRAGMA table_info(\"{}\")", table.replace('"', "\"\"")))
            .map_err(|e| AceError::Query(e.to_string()))?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, i64>(3)?,
                    r.get::<_, Option<String>>(4)?,
                    r.get::<_, i64>(5)?,
                ))
            })
            .map_err(|e| AceError::Query(e.to_string()))?;
        let mut columns = Vec::new();
        for (i, r) in rows.enumerate() {
            let (name, dtype, notnull, default, pk) =
                r.map_err(|e| AceError::Query(e.to_string()))?;
            columns.push(ColumnMeta {
                name,
                data_type: dtype.clone(),
                full_type: dtype,
                nullable: notnull == 0,
                primary_key: pk != 0,
                auto_increment: false,
                default,
                extra: None,
                comment: None,
                charset: None,
                collation: None,
                unsigned: false,
                zerofill: false,
                character_maximum_length: None,
                numeric_precision: None,
                numeric_scale: None,
                ordinal_position: (i + 1) as u32,
            });
        }
        let create_sql: Option<String> = conn
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type IN ('table','view') AND name=?1",
                [table],
                |r| r.get(0),
            )
            .ok();
        let indexes = query_sqlite_indexes(&conn, table).unwrap_or_default();
        let foreign_keys = query_sqlite_foreign_keys(&conn, table).unwrap_or_default();
        Ok(TableMeta {
            name: table.to_string(),
            database: database.to_string(),
            engine: Some("SQLite".into()),
            charset: Some("UTF-8".into()),
            collation: Some("BINARY".into()),
            comment: None,
            auto_increment: None,
            create_options: None,
            columns,
            indexes,
            foreign_keys,
            create_sql,
        })
    }

    async fn list_indexes(&self, _database: &str, table: &str) -> AceResult<Vec<IndexMeta>> {
        let conn = self.conn.lock().map_err(|e| AceError::Connection(e.to_string()))?;
        query_sqlite_indexes(&conn, table)
    }
    async fn list_foreign_keys(
        &self,
        _database: &str,
        table: &str,
    ) -> AceResult<Vec<ForeignKeyMeta>> {
        let conn = self.conn.lock().map_err(|e| AceError::Connection(e.to_string()))?;
        query_sqlite_foreign_keys(&conn, table)
    }
    async fn list_triggers(&self, _database: &str, table: &str) -> AceResult<Vec<TriggerMeta>> {
        let conn = self.conn.lock().map_err(|e| AceError::Connection(e.to_string()))?;
        let mut stmt = conn
            .prepare("SELECT name, sql FROM sqlite_master WHERE type='trigger' AND tbl_name=?1 ORDER BY name")
            .map_err(|e| AceError::Query(e.to_string()))?;
        let rows = stmt
            .query_map([table], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?))
            })
            .map_err(|e| AceError::Query(e.to_string()))?;
        let mut out = Vec::new();
        for r in rows {
            let (name, sql) = r.map_err(|e| AceError::Query(e.to_string()))?;
            let statement = sql.unwrap_or_default();
            let upper = statement.to_uppercase();
            let timing = if upper.contains("BEFORE") {
                "BEFORE"
            } else if upper.contains("INSTEAD OF") {
                "INSTEAD OF"
            } else {
                "AFTER"
            };
            let event = if upper.contains("INSERT") {
                "INSERT"
            } else if upper.contains("UPDATE") {
                "UPDATE"
            } else if upper.contains("DELETE") {
                "DELETE"
            } else {
                ""
            };
            out.push(TriggerMeta {
                name,
                timing: timing.into(),
                event: event.into(),
                statement,
                definer: None,
            });
        }
        Ok(out)
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
        let conn = self.conn.lock().map_err(|e| AceError::Connection(e.to_string()))?;
        let mut stmt = conn
            .prepare(sql)
            .map_err(|e| AceError::Query(e.to_string()))?;
        let column_count = stmt.column_count();
        let columns: Vec<ResultColumn> = (0..column_count)
            .map(|i| ResultColumn {
                name: stmt.column_name(i).unwrap_or("?").to_string(),
                table: None,
                database: None,
                data_type: String::new(),
                nullable: true,
                primary_key: false,
                org_name: None,
            })
            .collect();

        if columns.is_empty() {
            let affected = conn
                .execute(sql, [])
                .map_err(|e| AceError::Query(e.to_string()))? as u64;
            return Ok(QueryResult {
                columns: Vec::new(),
                rows: Vec::new(),
                affected_rows: affected,
                last_insert_id: Some(conn.last_insert_rowid() as u64),
                warnings: 0,
                info: String::new(),
                duration_ms: started.elapsed().as_millis() as u64,
            });
        }

        let max = options.max_rows.unwrap_or(usize::MAX);
        let mut rows = Vec::new();
        let mut rows_iter = stmt
            .query([])
            .map_err(|e| AceError::Query(e.to_string()))?;
        while let Some(row) = rows_iter
            .next()
            .map_err(|e| AceError::Query(e.to_string()))?
        {
            if rows.len() >= max {
                break;
            }
            let mut values = Vec::with_capacity(column_count);
            for i in 0..column_count {
                values.push(value_ref_to_ace(row.get_ref(i).map_err(|e| AceError::Query(e.to_string()))?));
            }
            rows.push(values);
        }
        Ok(QueryResult {
            columns,
            rows,
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
        Ok(Vec::new())
    }
    async fn kill_query(&self, _process_id: u64) -> AceResult<()> {
        Err(AceError::NotSupported("SQLite has no process list".into()))
    }
    async fn kill_connection(&self, _process_id: u64) -> AceResult<()> {
        Err(AceError::NotSupported("SQLite has no process list".into()))
    }
    async fn server_variables(&self) -> AceResult<HashMap<String, String>> {
        Ok(HashMap::new())
    }
    async fn list_users(&self) -> AceResult<Vec<UserPrivilege>> {
        Ok(Vec::new())
    }
}