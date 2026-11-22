//! Sequel Ace / Sequel Pro Favorites.plist import & export.
//!
//! Wire format keys mirror `SPConstants.m` (`SPFavorite*Key`).

use ace_core::{
    AceError, AceResult, AwsAuthOptions, ConnectionInfo, ConnectionMethod, EngineKind,
    FavoriteNode, FavoritesTree, SslOptions, SshOptions, TimeZoneMode, VaultOptions,
};
use plist::{Dictionary, Value};
use std::path::Path;
use uuid::Uuid;

const ROOT_KEY: &str = "SPConnectionFavorites";
const FAVORITES_ROOT: &str = "Favorites Root";
const CHILDREN: &str = "Children";
const GROUP_NAME: &str = "Name";
const IS_EXPANDED: &str = "IsExpanded";

pub fn export_favorites_plist(tree: &FavoritesTree, path: &Path) -> AceResult<()> {
    let children: Vec<Value> = tree.roots.iter().filter_map(node_to_plist).collect();
    let mut favorites_root = Dictionary::new();
    favorites_root.insert(CHILDREN.to_string(), Value::Array(children));
    favorites_root.insert(GROUP_NAME.to_string(), Value::String("Favorites Root".into()));
    favorites_root.insert(IS_EXPANDED.to_string(), Value::Boolean(true));

    let mut outer = Dictionary::new();
    outer.insert(FAVORITES_ROOT.to_string(), Value::Dictionary(favorites_root));

    let mut doc = Dictionary::new();
    doc.insert(ROOT_KEY.to_string(), Value::Dictionary(outer));

    plist::to_file_xml(path, &Value::Dictionary(doc)).map_err(|e| AceError::Io(e.to_string()))?;
    Ok(())
}

pub fn import_favorites_plist(path: &Path) -> AceResult<FavoritesTree> {
    let value = plist::from_file(path).map_err(|e| AceError::Io(e.to_string()))?;
    let Value::Dictionary(root) = value else {
        return Err(AceError::Message("Invalid favorites plist root".into()));
    };

    let favorites = root
        .get(ROOT_KEY)
        .or_else(|| root.get(FAVORITES_ROOT))
        .cloned();

    let children = match favorites {
        Some(Value::Dictionary(d)) => {
            let inner = d
                .get(FAVORITES_ROOT)
                .cloned()
                .unwrap_or(Value::Dictionary(d.clone()));
            match inner {
                Value::Dictionary(fr) => fr
                    .get(CHILDREN)
                    .and_then(|c| match c {
                        Value::Array(a) => Some(a.clone()),
                        _ => None,
                    })
                    .unwrap_or_default(),
                Value::Array(a) => a,
                _ => Vec::new(),
            }
        }
        Some(Value::Array(a)) => a,
        _ => {
            // Flat export without wrapper
            if let Some(Value::Array(a)) = root.get(CHILDREN) {
                a.clone()
            } else {
                return Err(AceError::Message(
                    "Could not find SPConnectionFavorites / Favorites Root".into(),
                ));
            }
        }
    };

    let mut tree = FavoritesTree::new();
    for child in children {
        if let Some(node) = plist_to_node(&child) {
            tree.add_root(node);
        }
    }
    Ok(tree)
}

fn node_to_plist(node: &FavoriteNode) -> Option<Value> {
    match node {
        FavoriteNode::Group {
            name,
            children,
            expanded,
            ..
        } => {
            let mut d = Dictionary::new();
            d.insert(GROUP_NAME.to_string(), Value::String(name.clone()));
            d.insert(IS_EXPANDED.to_string(), Value::Boolean(*expanded));
            let kids: Vec<Value> = children.iter().filter_map(node_to_plist).collect();
            d.insert(CHILDREN.to_string(), Value::Array(kids));
            Some(Value::Dictionary(d))
        }
        FavoriteNode::Connection { info } => Some(Value::Dictionary(info_to_dict(info))),
    }
}

