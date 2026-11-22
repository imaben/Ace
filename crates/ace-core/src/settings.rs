use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub general: GeneralSettings,
    pub table: TableSettings,
    pub editor: EditorSettings,
    pub network: NetworkSettings,
    pub mcp: McpSettings,
    pub appearance: AppearanceSettings,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            general: GeneralSettings::default(),
            table: TableSettings::default(),
            editor: EditorSettings::default(),
            network: NetworkSettings::default(),
            mcp: McpSettings::default(),
            appearance: AppearanceSettings::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralSettings {
    pub language: String,
    pub check_for_updates: bool,
    pub confirm_delete: bool,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            language: "en".into(),
            check_for_updates: true,
            confirm_delete: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSettings {
    pub rows_per_page: usize,
    pub null_display: String,
    pub font_size: f32,
    pub row_height: f32,
    pub show_row_numbers: bool,
}

impl Default for TableSettings {
    fn default() -> Self {
        Self {
            rows_per_page: 1000,
            null_display: "NULL".into(),
            font_size: 13.0,
            row_height: 22.0,
            show_row_numbers: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorSettings {
    pub font_family: String,
    pub font_size: f32,
    pub tab_width: usize,
    pub soft_wrap: bool,
    pub auto_complete: bool,
    pub theme: String,
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            font_family: "Menlo".into(),
            font_size: 13.0,
            tab_width: 4,
            soft_wrap: false,
            auto_complete: true,
            theme: "Dark Ace".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSettings {
    pub connect_timeout_secs: u64,
    pub keep_alive_secs: u64,
    pub ssh_binary: String,
    pub query_timeout_secs: u64,
}

impl Default for NetworkSettings {
    fn default() -> Self {
        Self {
            connect_timeout_secs: 30,
            keep_alive_secs: 60,
            ssh_binary: "ssh".into(),
            query_timeout_secs: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpSettings {
    pub enabled: bool,
    pub bind_host: String,
    pub bind_port: u16,
    pub read_only: bool,
    pub auth_token: Option<String>,
}

impl Default for McpSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            bind_host: "127.0.0.1".into(),
            bind_port: 3847,
            read_only: true,
            auth_token: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThemeMode {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppearanceSettings {
    pub theme_mode: ThemeMode,
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            theme_mode: ThemeMode::System,
        }
    }
}

impl AppSettings {
    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }

    pub fn from_toml(s: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(s)
    }
}