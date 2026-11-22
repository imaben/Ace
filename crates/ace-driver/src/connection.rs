use crate::capability::Capabilities;
use crate::dialect::Dialect;
use crate::meta::{
    DatabaseObject, ForeignKeyMeta, IndexMeta, ProcessInfo, ServerInfo, TableMeta, TriggerMeta,
    UserPrivilege,
};
use crate::query::{CancelToken, QueryOptions};
use crate::result::{QueryResult, StreamingQueryResult};
use ace_core::{AceResult, ConnectionInfo, Value};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

#[async_trait]
pub trait Connection: Send + Sync {
    fn id(&self) -> uuid::Uuid;
    fn info(&self) -> &ConnectionInfo;
    fn dialect(&self) -> Arc<dyn Dialect>;
    fn capabilities(&self) -> Capabilities {
        self.dialect().capabilities()
    }

    async fn ping(&self) -> AceResult<()>;
    async fn close(&self) -> AceResult<()>;
    async fn server_info(&self) -> AceResult<ServerInfo>;

    async fn list_databases(&self) -> AceResult<Vec<String>>;
    async fn use_database(&self, name: &str) -> AceResult<()>;
    async fn current_database(&self) -> AceResult<Option<String>>;

    async fn list_objects(&self, database: &str) -> AceResult<Vec<DatabaseObject>>;
    async fn table_meta(&self, database: &str, table: &str) -> AceResult<TableMeta>;
    async fn list_indexes(&self, database: &str, table: &str) -> AceResult<Vec<IndexMeta>>;
    async fn list_foreign_keys(
        &self,
        database: &str,
        table: &str,
    ) -> AceResult<Vec<ForeignKeyMeta>>;
    async fn list_triggers(&self, database: &str, table: &str) -> AceResult<Vec<TriggerMeta>>;

    async fn execute(
        &self,
        sql: &str,
        options: QueryOptions,
        cancel: CancelToken,
    ) -> AceResult<QueryResult>;

    async fn execute_many(
        &self,
        statements: &[String],
        options: QueryOptions,
        cancel: CancelToken,
    ) -> AceResult<Vec<QueryResult>> {
        let mut out = Vec::with_capacity(statements.len());
        for sql in statements {
            if cancel.is_cancelled() {
                return Err(ace_core::AceError::Cancelled);
            }
            out.push(self.execute(sql, options.clone(), cancel.child()).await?);
        }
        Ok(out)
    }

    async fn stream_query(
        &self,
        sql: &str,
        options: QueryOptions,
        cancel: CancelToken,
    ) -> AceResult<StreamingQueryResult>;

    async fn process_list(&self) -> AceResult<Vec<ProcessInfo>>;
    async fn kill_query(&self, process_id: u64) -> AceResult<()>;
    async fn kill_connection(&self, process_id: u64) -> AceResult<()>;
    async fn server_variables(&self) -> AceResult<HashMap<String, String>>;
    async fn list_users(&self) -> AceResult<Vec<UserPrivilege>>;

    /// Connection-thread id used for KILL QUERY from a sibling connection.
    async fn thread_id(&self) -> AceResult<Option<u64>> {
        Ok(None)
    }

    async fn fetch_page(
        &self,
        database: &str,
        table: &str,
        where_clause: Option<&str>,
        order_by: Option<&str>,
        limit: usize,
        offset: usize,
        cancel: CancelToken,
    ) -> AceResult<QueryResult> {
        let dialect = self.dialect();
        let mut sql = dialect.select_all_sql(database, table, where_clause);
        if let Some(ob) = order_by {
            if !ob.trim().is_empty() {
                sql.push_str(" ORDER BY ");
                sql.push_str(ob);
            }
        }
        sql.push(' ');
        sql.push_str(&dialect.limit_offset(limit, offset));
        self.execute(&sql, QueryOptions::default(), cancel).await
    }

    async fn count_rows(
        &self,
        database: &str,
        table: &str,
        where_clause: Option<&str>,
        cancel: CancelToken,
    ) -> AceResult<u64> {
        let sql = self.dialect().count_sql(database, table, where_clause);
        let result = self.execute(&sql, QueryOptions::default(), cancel).await?;
        match result.rows.first().and_then(|r| r.first()) {
            Some(Value::I64(v)) => Ok(*v as u64),
            Some(Value::U64(v)) => Ok(*v),
            Some(Value::String(s)) => Ok(s.parse().unwrap_or(0)),
            _ => Ok(0),
        }
    }
}