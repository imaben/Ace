use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Engine family — used for driver selection and capability flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EngineKind {
    #[default]
    Mysql,
    MariaDb,
    Postgres,
    Sqlite,
}

impl EngineKind {
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Mysql => "MySQL",
            Self::MariaDb => "MariaDB",
            Self::Postgres => "PostgreSQL",
            Self::Sqlite => "SQLite",
        }
    }
}

/// Mirrors Sequel Ace `SAConnectionType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionMethod {
    #[default]
    Tcp,
    Socket,
    SshTunnel,
    AwsIam,
    Vault,
}

impl ConnectionMethod {
    pub fn label(self) -> &'static str {
        match self {
            Self::Tcp => "TCP/IP",
            Self::Socket => "Socket",
            // Short labels match Sequel Ace type tabs (avoid truncation).
            Self::SshTunnel => "SSH",
            Self::AwsIam => "AWS IAM",
            Self::Vault => "Vault",
        }
    }

    pub fn all() -> &'static [ConnectionMethod] {
        &[
            Self::Tcp,
            Self::Socket,
            Self::SshTunnel,
            Self::AwsIam,
            Self::Vault,
        ]
    }
}

/// Mirrors Sequel Ace `SAConnectionTimeZoneMode`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TimeZoneMode {
    #[default]
    Server,
    System,
    /// IANA / MySQL timezone identifier (e.g. `America/Los_Angeles`, `+00:00`).
    Fixed(String),
}

impl TimeZoneMode {
    pub fn label(&self) -> String {
        match self {
            Self::Server => "Use Server Time Zone".into(),
            Self::System => "Use System Time Zone".into(),
            Self::Fixed(id) if id.is_empty() => "Use Server Time Zone".into(),
            Self::Fixed(id) => id.clone(),
        }
    }

