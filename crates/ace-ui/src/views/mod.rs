pub mod connection;
pub mod console;
pub mod content;
pub mod preferences;
pub mod query;
pub mod server;
pub mod structure;

pub use connection::*;
pub use console::*;
pub use content::*;
pub use preferences::*;
pub use query::*;
pub use server::*;
pub use structure::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MainViewTab {
    #[default]
    Structure,
    Content,
    Relations,
    Triggers,
    TableInfo,
    CustomQuery,
}

impl MainViewTab {
    pub fn label(self) -> &'static str {
        match self {
            Self::Structure => "Structure",
            Self::Content => "Content",
            Self::Relations => "Relations",
            Self::Triggers => "Triggers",
            Self::TableInfo => "Table Info",
            Self::CustomQuery => "Query",
        }
    }

    pub fn all() -> &'static [MainViewTab] {
        &[
            Self::Structure,
            Self::Content,
            Self::Relations,
            Self::Triggers,
            Self::TableInfo,
            Self::CustomQuery,
        ]
    }
}