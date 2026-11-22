//! GPUI Kit desktop shell — faithful minimalist Sequel Ace layout & workflows.

pub mod console;
pub mod connection_view;
pub mod data_grid;
pub mod design;
pub mod modals;
pub mod root;
pub mod runtime;
mod sa_theme;
pub mod sql_complete;
pub mod sql_highlight;
pub mod state;
pub mod theme;
mod widgets;
pub mod workspace_view;

pub use root::run_gui;
