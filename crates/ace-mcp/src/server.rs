use crate::tools::ToolName;
use ace_core::{AceError, AceResult, McpSettings};
use ace_driver::meta::ObjectKind;
use ace_driver::query::{CancelToken, QueryOptions};
use ace_driver::Connection;
use ace_sql::is_read_only;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::oneshot;
use tracing::info;
use uuid::Uuid;

pub struct McpState {
    pub settings: McpSettings,
    pub connections: RwLock<HashMap<Uuid, Arc<dyn Connection>>>,
    pub connection_labels: RwLock<HashMap<Uuid, String>>,
}

impl McpState {
    pub fn new(settings: McpSettings) -> Self {
        Self {
            settings,
            connections: RwLock::new(HashMap::new()),
            connection_labels: RwLock::new(HashMap::new()),
        }
    }

    pub fn register(&self, conn: Arc<dyn Connection>, label: impl Into<String>) {
        let id = conn.id();
        self.connection_labels.write().insert(id, label.into());
        self.connections.write().insert(id, conn);
    }

    pub fn unregister(&self, id: Uuid) {
        self.connections.write().remove(&id);
        self.connection_labels.write().remove(&id);
    }
}

pub struct McpServerHandle {
    shutdown: Option<oneshot::Sender<()>>,
}

impl McpServerHandle {
    pub fn stop(mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
    }
}

pub struct McpServer;

impl McpServer {
    pub async fn start(state: Arc<McpState>) -> AceResult<McpServerHandle> {
        let host = state.settings.bind_host.clone();
        let port = state.settings.bind_port;
        let app = Router::new()
            .route("/health", get(|| async { "ok" }))
            .route("/mcp", post(handle_mcp))
            .route("/tools", get(list_tools))
            .with_state(state);

        let addr: SocketAddr = format!("{host}:{port}")
            .parse()
            .map_err(|e| AceError::Message(format!("bad bind addr: {e}")))?;

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| AceError::Io(e.to_string()))?;
        info!("MCP server listening on http://{addr}");

        let (tx, rx) = oneshot::channel::<()>();
        tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = rx.await;
                })
                .await
                .ok();
        });

        Ok(McpServerHandle {
            shutdown: Some(tx),
        })
    }
}

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[serde(default)]
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: &'static str,
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<Value>,
}

async fn list_tools(State(state): State<Arc<McpState>>) -> Json<Value> {
    let tools: Vec<Value> = ToolName::all()
        .iter()
        .filter(|t| !(state.settings.read_only && t.is_mutating() && **t != ToolName::RunQuery))
        .map(|t| {
            json!({
                "name": t.as_str(),
                "description": t.description(),
            })
        })
        .collect();
    Json(json!({ "tools": tools }))
}

async fn handle_mcp(
    State(state): State<Arc<McpState>>,
    headers: HeaderMap,
    Json(req): Json<JsonRpcRequest>,
) -> (StatusCode, Json<JsonRpcResponse>) {
    if let Some(expected) = &state.settings.auth_token {
        let got = headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let got = got.strip_prefix("Bearer ").unwrap_or(got);
        if got != expected {
            return (
                StatusCode::UNAUTHORIZED,
                Json(JsonRpcResponse {
                    jsonrpc: "2.0",
                    id: req.id,
                    result: None,
                    error: Some(json!({"code": 401, "message": "unauthorized"})),
                }),
            );
        }
    }

    let result = match req.method.as_str() {
        "tools/list" | "list_tools" => Ok(json!({
            "tools": ToolName::all().iter().map(|t| json!({
                "name": t.as_str(),
                "description": t.description(),
            })).collect::<Vec<_>>()
        })),
        "tools/call" | "call_tool" => call_tool(&state, &req.params).await,
        other => Err(format!("unknown method: {other}")),
    };

    match result {
        Ok(value) => (
            StatusCode::OK,
            Json(JsonRpcResponse {
                jsonrpc: "2.0",
                id: req.id,
                result: Some(value),
                error: None,
            }),
        ),
        Err(msg) => (
            StatusCode::OK,
            Json(JsonRpcResponse {
                jsonrpc: "2.0",
                id: req.id,
                result: None,
                error: Some(json!({"code": -32000, "message": msg})),
            }),
        ),
    }
}

