use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerInfo {
    pub version: String,
    pub version_comment: String,
    pub host: String,
    pub port: u16,
    pub current_user: String,
    pub current_database: Option<String>,
    pub charset: String,
    pub engine_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectKind {
    Table,
    View,
    Procedure,
    Function,
    Trigger,
    Event,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatabaseObject {
    pub name: String,
    pub kind: ObjectKind,
    pub engine: Option<String>,
    pub rows_estimate: Option<u64>,
    pub data_length: Option<u64>,
    pub comment: Option<String>,
    pub pinned: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColumnMeta {
    pub name: String,
    pub data_type: String,
    pub full_type: String,
    pub nullable: bool,
    pub primary_key: bool,
    pub auto_increment: bool,
    pub default: Option<String>,
    pub extra: Option<String>,
    pub comment: Option<String>,
    pub charset: Option<String>,
    pub collation: Option<String>,
    pub unsigned: bool,
    pub zerofill: bool,
    pub character_maximum_length: Option<u64>,
    pub numeric_precision: Option<u32>,
    pub numeric_scale: Option<u32>,
    pub ordinal_position: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexColumn {
    pub name: String,
    pub sub_part: Option<u32>,
    pub descending: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexMeta {
    pub name: String,
    pub columns: Vec<IndexColumn>,
    pub unique: bool,
    pub primary: bool,
    pub index_type: String,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForeignKeyMeta {
    pub name: String,
    pub columns: Vec<String>,
    pub ref_database: Option<String>,
    pub ref_table: String,
    pub ref_columns: Vec<String>,
    pub on_update: String,
    pub on_delete: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TriggerMeta {
    pub name: String,
    pub timing: String,
    pub event: String,
    pub statement: String,
    pub definer: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableMeta {
    pub name: String,
    pub database: String,
    pub engine: Option<String>,
    pub charset: Option<String>,
    pub collation: Option<String>,
    pub comment: Option<String>,
    pub auto_increment: Option<u64>,
    pub create_options: Option<String>,
    pub columns: Vec<ColumnMeta>,
    pub indexes: Vec<IndexMeta>,
    pub foreign_keys: Vec<ForeignKeyMeta>,
    pub create_sql: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub id: u64,
    pub user: String,
    pub host: String,
    pub database: Option<String>,
    pub command: String,
    pub time_secs: u64,
    pub state: Option<String>,
    pub info: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserPrivilege {
    pub user: String,
    pub host: String,
    pub global_privileges: Vec<String>,
    pub database_privileges: Vec<(String, Vec<String>)>,
    pub max_questions: Option<u32>,
    pub max_updates: Option<u32>,
    pub max_connections: Option<u32>,
    pub max_user_connections: Option<u32>,
}