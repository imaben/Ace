//! Self-update helper (phase 8). Wired for GitHub Releases.

use ace_core::{AceError, AceResult};

const GITHUB_REPO: &str = "imaben/Ace";

pub async fn check_latest_version() -> AceResult<Option<String>> {
    let url = format!("https://api.github.com/repos/{GITHUB_REPO}/releases/latest");
    let client = reqwest::Client::builder()
        .user_agent("ace")
        .build()
        .map_err(|e| AceError::Io(e.to_string()))?;
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| AceError::Io(e.to_string()))?;
    if !resp.status().is_success() {
        return Ok(None);
    }
    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AceError::Io(e.to_string()))?;
    Ok(body
        .get("tag_name")
        .and_then(|v| v.as_str())
        .map(|s| s.trim_start_matches('v').to_string()))
}

pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}