fn info_to_dict(info: &ConnectionInfo) -> Dictionary {
    let mut d = Dictionary::new();
    d.insert("name".into(), Value::String(info.name.clone()));
    d.insert(
        "type".into(),
        Value::Integer(method_to_int(info.method).into()),
    );
    d.insert("host".into(), Value::String(info.host.clone()));
    d.insert("user".into(), Value::String(info.user.clone()));
    d.insert(
        "port".into(),
        Value::String(info.port.to_string()),
    );
    d.insert(
        "database".into(),
        Value::String(info.database.clone().unwrap_or_default()),
    );
    d.insert(
        "colorIndex".into(),
        Value::Integer((info.color_index as i64).into()),
    );
    d.insert(
        "socket".into(),
        Value::String(info.socket_path.clone().unwrap_or_default()),
    );
    d.insert(
        "useSSL".into(),
        Value::Integer(i64::from(info.ssl.enabled).into()),
    );
    d.insert(
        "requestServerPublicKey".into(),
        Value::Integer(i64::from(info.ssl.request_rsa_key).into()),
    );
    d.insert(
        "allowDataLocalInfile".into(),
        Value::Integer(i64::from(info.allow_data_local_infile).into()),
    );
    d.insert(
        "enableClearTextPlugin".into(),
        Value::Integer(i64::from(info.enable_cleartext_plugin).into()),
    );
    d.insert(
        "useCompression".into(),
        Value::Integer(i64::from(info.use_compression).into()),
    );

    let (tz_mode, tz_id) = match &info.timezone {
        TimeZoneMode::Server => (0, String::new()),
        TimeZoneMode::System => (1, String::new()),
        TimeZoneMode::Fixed(id) => (2, id.clone()),
    };
    d.insert("timeZoneMode".into(), Value::Integer(tz_mode.into()));
    d.insert("timeZone".into(), Value::String(tz_id));

    if let Some(path) = &info.ssl.client_key_path {
        d.insert("sslKeyFileLocationEnabled".into(), Value::Integer(1.into()));
        d.insert("sslKeyFileLocation".into(), Value::String(path.clone()));
    }
    if let Some(path) = &info.ssl.client_cert_path {
        d.insert(
            "sslCertificateFileLocationEnabled".into(),
            Value::Integer(1.into()),
        );
        d.insert(
            "sslCertificateFileLocation".into(),
            Value::String(path.clone()),
        );
    }
    if let Some(path) = &info.ssl.ca_cert_path {
        d.insert(
            "sslCACertFileLocationEnabled".into(),
            Value::Integer(1.into()),
        );
        d.insert("sslCACertFileLocation".into(), Value::String(path.clone()));
    }

    if let Some(ssh) = &info.ssh {
        d.insert("sshHost".into(), Value::String(ssh.host.clone()));
        d.insert("sshUser".into(), Value::String(ssh.user.clone()));
        d.insert("sshPort".into(), Value::String(ssh.port.to_string()));
        if let Some(key) = &ssh.private_key_path {
            d.insert("sshKeyLocationEnabled".into(), Value::Integer(1.into()));
            d.insert("sshKeyLocation".into(), Value::String(key.clone()));
        }
        if let Some(rs) = &ssh.remote_socket_path {
            d.insert("sshRemoteSocketPath".into(), Value::String(rs.clone()));
        }
    }

    if info.aws.enabled || info.method == ConnectionMethod::AwsIam {
        d.insert("useAWSIAMAuth".into(), Value::Integer(1.into()));
        d.insert(
            "awsProfile".into(),
            Value::String(info.aws.profile.clone().unwrap_or_else(|| "default".into())),
        );
        d.insert(
            "awsRegion".into(),
            Value::String(info.aws.region.clone().unwrap_or_default()),
        );
    }

    if info.vault.enabled || info.method == ConnectionMethod::Vault {
        d.insert(
            "vaultHost".into(),
            Value::String(info.vault.host.clone().unwrap_or_default()),
        );
        d.insert(
            "vaultPort".into(),
            Value::String(info.vault.port.to_string()),
        );
        d.insert(
            "vaultOIDCMount".into(),
            Value::String(
                info.vault
                    .oidc_mount
                    .clone()
                    .unwrap_or_else(|| "oidc".into()),
            ),
        );
        if let Some(path) = info.vault.credentials_path_resolved() {
            d.insert("vaultCredentialsPath".into(), Value::String(path));
        }
    }

    // Stable-ish integer id for SA wire format
    d.insert(
        "id".into(),
        Value::Integer((info.id.as_u128() as i64).wrapping_abs().into()),
    );
    d
}

