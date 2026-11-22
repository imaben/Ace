use crate::paths::{ensure_dirs, settings_path};
use ace_core::{AceError, AceResult, AppSettings};

pub fn load_settings() -> AceResult<AppSettings> {
    ensure_dirs().map_err(|e| AceError::Io(e.to_string()))?;
    let path = settings_path();
    if !path.exists() {
        return Ok(AppSettings::default());
    }
    let raw = std::fs::read_to_string(&path).map_err(|e| AceError::Io(e.to_string()))?;
    AppSettings::from_toml(&raw).map_err(|e| AceError::Message(e.to_string()))
}

pub fn save_settings(settings: &AppSettings) -> AceResult<()> {
    ensure_dirs().map_err(|e| AceError::Io(e.to_string()))?;
    let raw = settings
        .to_toml()
        .map_err(|e| AceError::Message(e.to_string()))?;
    std::fs::write(settings_path(), raw).map_err(|e| AceError::Io(e.to_string()))
}