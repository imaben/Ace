use ace_driver::meta::{ColumnMeta, ForeignKeyMeta, IndexMeta, TableMeta, TriggerMeta};

#[derive(Debug, Clone, Default)]
pub struct StructureViewState {
    pub meta: Option<TableMeta>,
    pub selected_column: Option<usize>,
    pub draft_column: Option<ColumnMeta>,
    pub dirty: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct IndexesViewState {
    pub indexes: Vec<IndexMeta>,
    pub selected: Option<usize>,
}

#[derive(Debug, Clone, Default)]
pub struct RelationsViewState {
    pub foreign_keys: Vec<ForeignKeyMeta>,
    pub selected: Option<usize>,
}

#[derive(Debug, Clone, Default)]
pub struct TriggersViewState {
    pub triggers: Vec<TriggerMeta>,
    pub selected: Option<usize>,
    pub draft_sql: String,
}

#[derive(Debug, Clone, Default)]
pub struct TableInfoViewState {
    pub engine: Option<String>,
    pub charset: Option<String>,
    pub collation: Option<String>,
    pub comment: Option<String>,
    pub auto_increment: Option<u64>,
    pub create_sql: Option<String>,
    pub row_count: Option<u64>,
    pub data_length: Option<u64>,
}