fn plist_to_node(value: &Value) -> Option<FavoriteNode> {
    let Value::Dictionary(d) = value else {
        return None;
    };
    if d.contains_key(CHILDREN) {
        let name = string_val(d, GROUP_NAME).unwrap_or_else(|| "Group".into());
        let expanded = bool_val(d, IS_EXPANDED).unwrap_or(true);
        let kids = match d.get(CHILDREN) {
            Some(Value::Array(a)) => a.iter().filter_map(plist_to_node).collect(),
            _ => Vec::new(),
        };
        return Some(FavoriteNode::Group {
            id: Uuid::new_v4(),
            name,
            children: kids,
            expanded,
        });
    }
    Some(FavoriteNode::Connection {
        info: dict_to_info(d),
    })
}

fn dict_to_info(d: &Dictionary) -> ConnectionInfo {
    let mut info = ConnectionInfo::default();
    info.id = Uuid::new_v4();
    info.engine = EngineKind::Mysql;
    info.name = string_val(d, "name").unwrap_or_default();
    info.method = int_val(d, "type")
        .map(int_to_method)
        .unwrap_or(ConnectionMethod::Tcp);
    info.host = string_val(d, "host").unwrap_or_default();
    info.user = string_val(d, "user").unwrap_or_default();
    info.port = string_val(d, "port")
        .and_then(|p| p.parse().ok())
        .or_else(|| int_val(d, "port").map(|n| n as u16))
        .unwrap_or(3306);
    let db = string_val(d, "database").unwrap_or_default();
    info.database = if db.is_empty() { None } else { Some(db) };
    info.color_index = int_val(d, "colorIndex").unwrap_or(-1) as i32;
    let sock = string_val(d, "socket").unwrap_or_default();
    info.socket_path = if sock.is_empty() { None } else { Some(sock) };

    info.ssl = SslOptions {
        enabled: int_val(d, "useSSL").unwrap_or(0) != 0,
        request_rsa_key: int_val(d, "requestServerPublicKey").unwrap_or(0) != 0,
        client_key_path: string_val(d, "sslKeyFileLocation").filter(|s| !s.is_empty()),
        client_cert_path: string_val(d, "sslCertificateFileLocation").filter(|s| !s.is_empty()),
        ca_cert_path: string_val(d, "sslCACertFileLocation").filter(|s| !s.is_empty()),
        verify_server_cert: false,
    };
    info.allow_data_local_infile = int_val(d, "allowDataLocalInfile").unwrap_or(0) != 0;
    info.enable_cleartext_plugin = int_val(d, "enableClearTextPlugin").unwrap_or(0) != 0;
    info.use_compression = int_val(d, "useCompression").unwrap_or(1) != 0;

    info.timezone = match int_val(d, "timeZoneMode").unwrap_or(0) {
        1 => TimeZoneMode::System,
        2 => TimeZoneMode::Fixed(string_val(d, "timeZone").unwrap_or_default()),
        _ => TimeZoneMode::Server,
    };

    let ssh_host = string_val(d, "sshHost").unwrap_or_default();
    if !ssh_host.is_empty() || info.method == ConnectionMethod::SshTunnel {
        info.ssh = Some(SshOptions {
            host: ssh_host,
            port: string_val(d, "sshPort")
                .and_then(|p| p.parse().ok())
                .unwrap_or(22),
            user: string_val(d, "sshUser").unwrap_or_default(),
            password: None,
            private_key_path: string_val(d, "sshKeyLocation").filter(|s| !s.is_empty()),
            passphrase: None,
            use_ssh_config: true,
            remote_socket_path: string_val(d, "sshRemoteSocketPath").filter(|s| !s.is_empty()),
        });
    }

    if int_val(d, "useAWSIAMAuth").unwrap_or(0) != 0 || info.method == ConnectionMethod::AwsIam {
        info.method = ConnectionMethod::AwsIam;
        info.aws = AwsAuthOptions {
            enabled: true,
            region: string_val(d, "awsRegion").filter(|s| !s.is_empty()),
            profile: Some(
                string_val(d, "awsProfile").unwrap_or_else(|| "default".into()),
            ),
            use_sso: false,
        };
    }

    let vault_host = string_val(d, "vaultHost").unwrap_or_default();
    if !vault_host.is_empty() || info.method == ConnectionMethod::Vault {
        info.method = ConnectionMethod::Vault;
        let creds = string_val(d, "vaultCredentialsPath");
        let (mount, role) = split_vault_path(creds.as_deref());
        info.vault = VaultOptions {
            enabled: true,
            host: if vault_host.is_empty() {
                None
            } else {
                Some(vault_host)
            },
            port: string_val(d, "vaultPort")
                .and_then(|p| p.parse().ok())
                .unwrap_or(443),
            oidc_mount: Some(
                string_val(d, "vaultOIDCMount").unwrap_or_else(|| "oidc".into()),
            ),
            mount_path: mount,
            role,
            credentials_path: creds,
        };
    }

    info
}

