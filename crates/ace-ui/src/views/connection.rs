use ace_core::{ConnectionInfo, ConnectionMethod, EngineKind, FavoritesTree};

#[derive(Debug, Clone)]
pub struct ConnectionFormState {
    pub info: ConnectionInfo,
    pub show_password: bool,
    pub testing: bool,
    pub test_message: Option<String>,
    pub error: Option<String>,
}

impl Default for ConnectionFormState {
    fn default() -> Self {
        Self {
            info: ConnectionInfo::default(),
            show_password: false,
            testing: false,
            test_message: None,
            error: None,
        }
    }
}

impl ConnectionFormState {
    pub fn from_info(info: ConnectionInfo) -> Self {
        Self {
            info,
            ..Default::default()
        }
    }

    pub fn set_engine(&mut self, engine: EngineKind) {
        self.info.engine = engine;
        self.info.port = ConnectionInfo::default_port_for_engine(engine);
    }

    pub fn set_method(&mut self, method: ConnectionMethod) {
        self.info.method = method;
        if method == ConnectionMethod::SshTunnel && self.info.ssh.is_none() {
            self.info.ssh = Some(Default::default());
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.info.name.trim().is_empty() {
            return Err("Name is required".into());
        }
        match self.info.engine {
            EngineKind::Sqlite => {
                if self.info.database.as_deref().unwrap_or("").is_empty() {
                    return Err("SQLite path is required".into());
                }
            }
            _ => {
                if self.info.method != ConnectionMethod::Socket && self.info.host.trim().is_empty()
                {
                    return Err("Host is required".into());
                }
                if self.info.user.trim().is_empty() {
                    return Err("User is required".into());
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct FavoritesViewState {
    pub tree: FavoritesTree,
    pub filter: String,
    pub selected_id: Option<uuid::Uuid>,
    pub expanded: Vec<uuid::Uuid>,
}

impl FavoritesViewState {
    pub fn visible_connections(&self) -> Vec<&ace_core::ConnectionInfo> {
        if self.filter.trim().is_empty() {
            self.tree.all_connections()
        } else {
            self.tree.search(&self.filter)
        }
    }
}