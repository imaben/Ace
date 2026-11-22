//! Local MCP-compatible HTTP server (JSON-RPC over HTTP).

mod server;
mod tools;

pub use server::{McpServer, McpServerHandle, McpState};
pub use tools::ToolName;