fn split_vault_path(path: Option<&str>) -> (Option<String>, Option<String>) {
    let Some(path) = path.filter(|p| !p.is_empty()) else {
        return (None, None);
    };
    // `<mount>/creds/<role>`
    if let Some((mount, rest)) = path.split_once("/creds/") {
        return (Some(mount.to_string()), Some(rest.to_string()));
    }
    (None, None)
}

fn method_to_int(m: ConnectionMethod) -> i64 {
    match m {
        ConnectionMethod::Tcp => 0,
        ConnectionMethod::Socket => 1,
        ConnectionMethod::SshTunnel => 2,
        ConnectionMethod::AwsIam => 3,
        ConnectionMethod::Vault => 4,
    }
}

fn int_to_method(v: i64) -> ConnectionMethod {
    match v {
        1 => ConnectionMethod::Socket,
        2 => ConnectionMethod::SshTunnel,
        3 => ConnectionMethod::AwsIam,
        4 => ConnectionMethod::Vault,
        _ => ConnectionMethod::Tcp,
    }
}

fn string_val(d: &Dictionary, key: &str) -> Option<String> {
    match d.get(key)? {
        Value::String(s) => Some(s.clone()),
        Value::Integer(i) => i.as_signed().map(|n| n.to_string()),
        _ => None,
    }
}

fn int_val(d: &Dictionary, key: &str) -> Option<i64> {
    match d.get(key)? {
        Value::Integer(i) => i.as_signed(),
        Value::String(s) => s.parse().ok(),
        Value::Boolean(b) => Some(i64::from(*b)),
        _ => None,
    }
}

fn bool_val(d: &Dictionary, key: &str) -> Option<bool> {
    match d.get(key)? {
        Value::Boolean(b) => Some(*b),
        Value::Integer(i) => i.as_signed().map(|n| n != 0),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[test]
    fn roundtrip_plist() {
        let mut tree = FavoritesTree::new();
        let mut info = ConnectionInfo::mysql_local();
        info.name = "Local".into();
        info.host = "127.0.0.1".into();
        info.color_index = 2;
        tree.add_root(FavoriteNode::connection(info));

        let path = temp_dir().join("ace-favorites-test.plist");
        export_favorites_plist(&tree, &path).unwrap();
        let imported = import_favorites_plist(&path).unwrap();
        assert_eq!(imported.all_connections().len(), 1);
        assert_eq!(imported.all_connections()[0].name, "Local");
        assert_eq!(imported.all_connections()[0].color_index, 2);
        let _ = std::fs::remove_file(path);
    }
}
