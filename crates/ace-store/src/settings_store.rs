use crate::paths::{ensure_dirs, settings_path};
use ace_core::{AceError, AceResult, AppSettings};
use std::path::Path;

pub fn load_settings() -> AceResult<AppSettings> {
    ensure_dirs().map_err(|e| AceError::Io(e.to_string()))?;
    load_settings_from(&settings_path())
}

pub fn save_settings(settings: &AppSettings) -> AceResult<()> {
    ensure_dirs().map_err(|e| AceError::Io(e.to_string()))?;
    save_settings_to(&settings_path(), settings)
}

pub fn load_settings_from(path: &Path) -> AceResult<AppSettings> {
    if !path.exists() {
        return Ok(AppSettings::default());
    }
    let raw = std::fs::read_to_string(path).map_err(|e| AceError::Io(e.to_string()))?;
    AppSettings::from_toml(&raw).map_err(|e| AceError::Message(e.to_string()))
}

pub fn save_settings_to(path: &Path, settings: &AppSettings) -> AceResult<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| AceError::Io(e.to_string()))?;
        }
    }
    let raw = settings
        .to_toml()
        .map_err(|e| AceError::Message(e.to_string()))?;
    std::fs::write(path, raw).map_err(|e| AceError::Io(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ace_core::{AppSettings, ThemeMode};

    fn temp_settings_path(label: &str) -> std::path::PathBuf {
        std::env::temp_dir()
            .join(format!(
                "ace-settings-test-{label}-{}",
                uuid::Uuid::new_v4()
            ))
            .join("settings.toml")
    }

    #[test]
    fn load_missing_file_returns_defaults() {
        let path = temp_settings_path("missing");
        let loaded = load_settings_from(&path).unwrap();
        assert_eq!(loaded, AppSettings::default());
        assert!(!path.exists());
    }

    #[test]
    fn save_and_load_roundtrip_in_temp_dir() {
        let path = temp_settings_path("roundtrip");
        let mut settings = AppSettings::default();
        settings.appearance.theme_mode = ThemeMode::Light;
        settings.editor.tab_width = 2;
        settings.network.connect_timeout_secs = 15;
        save_settings_to(&path, &settings).unwrap();
        let loaded = load_settings_from(&path).unwrap();
        assert_eq!(loaded, settings);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}
