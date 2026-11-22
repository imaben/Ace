//! Deprecated compatibility shim. Use `crate::gui::design` instead.
#![allow(dead_code, unused_imports)]

pub use super::design::{apply_theme as apply_dark_theme, CONN_TOOLBAR};
pub use super::design::components::{
    accent_button as connect_button, color_dot, form_row, toolbar_inactive,
};
pub use super::design::tokens::tokens;
