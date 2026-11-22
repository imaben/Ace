use crate::paths::{ensure_dirs, favorites_path};
use ace_core::{AceError, AceResult, FavoritesTree};

pub fn load_favorites() -> AceResult<FavoritesTree> {
    ensure_dirs().map_err(|e| AceError::Io(e.to_string()))?;
    let path = favorites_path();
    if !path.exists() {
        return Ok(FavoritesTree::new());
    }
    let raw = std::fs::read_to_string(&path).map_err(|e| AceError::Io(e.to_string()))?;
    serde_json::from_str(&raw).map_err(|e| AceError::Message(e.to_string()))
}

pub fn save_favorites(tree: &FavoritesTree) -> AceResult<()> {
    ensure_dirs().map_err(|e| AceError::Io(e.to_string()))?;
    let raw = serde_json::to_string_pretty(tree)?;
    std::fs::write(favorites_path(), raw).map_err(|e| AceError::Io(e.to_string()))
}