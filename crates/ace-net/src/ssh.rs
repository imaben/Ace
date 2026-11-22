use ace_core::{AceError, AceResult, SshOptions};
use std::net::SocketAddr;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tracing::{info, warn};

/// SSH local port-forward tunnel using the system `ssh` binary (compatible mode).
/// Pure-Rust `russh` backend can be swapped in later without changing callers.
pub struct SshTunnel {
    child: Child,
    pub local_port: u16,
    pub remote_host: String,
    pub remote_port: u16,
}

impl SshTunnel {
    pub async fn open(
        ssh: &SshOptions,
        remote_host: &str,
        remote_port: u16,
        ssh_binary: &str,
    ) -> AceResult<Self> {
        let local_port = find_free_port().await?;
        let target = format!("{local_port}:{remote_host}:{remote_port}");
        let mut cmd = Command::new(ssh_binary);
        cmd.arg("-N")
            .arg("-L")
            .arg(&target)
            .arg("-p")
            .arg(ssh.port.to_string())
            .arg("-o")
            .arg("ExitOnForwardFailure=yes")
            .arg("-o")
            .arg("StrictHostKeyChecking=accept-new")
            .arg(format!("{}@{}", ssh.user, ssh.host))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        if let Some(key) = &ssh.private_key_path {
            cmd.arg("-i").arg(key);
        }
        if ssh.password.is_some() {
            warn!("password-based SSH via system ssh requires sshpass / askpass; prefer key auth");
        }

        info!(
            "opening SSH tunnel {}@{}:{} -> localhost:{} -> {}:{}",
            ssh.user, ssh.host, ssh.port, local_port, remote_host, remote_port
        );

        let mut child = cmd
            .spawn()
            .map_err(|e| AceError::Connection(format!("failed to spawn ssh: {e}")))?;

        // Brief wait for forward to come up
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
        if let Ok(Some(status)) = child.try_wait() {
            let mut err = String::new();
            if let Some(stderr) = child.stderr.take() {
                let mut reader = BufReader::new(stderr);
                let _ = reader.read_line(&mut err).await;
            }
            return Err(AceError::Connection(format!(
                "ssh exited early ({status}): {err}"
            )));
        }

        Ok(Self {
            child,
            local_port,
            remote_host: remote_host.to_string(),
            remote_port,
        })
    }

    pub fn local_addr(&self) -> String {
        format!("127.0.0.1:{}", self.local_port)
    }

    pub async fn close(mut self) -> AceResult<()> {
        let _ = self.child.kill().await;
        Ok(())
    }
}

async fn find_free_port() -> AceResult<u16> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| AceError::Io(e.to_string()))?;
    let addr: SocketAddr = listener
        .local_addr()
        .map_err(|e| AceError::Io(e.to_string()))?;
    Ok(addr.port())
}

/// Parse a minimal subset of OpenSSH config Host blocks.
pub fn parse_ssh_config_host(content: &str, host_alias: &str) -> Option<SshOptions> {
    let mut current: Option<String> = None;
    let mut opts = SshOptions {
        host: host_alias.to_string(),
        port: 22,
        user: whoami_user(),
        ..Default::default()
    };
    let mut matched = false;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let key = parts.next()?.to_lowercase();
        let value = parts.collect::<Vec<_>>().join(" ");
        if key == "host" {
            current = Some(value.clone());
            matched = value.split_whitespace().any(|p| p == host_alias || p == "*");
            continue;
        }
        if !matched {
            continue;
        }
        match key.as_str() {
            "hostname" => opts.host = value,
            "user" => opts.user = value,
            "port" => opts.port = value.parse().unwrap_or(22),
            "identityfile" => {
                opts.private_key_path = Some(shellexpand_tilde(&value));
            }
            _ => {}
        }
    }
    current?;
    Some(opts)
}

fn shellexpand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = directories::UserDirs::new().map(|u| u.home_dir().to_path_buf()) {
            return home.join(rest).display().to_string();
        }
    }
    path.to_string()
}

fn whoami_user() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "user".into())
}