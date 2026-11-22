//! Ace design system — Sequel Ace look & feel.
//!
//! **Rule:** screens compose these primitives. Do not invent per-page colours,
//! paddings, or button styles. Change tokens/components here to restyle the app.

pub mod components;
pub mod tokens;

pub use components::*;
pub use tokens::{tokens, ColorTokens, LayoutTokens, SpaceTokens, Tokens};

use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::*;

pub fn apply_theme(cx: &mut App) {
    Theme::change(ThemeMode::Dark, None, cx);
}

/// Standard connection-window inactive toolbar items (Sequel Ace order).
pub const CONN_TOOLBAR: &[&str] = &[
    "Structure",
    "Content",
    "Relations",
    "Triggers",
    "Table Info",
    "Query",
    "History",
    "Users",
    "Console",
];