async fn call_tool(state: &McpState, params: &Value) -> Result<Value, String> {
    let name = params
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or("missing tool name")?;
    let args = params.get("arguments").cloned().unwrap_or(json!({}));

    let conn_id = args
        .get("connection_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok());

    match name {
        "list_connections" => {
            let labels = state.connection_labels.read();
            let list: Vec<Value> = labels
                .iter()
                .map(|(id, label)| json!({"id": id.to_string(), "label": label}))
                .collect();
            Ok(json!({ "connections": list }))
        }
        "list_databases" => {
            let conn = require_conn(state, conn_id)?;
            let dbs = conn.list_databases().await.map_err(|e| e.to_string())?;
            Ok(json!({ "databases": dbs }))
        }
        "list_tables" | "list_views" | "list_procedures" | "list_functions" => {
            let conn = require_conn(state, conn_id)?;
            let db = args
                .get("database")
                .and_then(|v| v.as_str())
                .ok_or("missing database")?;
            let objects = conn.list_objects(db).await.map_err(|e| e.to_string())?;
            let kind = match name {
                "list_views" => ObjectKind::View,
                "list_procedures" => ObjectKind::Procedure,
                "list_functions" => ObjectKind::Function,
                _ => ObjectKind::Table,
            };
            let names: Vec<_> = objects
                .into_iter()
                .filter(|o| o.kind == kind || (name == "list_tables" && o.kind == ObjectKind::Table))
                .map(|o| o.name)
                .collect();
            Ok(json!({ "items": names }))
        }
        "describe_table" | "get_table_ddl" | "list_triggers" => {
            let conn = require_conn(state, conn_id)?;
            let db = args.get("database").and_then(|v| v.as_str()).ok_or("missing database")?;
            let table = args.get("table").and_then(|v| v.as_str()).ok_or("missing table")?;
            let meta = conn.table_meta(db, table).await.map_err(|e| e.to_string())?;
            if name == "get_table_ddl" {
                Ok(json!({ "ddl": meta.create_sql }))
            } else if name == "list_triggers" {
                let triggers = conn.list_triggers(db, table).await.map_err(|e| e.to_string())?;
                Ok(serde_json::to_value(triggers).unwrap_or(json!([])))
            } else {
                Ok(serde_json::to_value(meta.columns).unwrap_or(json!([])))
            }
        }
        "run_query" | "explain_query" => {
            let conn = require_conn(state, conn_id)?;
            let mut sql = args
                .get("sql")
                .and_then(|v| v.as_str())
                .ok_or("missing sql")?
                .to_string();
            if name == "explain_query" {
                sql = format!("EXPLAIN {sql}");
            }
            if state.settings.read_only && !is_read_only(&sql) {
                return Err("MCP server is in read-only mode".into());
            }
            let result = conn
                .execute(&sql, QueryOptions { max_rows: Some(1000), ..Default::default() }, CancelToken::new())
                .await
                .map_err(|e| e.to_string())?;
            Ok(json!({
                "columns": result.columns.iter().map(|c| &c.name).collect::<Vec<_>>(),
                "rows": result.rows.iter().map(|r| r.iter().map(|v| v.to_string()).collect::<Vec<_>>()).collect::<Vec<_>>(),
                "affected_rows": result.affected_rows,
                "duration_ms": result.duration_ms,
            }))
        }
        "sample_table" | "count_rows" => {
            let conn = require_conn(state, conn_id)?;
            let db = args.get("database").and_then(|v| v.as_str()).ok_or("missing database")?;
            let table = args.get("table").and_then(|v| v.as_str()).ok_or("missing table")?;
            if name == "count_rows" {
                let n = conn
                    .count_rows(db, table, None, CancelToken::new())
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(json!({ "count": n }))
            } else {
                let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
                let result = conn
                    .fetch_page(db, table, None, None, limit, 0, CancelToken::new())
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(json!({
                    "columns": result.columns.iter().map(|c| &c.name).collect::<Vec<_>>(),
                    "rows": result.rows.iter().map(|r| r.iter().map(|v| v.to_string()).collect::<Vec<_>>()).collect::<Vec<_>>(),
                }))
            }
        }
        "process_list" => {
            let conn = require_conn(state, conn_id)?;
            let list = conn.process_list().await.map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(list).unwrap_or(json!([])))
        }
        "kill_query" => {
            if state.settings.read_only {
                return Err("MCP server is in read-only mode".into());
            }
            let conn = require_conn(state, conn_id)?;
            let pid = args.get("process_id").and_then(|v| v.as_u64()).ok_or("missing process_id")?;
            conn.kill_query(pid).await.map_err(|e| e.to_string())?;
            Ok(json!({ "ok": true }))
        }
        "server_info" => {
            let conn = require_conn(state, conn_id)?;
            let info = conn.server_info().await.map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(info).unwrap_or(json!({})))
        }
        "table_sizes" => {
            let conn = require_conn(state, conn_id)?;
            let db = args.get("database").and_then(|v| v.as_str()).ok_or("missing database")?;
            let objects = conn.list_objects(db).await.map_err(|e| e.to_string())?;
            let sizes: Vec<Value> = objects
                .into_iter()
                .filter(|o| o.kind == ObjectKind::Table)
                .map(|o| {
                    json!({
                        "table": o.name,
                        "rows": o.rows_estimate,
                        "data_length": o.data_length,
                    })
                })
                .collect();
            Ok(json!({ "tables": sizes }))
        }
        "get_routine_definition" => {
            let conn = require_conn(state, conn_id)?;
            let db = args.get("database").and_then(|v| v.as_str()).ok_or("missing database")?;
            let name = args.get("name").and_then(|v| v.as_str()).ok_or("missing name")?;
            let sql = format!("SHOW CREATE PROCEDURE `{db}`.`{name}`");
            let result = conn
                .execute(&sql, QueryOptions::default(), CancelToken::new())
                .await;
            match result {
                Ok(r) => Ok(json!({
                    "definition": r.rows.first().and_then(|row| row.get(2)).map(|v| v.to_string())
                })),
                Err(_) => {
                    let sql = format!("SHOW CREATE FUNCTION `{db}`.`{name}`");
                    let r = conn
                        .execute(&sql, QueryOptions::default(), CancelToken::new())
                        .await
                        .map_err(|e| e.to_string())?;
                    Ok(json!({
                        "definition": r.rows.first().and_then(|row| row.get(2)).map(|v| v.to_string())
                    }))
                }
            }
        }
        "export_results" => Err("export_results: pass sql + path via future UI bridge".into()),
        other => Err(format!("unknown tool: {other}")),
    }
}

fn require_conn(
    state: &McpState,
    id: Option<Uuid>,
) -> Result<Arc<dyn Connection>, String> {
    let id = id.ok_or("missing connection_id")?;
    state
        .connections
        .read()
        .get(&id)
        .cloned()
        .ok_or_else(|| format!("connection not found: {id}"))
}