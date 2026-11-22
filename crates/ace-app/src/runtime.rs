use ace_core::{parse_connection_string, AceError, McpSettings};
use ace_driver::query::{CancelToken, QueryOptions};
use ace_driver::DriverRegistry;
use ace_io::export::{export_query_result, ExportFormat, ExportOptions};
use ace_mcp::{McpServer, McpState};
use std::path::Path;
use std::sync::Arc;

pub fn registry() -> DriverRegistry {
    let mut reg = DriverRegistry::new();
    reg.register(ace_mysql::driver());
    reg.register(ace_postgres::driver());
    reg.register(ace_sqlite::driver());
    reg
}

pub async fn open_url(url: &str) -> Result<Arc<dyn ace_driver::Connection>, AceError> {
    let info = parse_connection_string(url).map_err(AceError::msg)?;
    let mut info = info;
    if info.aws.enabled {
        let token = ace_net::resolve_aws_token(&info.aws, &info.host, info.port, &info.user).await?;
        info.password = Some(token);
    }
    if info.method == ace_core::ConnectionMethod::SshTunnel {
        if let Some(ssh) = info.ssh.clone() {
            let tunnel = ace_net::SshTunnel::open(&ssh, &info.host, info.port, "ssh").await?;
            info.host = "127.0.0.1".into();
            info.port = tunnel.local_port;
            // Leak tunnel for process lifetime of this connection (CLI sessions).
            std::mem::forget(tunnel);
        }
    }
    registry().open(&info).await
}

pub async fn run_query(url: &str, sql: &str) -> anyhow::Result<()> {
    let conn = open_url(url).await?;
    let result = conn
        .execute(sql, QueryOptions::default(), CancelToken::new())
        .await?;
    println!(
        "{}",
        serde_json::json!({
            "columns": result.columns.iter().map(|c| &c.name).collect::<Vec<_>>(),
            "rows": result.rows.iter().map(|r| {
                r.iter().map(|v| v.to_string()).collect::<Vec<_>>()
            }).collect::<Vec<_>>(),
            "affected_rows": result.affected_rows,
            "duration_ms": result.duration_ms,
        })
    );
    Ok(())
}

pub async fn run_export(url: &str, sql: &str, output: &str, format: &str) -> anyhow::Result<()> {
    let conn = open_url(url).await?;
    let result = conn
        .execute(sql, QueryOptions::default(), CancelToken::new())
        .await?;
    let format = match format.to_lowercase().as_str() {
        "sql" => ExportFormat::Sql,
        "json" => ExportFormat::Json,
        "xml" => ExportFormat::Xml,
        "html" => ExportFormat::Html,
        "dot" => ExportFormat::Dot,
        _ => ExportFormat::Csv,
    };
    let n = export_query_result(
        Path::new(output),
        &result,
        None,
        &ExportOptions {
            format,
            ..Default::default()
        },
    )?;
    println!("exported {n} rows to {output}");
    Ok(())
}

pub async fn run_mcp(host: &str, port: u16, read_only: bool) -> anyhow::Result<()> {
    let settings = McpSettings {
        enabled: true,
        bind_host: host.to_string(),
        bind_port: port,
        read_only,
        auth_token: None,
    };
    let state = Arc::new(McpState::new(settings));
    let handle = McpServer::start(state).await?;
    println!("MCP server on http://{host}:{port}/mcp (Ctrl+C to stop)");
    tokio::signal::ctrl_c().await?;
    handle.stop();
    Ok(())
}