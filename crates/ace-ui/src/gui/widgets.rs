//! Compatibility shim — prefer `crate::gui::design::components`.
#![allow(dead_code, unused_imports)]

pub use super::design::components::{grid_body as render_grid, truncate};

use super::design::components::empty_state;
use gpui_kit::AnyElement;

pub fn empty_grid() -> AnyElement {
    empty_state("No data")
}

pub fn render_grid_body(columns: &[String], rows: &[Vec<String>]) -> AnyElement {
    render_grid(columns, rows)
}
