use directories::{ProjectDirs, UserDirs};
use std::path::PathBuf;

pub fn project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("com", "Ace", "Ace")
}

pub fn data_dir() -> PathBuf {
    project_dirs()
        .map(|p| p.data_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from(".ace"))
}

pub fn config_dir() -> PathBuf {
    project_dirs()
        .map(|p| p.config_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from(".ace"))
}

pub fn ensure_dirs() -> std::io::Result<()> {
    std::fs::create_dir_all(data_dir())?;
    std::fs::create_dir_all(config_dir())?;
    Ok(())
}

pub fn settings_path() -> PathBuf {
    config_dir().join("settings.toml")
}

pub fn favorites_path() -> PathBuf {
    data_dir().join("favorites.json")
}

pub fn credentials_path() -> PathBuf {
    data_dir().join("credentials.json")
}

pub fn history_db_path() -> PathBuf {
    data_dir().join("history.sqlite3")
}

pub fn home_dir() -> Option<PathBuf> {
    UserDirs::new().map(|u| u.home_dir().to_path_buf())
}