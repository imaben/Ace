use ace_core::{AceError, AceResult, VaultOptions};
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct VaultCredentials {
    pub username: String,
    pub password: String,
    pub lease_duration_secs: u64,
}

#[derive(Debug, Deserialize)]
struct VaultResponse {
    data: VaultData,
    #[serde(default)]
    lease_duration: u64,
}

#[derive(Debug, Deserialize)]
struct VaultData {
    username: String,
    password: String,
}

fn vault_base_url(opts: &VaultOptions) -> AceResult<String> {
    let host = opts
        .host
        .as_deref()
        .map(str::trim)
        .filter(|h| !h.is_empty())
        .ok_or_else(|| AceError::Auth("Vault address missing".into()))?;
    if host.starts_with("http://") || host.starts_with("https://") {
        Ok(host.trim_end_matches('/').to_string())
    } else {
        Ok(format!("https://{}:{}", host.trim_end_matches('/'), opts.port))
    }
}

/// Fetch dynamic database credentials from HashiCorp Vault.
pub async fn fetch_vault_credentials(
    opts: &VaultOptions,
    token: &str,
) -> AceResult<VaultCredentials> {
    if !opts.enabled {
        return Err(AceError::Auth("Vault not enabled".into()));
    }
    let address = vault_base_url(opts)?;
    let path = opts
        .credentials_path_resolved()
        .ok_or_else(|| AceError::Auth("Vault credentials path missing".into()))?;

    let url = format!("{}/v1/{}", address, path.trim_matches('/'));

    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("X-Vault-Token", token)
        .send()
        .await
        .map_err(|e| AceError::Auth(format!("Vault request failed: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AceError::Auth(format!("Vault error {status}: {body}")));
    }

    let parsed: VaultResponse = resp
        .json()
        .await
        .map_err(|e| AceError::Auth(format!("Vault parse error: {e}")))?;

    Ok(VaultCredentials {
        username: parsed.data.username,
        password: parsed.data.password,
        lease_duration_secs: parsed.lease_duration,
    })
}

/// OIDC login helper — opens browser and waits for callback (stub for phase 6).
pub async fn vault_oidc_login(address: &str, role: &str) -> AceResult<String> {
    let _ = (address, role);
    Err(AceError::NotSupported(
        "Vault OIDC login: implement browser callback in phase 6 UI".into(),
    ))
}
