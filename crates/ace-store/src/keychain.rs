//! Persistent local credential storage (owner-only 0600 file + optional keyring).

use crate::paths::{credentials_path, ensure_dirs};
use ace_core::{AceError, AceResult};
use std::collections::BTreeMap;
use std::path::Path;

fn load_map(path: &Path) -> BTreeMap<String, String> {
    if !path.exists() {
        return BTreeMap::new();
    }
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn write_map(path: &Path, map: &BTreeMap<String, String>) -> AceResult<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| AceError::Io(e.to_string()))?;
    }
    let raw = serde_json::to_string_pretty(map)?;
    std::fs::write(path, raw).map_err(|e| AceError::Io(e.to_string()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

pub fn save_password(account: &str, password: &str) -> AceResult<()> {
    ensure_dirs().map_err(|e| AceError::Io(e.to_string()))?;
    let path = credentials_path();
    let mut map = load_map(&path);
    map.insert(account.to_string(), password.to_string());
    write_map(&path, &map)
}

pub fn load_password(account: &str) -> AceResult<Option<String>> {
    let path = credentials_path();
    let map = load_map(&path);
    Ok(map.get(account).cloned())
}

pub fn delete_password(account: &str) -> AceResult<()> {
    let path = credentials_path();
    if !path.exists() {
        return Ok(());
    }
    let mut map = load_map(&path);
    if map.remove(account).is_some() {
        write_map(&path, &map)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_local_credentials() {
        let dir = std::env::temp_dir().join(format!("ace-cred-test-{}", uuid::Uuid::new_v4()));
        let path = dir.join("credentials.json");
        let mut map = BTreeMap::new();
        map.insert("fav-1".into(), "s3cr3t!".into());
        write_map(&path, &map).unwrap();
        let loaded = load_map(&path);
        assert_eq!(loaded.get("fav-1").map(String::as_str), Some("s3cr3t!"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