    pub fn mysql_set_value(&self) -> Option<String> {
        match self {
            Self::Server => None,
            Self::System => Some("SYSTEM".into()),
            Self::Fixed(id) if id.trim().is_empty() => None,
            Self::Fixed(id) => Some(id.clone()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SslOptions {
    pub enabled: bool,
    pub ca_cert_path: Option<String>,
    pub client_cert_path: Option<String>,
    pub client_key_path: Option<String>,
    pub verify_server_cert: bool,
    /// Request server RSA public key for caching_sha2_password over non-SSL.
    pub request_rsa_key: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SshOptions {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: Option<String>,
    pub private_key_path: Option<String>,
    pub passphrase: Option<String>,
    pub use_ssh_config: bool,
    /// MySQL socket path on the remote side of the tunnel (Sequel Ace).
    pub remote_socket_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AwsAuthOptions {
    pub enabled: bool,
    pub region: Option<String>,
    pub profile: Option<String>,
    pub use_sso: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VaultOptions {
    pub enabled: bool,
    pub host: Option<String>,
    pub port: u16,
    pub oidc_mount: Option<String>,
    /// Database secrets mount (e.g. `databases_credentials`).
    pub mount_path: Option<String>,
    pub role: Option<String>,
    /// Legacy combined path `<mount>/creds/<role>`.
    pub credentials_path: Option<String>,
}

impl Default for VaultOptions {
    fn default() -> Self {
        Self {
            enabled: false,
            host: None,
            port: 443,
            oidc_mount: Some("oidc".into()),
            mount_path: Some("databases_credentials".into()),
            role: None,
            credentials_path: None,
        }
    }
}

impl VaultOptions {
    pub fn credentials_path_resolved(&self) -> Option<String> {
        if let Some(p) = &self.credentials_path {
            if !p.trim().is_empty() {
                return Some(p.clone());
            }
        }
        match (&self.mount_path, &self.role) {
            (Some(m), Some(r)) if !m.is_empty() && !r.is_empty() => {
                Some(format!("{}/creds/{}", m.trim_matches('/'), r.trim_matches('/')))
            }
            _ => None,
        }
    }
}

/// Full connection parameters (password may be redacted when persisted).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub id: Uuid,
    pub name: String,
    pub engine: EngineKind,
    pub method: ConnectionMethod,
    pub host: String,
    pub port: u16,
    pub user: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    pub database: Option<String>,
    pub socket_path: Option<String>,
    pub ssl: SslOptions,
    pub ssh: Option<SshOptions>,
    pub aws: AwsAuthOptions,
    pub vault: VaultOptions,
    pub timezone: TimeZoneMode,
    /// Legacy CSS color string (optional display).
    pub color: Option<String>,
    /// Sequel Ace favorite colour index (0…N, -1 = none).
    pub color_index: i32,
    pub keychain_id: Option<String>,
    /// Mirrors Sequel Ace `useCompression` (default true in favorites decode).
    pub use_compression: bool,
    pub allow_data_local_infile: bool,
    pub enable_cleartext_plugin: bool,
}

impl Default for ConnectionInfo {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: String::new(),
            engine: EngineKind::Mysql,
            method: ConnectionMethod::Tcp,
            host: String::new(),
            port: 3306,
            user: String::new(),
            password: None,
            database: None,
            socket_path: None,
            ssl: SslOptions::default(),
            ssh: None,
            aws: AwsAuthOptions {
                enabled: false,
                region: None,
                profile: Some("default".into()),
                use_sso: false,
            },
            vault: VaultOptions::default(),
            timezone: TimeZoneMode::Server,
            color: None,
            color_index: -1,
            keychain_id: None,
            use_compression: true,
            allow_data_local_infile: false,
            enable_cleartext_plugin: false,
        }
    }
}

impl ConnectionInfo {
    pub fn mysql_local() -> Self {
        let mut c = Self::default();
        c.name = "localhost".into();
        c.host = "127.0.0.1".into();
        c.user = "root".into();
        c
    }

    pub fn default_port_for_engine(engine: EngineKind) -> u16 {
        match engine {
            EngineKind::Mysql | EngineKind::MariaDb => 3306,
            EngineKind::Postgres => 5432,
            EngineKind::Sqlite => 0,
        }
    }

    pub fn display_host(&self) -> &str {
        match self.method {
            ConnectionMethod::Socket => self.socket_path.as_deref().unwrap_or("(default socket)"),
            ConnectionMethod::Vault => self
                .vault
                .host
                .as_deref()
                .filter(|s| !s.is_empty())
                .unwrap_or(self.host.as_str()),
            _ => {
                if self.host.is_empty() {
                    "127.0.0.1"
                } else {
                    &self.host
                }
            }
        }
    }

    pub fn connection_string(&self) -> String {
        match self.engine {
            EngineKind::Sqlite => {
                format!("sqlite://{}", self.database.as_deref().unwrap_or(":memory:"))
            }
            _ => {
                let db = self.database.as_deref().unwrap_or("");
                format!(
                    "{}://{}@{}:{}/{}",
                    self.engine.display_name().to_lowercase(),
                    self.user,
                    self.display_host(),
                    self.port,
                    db
                )
            }
        }
    }

    pub fn without_secrets(&self) -> Self {
        let mut c = self.clone();
        c.password = None;
        if let Some(ssh) = c.ssh.as_mut() {
            ssh.password = None;
            ssh.passphrase = None;
        }
        c
    }

    /// Strip newlines and trim — mirrors Sequel Ace `stripInvalidCharacters`.
    pub fn sanitize_inputs(&mut self) {
        fn clean(s: &mut String) {
            *s = s
                .chars()
                .filter(|c| *c != '\n' && *c != '\r')
                .collect::<String>()
                .trim()
                .to_string();
        }
        clean(&mut self.name);
        clean(&mut self.host);
        clean(&mut self.user);
        if let Some(db) = self.database.as_mut() {
            clean(db);
        }
        if let Some(sock) = self.socket_path.as_mut() {
            clean(sock);
        }
    }

    /// Weak gate for enabling Connect (mirrors `canAttemptConnection`).
    pub fn can_attempt_connection(&self) -> bool {
        match self.method {
            ConnectionMethod::Socket => true,
            ConnectionMethod::SshTunnel => {
                let ssh_ok = self
                    .ssh
                    .as_ref()
                    .map(|s| !s.host.trim().is_empty())
                    .unwrap_or(false);
                let remote_sock = self
                    .ssh
                    .as_ref()
                    .and_then(|s| s.remote_socket_path.as_deref())
                    .map(|s| !s.trim().is_empty())
                    .unwrap_or(false);
                ssh_ok && (!self.host.trim().is_empty() || remote_sock)
            }
            ConnectionMethod::Vault => {
                !self.host.trim().is_empty()
                    && self
                        .vault
                        .host
                        .as_ref()
                        .map(|h| !h.trim().is_empty())
                        .unwrap_or(false)
                    && self.vault.credentials_path_resolved().is_some()
            }
            ConnectionMethod::Tcp | ConnectionMethod::AwsIam => !self.host.trim().is_empty(),
        }
    }
}

/// Sequel Ace default favorite colour palette (PreferenceDefaults.plist).
pub fn favorite_color_rgb(index: i32) -> Option<(u8, u8, u8)> {
    const COLORS: &[(f32, f32, f32)] = &[
        (0.8941, 0.4549, 0.4000), // coral
        (0.9294, 0.6824, 0.4196), // orange
        (0.8902, 0.8353, 0.4667), // yellow
        (0.6863, 0.8431, 0.4667), // green
        (0.4627, 0.7255, 0.9098), // blue
        (0.7922, 0.5961, 0.8784), // purple
        (0.7137, 0.7137, 0.7137), // gray
    ];
    if index < 0 {
        return None;
    }
    COLORS.get(index as usize).map(|(r, g, b)| {
        (
            (r * 255.0).round() as u8,
            (g * 255.0).round() as u8,
            (b * 255.0).round() as u8,
        )
    })
}

pub fn favorite_color_count() -> usize {
    7
}

/// Parse a simple `mysql://user:pass@host:port/db` style URL.
pub fn parse_connection_string(s: &str) -> Result<ConnectionInfo, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("empty connection string".into());
    }

    let (scheme, rest) = s.split_once("://").unwrap_or(("mysql", s));
    let engine = match scheme.to_lowercase().as_str() {
        "mysql" => EngineKind::Mysql,
        "mariadb" => EngineKind::MariaDb,
        "postgres" | "postgresql" => EngineKind::Postgres,
        "sqlite" => EngineKind::Sqlite,
        other => return Err(format!("unknown scheme: {other}")),
    };

    if engine == EngineKind::Sqlite {
        let mut info = ConnectionInfo {
            engine,
            name: "SQLite".into(),
            database: Some(rest.to_string()),
            host: String::new(),
            port: 0,
            user: String::new(),
            ..Default::default()
        };
        info.id = Uuid::new_v4();
        return Ok(info);
    }

    let (auth_host, database) = match rest.split_once('/') {
        Some((left, db)) => (left, Some(db.trim_start_matches('/').to_string())),
        None => (rest, None),
    };

    let (user, password, hostport) = if let Some((cred, hp)) = auth_host.split_once('@') {
        if let Some((u, p)) = cred.split_once(':') {
            (u.to_string(), Some(p.to_string()), hp)
        } else {
            (cred.to_string(), None, hp)
        }
    } else {
        ("root".into(), None, auth_host)
    };

    let (host, port) = if let Some((h, p)) = hostport.rsplit_once(':') {
        let port: u16 = p
            .parse()
            .unwrap_or(ConnectionInfo::default_port_for_engine(engine));
        (h.to_string(), port)
    } else {
        (
            hostport.to_string(),
            ConnectionInfo::default_port_for_engine(engine),
        )
    };

    Ok(ConnectionInfo {
        id: Uuid::new_v4(),
        name: format!("{user}@{host}"),
        engine,
        method: ConnectionMethod::Tcp,
        host,
        port,
        user,
        password,
        database,
        ..Default::default()
    })
}

/// Validate connection details before attempting connect (Sequel Ace style).
pub fn validate_connection(info: &ConnectionInfo) -> Result<(), String> {
    let mut info = info.clone();
    info.sanitize_inputs();

    if info.engine == EngineKind::Sqlite {
        match &info.database {
            Some(p) if !p.is_empty() => return Ok(()),
            _ => return Err("SQLite database file path is required.".into()),
        }
    }

    match info.method {
        ConnectionMethod::Tcp | ConnectionMethod::AwsIam => {
            if info.host.is_empty() {
                return Err("Please enter a host name.".into());
            }
        }
        ConnectionMethod::Socket => {}
        ConnectionMethod::SshTunnel => {
            let ssh = info
                .ssh
                .as_ref()
                .ok_or_else(|| "SSH details are required.".to_string())?;
            if ssh.host.trim().is_empty() {
                return Err("Please enter an SSH host name.".into());
            }
            let remote_sock = ssh
                .remote_socket_path
                .as_deref()
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false);
            if info.host.is_empty() && !remote_sock {
                return Err("Please enter a MySQL host name or remote socket.".into());
            }
            if let Some(key) = &ssh.private_key_path {
                let key = key.trim();
                if !key.is_empty() {
                    if key.ends_with(".pub") {
                        return Err("SSH key must be a private key, not a .pub file.".into());
                    }
                    let expanded = expand_tilde(key);
                    if !std::path::Path::new(&expanded).exists() {
                        return Err(format!("SSH key file not found: {expanded}"));
                    }
                }
            }
        }
        ConnectionMethod::Vault => {
            if info
                .vault
                .host
                .as_ref()
                .map(|h| h.trim().is_empty())
                .unwrap_or(true)
            {
                return Err("Please enter a Vault host.".into());
            }
            if info.vault.credentials_path_resolved().is_none() {
                return Err("Please enter Vault mount and role.".into());
            }
            if info.host.is_empty() {
                return Err("Please enter a MySQL host name.".into());
            }
        }
    }

    if info.ssl.enabled {
        for (label, path) in [
            ("SSL key", info.ssl.client_key_path.as_deref()),
            ("SSL certificate", info.ssl.client_cert_path.as_deref()),
            ("SSL CA certificate", info.ssl.ca_cert_path.as_deref()),
        ] {
            if let Some(p) = path {
                let p = p.trim();
                if !p.is_empty() {
                    let expanded = expand_tilde(p);
                    if !std::path::Path::new(&expanded).exists() {
                        return Err(format!("{label} file not found: {expanded}"));
                    }
                }
            }
        }
    }

    Ok(())
}

fn expand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return std::path::Path::new(&home).join(rest).display().to_string();
        }
    }
    path.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_mysql_url() {
        let c = parse_connection_string("mysql://alice:secret@db.example:3307/app").unwrap();
        assert_eq!(c.user, "alice");
        assert_eq!(c.password.as_deref(), Some("secret"));
        assert_eq!(c.host, "db.example");
        assert_eq!(c.port, 3307);
        assert_eq!(c.database.as_deref(), Some("app"));
    }

    #[test]
    fn parse_sqlite() {
        let c = parse_connection_string("sqlite:///tmp/test.db").unwrap();
        assert_eq!(c.engine, EngineKind::Sqlite);
        assert_eq!(c.database.as_deref(), Some("/tmp/test.db"));
    }

    #[test]
    fn validate_tcp_requires_host() {
        let mut c = ConnectionInfo::default();
        c.method = ConnectionMethod::Tcp;
        assert!(validate_connection(&c).is_err());
        c.host = "127.0.0.1".into();
        assert!(validate_connection(&c).is_ok());
    }

    #[test]
    fn favorite_colors() {
        assert!(favorite_color_rgb(-1).is_none());
        assert_eq!(favorite_color_rgb(0), Some((228, 116, 102)));
        assert_eq!(favorite_color_count(), 7);
    }
}
