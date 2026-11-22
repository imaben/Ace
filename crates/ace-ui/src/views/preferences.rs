use ace_core::AppSettings;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PrefPane {
    #[default]
    General,
    Tables,
    Editor,
    Network,
    Notifications,
    Mcp,
}

impl PrefPane {
    pub fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Tables => "Tables",
            Self::Editor => "Editor",
            Self::Network => "Network",
            Self::Notifications => "Notifications",
            Self::Mcp => "MCP Server",
        }
    }

    pub fn all() -> &'static [PrefPane] {
        &[
            Self::General,
            Self::Tables,
            Self::Editor,
            Self::Network,
            Self::Notifications,
            Self::Mcp,
        ]
    }
}

#[derive(Debug, Clone)]
pub struct PreferencesState {
    pub settings: AppSettings,
    pub active_pane: PrefPane,
    pub dirty: bool,
}

impl Default for PreferencesState {
    fn default() -> Self {
        Self {
            settings: AppSettings::default(),
            active_pane: PrefPane::General,
            dirty: false,
        }
    }
}