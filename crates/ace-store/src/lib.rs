//! Local persistence for settings, favorites, passwords, and history.

pub mod favorites_plist;
pub mod favorites_store;
pub mod history;
pub mod keychain;
pub mod paths;
pub mod settings_store;

pub use favorites_plist::*;
pub use favorites_store::*;
pub use history::*;
pub use keychain::*;
pub use paths::*;
pub use settings_store::*;