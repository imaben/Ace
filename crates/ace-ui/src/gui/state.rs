//! Strongly-typed GUI state models for Ace (Sequel Ace multi-tab architecture).

use crate::gui::console::ConsoleLog;
use crate::gui::data_grid::AceGridDelegate;
use crate::gui::sql_complete::SchemaCatalog;
use crate::views::MainViewTab;
use ace_core::{ConnectionInfo, ConnectionMethod, EngineKind, TimeZoneMode};
use ace_driver::meta::{
    DatabaseObject, ProcessInfo, ServerInfo, TableMeta, TriggerMeta, UserPrivilege,
};
use ace_driver::result::QueryResult;
use ace_driver::Connection;
use ace_io::ExportFormat;
use ace_sql::FilterOp;
use gpui_kit::component::input::{EditorState, InputState, TextareaState};
use gpui_kit::component::select::{SearchableVec, SelectState};
use gpui_kit::component::table::TableState;
use gpui_kit::*;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

pub const DEFAULT_PAGE_SIZE: usize = 100;

/// Sequel Ace filter operators for the Content view filter bar.
pub const FILTER_OPS: &[(&str, FilterOp)] = &[
    ("=", FilterOp::Eq),
    ("!=", FilterOp::Neq),
    (">", FilterOp::Gt),
    (">=", FilterOp::Gte),
    ("<", FilterOp::Lt),
    ("<=", FilterOp::Lte),
    ("contains", FilterOp::Contains),
    ("begins with", FilterOp::BeginsWith),
    ("ends with", FilterOp::EndsWith),
    ("LIKE", FilterOp::Like),
    ("NOT LIKE", FilterOp::NotLike),
    ("IN", FilterOp::In),
    ("IS NULL", FilterOp::IsNull),
    ("IS NOT NULL", FilterOp::IsNotNull),
];

pub fn filter_op_by_label(label: &str) -> FilterOp {
    FILTER_OPS
        .iter()
        .find(|(l, _)| *l == label)
        .map(|(_, op)| *op)
        .unwrap_or(FilterOp::Eq)
}

/// Inputs for the Sequel Ace Connection screen.
pub struct ConnectionFormInputs {
    pub name: Entity<InputState>,
    pub host: Entity<InputState>,
    pub port: Entity<InputState>,
    pub user: Entity<InputState>,
    pub password: Entity<InputState>,
    pub database: Entity<InputState>,
    pub socket: Entity<InputState>,
    pub ssh_host: Entity<InputState>,
    pub ssh_user: Entity<InputState>,
    pub ssh_password: Entity<InputState>,
    pub ssh_port: Entity<InputState>,
    pub ssh_key: Entity<InputState>,
    pub ssh_remote_socket: Entity<InputState>,
    pub aws_profile: Entity<InputState>,
    pub aws_region: Entity<InputState>,
    pub vault_host: Entity<InputState>,
    pub vault_port: Entity<InputState>,
    pub vault_oidc: Entity<InputState>,
    pub vault_mount: Entity<InputState>,
    pub vault_role: Entity<InputState>,
    pub ssl_key: Entity<InputState>,
    pub ssl_cert: Entity<InputState>,
    pub ssl_ca: Entity<InputState>,
    pub timezone_fixed: Entity<InputState>,
    pub favorites_search: Entity<InputState>,
}

/// Per-tab connection form draft state (each window tab can independently be on
/// the Connection screen or connected to a live database Workspace).
pub struct ConnectionDraft {
    pub selected_favorite: Option<Uuid>,
    pub selected_group: Option<Uuid>,
    pub engine: EngineKind,
    pub method: ConnectionMethod,
    pub color_index: i32,
    pub timezone: TimeZoneMode,
    pub use_ssl: bool,
    pub verify_server_cert: bool,
    pub request_rsa_key: bool,
    pub allow_data_local_infile: bool,
    pub enable_cleartext_plugin: bool,
    pub use_compression: bool,
    pub form_error: Option<String>,
    pub test_status: Option<(bool, String)>,
    pub connecting: bool,
    pub testing: bool,
    pub inputs: ConnectionFormInputs,
    /// Password retained from favorite / keychain when not explicitly retyped.
    pub retained_password: Option<String>,
}

/// A live connected database workspace inside a tab.
pub struct WorkspaceSession {
    pub info: ConnectionInfo,
    pub conn: Arc<dyn Connection>,
    pub raw_conn: Arc<dyn Connection>,
    pub schema_catalog: SchemaCatalog,
    pub server_info: Option<ServerInfo>,
    pub console: ConsoleLog,
    pub console_open: bool,
    pub console_filter: Entity<InputState>,

    pub databases: Vec<String>,
    pub current_db: String,
    pub db_select: Entity<SelectState<SearchableVec<SharedString>>>,

    pub objects: Vec<DatabaseObject>,
    pub object_filter: Entity<InputState>,
    pub selected_table: Option<String>,
    pub main_tab: MainViewTab,

    // Structure view
    pub table_meta: Option<TableMeta>,
    pub triggers: Vec<TriggerMeta>,
    pub structure_table: Entity<TableState<AceGridDelegate>>,
    pub index_table: Entity<TableState<AceGridDelegate>>,
    pub relation_table: Entity<TableState<AceGridDelegate>>,
    pub trigger_table: Entity<TableState<AceGridDelegate>>,
    pub selected_structure_row: Option<usize>,
    pub selected_index_row: Option<usize>,
    pub selected_relation_row: Option<usize>,
    pub selected_trigger_row: Option<usize>,

    // Content view
    pub content_table: Entity<TableState<AceGridDelegate>>,
    pub content_columns: Vec<String>,
    pub content_raw: Option<QueryResult>,
    pub row_count: Option<u64>,
    pub page_offset: usize,
    pub page_size: usize,
    pub sort_column: Option<String>,
    pub sort_desc: bool,
    pub where_clause: Option<String>,
    pub filter_col_select: Entity<SelectState<SearchableVec<SharedString>>>,
    pub filter_op_select: Entity<SelectState<SearchableVec<SharedString>>>,
    pub content_filter_value: Entity<InputState>,
    pub selected_cell: Option<(usize, usize)>,
    pub cell_edit_input: Entity<InputState>,

    // Table info editable comment
    pub table_comment_input: Entity<InputState>,

    // Query view
    pub query_editor: Entity<EditorState>,
    pub query_results: Vec<QueryResult>,
    pub active_result_idx: usize,
    pub query_result_table: Entity<TableState<AceGridDelegate>>,
    pub query_message: String,
    pub query_error: Option<String>,
    pub query_drawer: QueryDrawerTab,
    pub query_drawer_open: bool,

    // General status & scoped loading tracker
    pub status: String,
    pub loading: LoadingTracker,
    pub table_load_seq: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadingScope {
    Objects,
    Structure,
    Content,
    Query,
    General,
}

#[derive(Debug, Default, Clone)]
pub struct LoadingTracker {
    objects: usize,
    structure: usize,
    content: usize,
    query: usize,
    general: usize,
    label: Option<String>,
}

impl LoadingTracker {
    pub fn begin(&mut self, scope: LoadingScope, label: impl Into<String>) {
        *self.counter_mut(scope) += 1;
        self.label = Some(label.into());
    }

    pub fn end(&mut self, scope: LoadingScope) {
        let c = self.counter_mut(scope);
        *c = c.saturating_sub(1);
        if !self.is_any() {
            self.label = None;
        }
    }

    fn counter_mut(&mut self, scope: LoadingScope) -> &mut usize {
        match scope {
            LoadingScope::Objects => &mut self.objects,
            LoadingScope::Structure => &mut self.structure,
            LoadingScope::Content => &mut self.content,
            LoadingScope::Query => &mut self.query,
            LoadingScope::General => &mut self.general,
        }
    }

    pub fn objects(&self) -> bool {
        self.objects > 0
    }

    pub fn structure(&self) -> bool {
        self.structure > 0
    }

    pub fn content(&self) -> bool {
        self.content > 0
    }

    pub fn query(&self) -> bool {
        self.query > 0
    }

    pub fn is_any(&self) -> bool {
        self.objects > 0
            || self.structure > 0
            || self.content > 0
            || self.query > 0
            || self.general > 0
    }

    pub fn active_label(&self) -> &str {
        if let Some(lbl) = self.label.as_deref() {
            return lbl;
        }
        if self.query() {
            "Running query…"
        } else if self.content() {
            "Loading rows…"
        } else if self.structure() {
            "Loading structure…"
        } else if self.objects() {
            "Loading objects…"
        } else {
            "Loading…"
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QueryDrawerTab {
    #[default]
    History,
    Favorites,
}

/// One window tab — either showing the Connection view or a connected Workspace.
pub struct WorkspaceTab {
    pub id: Uuid,
    pub draft: ConnectionDraft,
    pub session: Option<WorkspaceSession>,
}

impl WorkspaceTab {
    pub fn title(&self, cx: &App) -> String {
        if let Some(s) = &self.session {
            let name = if s.info.name.trim().is_empty() {
                s.info.display_host().to_string()
            } else {
                s.info.name.clone()
            };
            if s.current_db.is_empty() {
                name
            } else {
                format!("{name} — {}", s.current_db)
            }
        } else {
            let n = self.draft.inputs.name.read(cx).value().to_string();
            if n.trim().is_empty() {
                "New Connection".into()
            } else {
                n
            }
        }
    }

    pub fn color_index(&self) -> i32 {
        if let Some(s) = &self.session {
            s.info.color_index
        } else {
            self.draft.color_index
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerToolsTab {
    Processes,
    Variables,
    Users,
    ServerInfo,
}

#[derive(Debug, Clone)]
pub enum ConfirmAction {
    DeleteFavorite(Uuid),
    DropDatabase(String),
    DropTable(String),
    TruncateTable(String),
    DropColumn { table: String, column: String },
    DropIndex { table: String, index: String },
    DropForeignKey { table: String, constraint: String },
    DropTrigger { trigger: String },
    DeleteRow { table: String, row_ix: usize },
}

pub enum ModalState {
    Confirm {
        title: String,
        message: String,
        confirm_label: String,
        action: ConfirmAction,
    },
    FieldEditor {
        row: usize,
        col: usize,
        column_name: String,
        is_null: bool,
        textarea: Entity<TextareaState>,
    },
    InsertRow {
        table: String,
        columns: Vec<String>,
        inputs: Vec<Entity<InputState>>,
        null_flags: Vec<bool>,
    },
    AddDatabase {
        name: Entity<InputState>,
        charset: Entity<InputState>,
        collation: Entity<InputState>,
    },
    AddTable {
        name: Entity<InputState>,
        engine: Entity<InputState>,
        charset: Entity<InputState>,
    },
    RenameTable {
        old_name: String,
        new_name: Entity<InputState>,
    },
    EditColumn {
        table: String,
        editing_old_name: Option<String>,
        name: Entity<InputState>,
        data_type: Entity<InputState>,
        nullable: bool,
        auto_increment: bool,
        default_val: Entity<InputState>,
        comment: Entity<InputState>,
    },
    AddIndex {
        table: String,
        name: Entity<InputState>,
        columns: Entity<InputState>,
        unique: bool,
    },
    AddForeignKey {
        table: String,
        name: Entity<InputState>,
        column: Entity<InputState>,
        ref_table: Entity<InputState>,
        ref_column: Entity<InputState>,
        on_update: Entity<InputState>,
        on_delete: Entity<InputState>,
    },
    AddTrigger {
        table: String,
        name: Entity<InputState>,
        timing: String, // BEFORE / AFTER
        event: String,  // INSERT / UPDATE / DELETE
        body: Entity<TextareaState>,
    },
    SaveQueryFavorite {
        name: Entity<InputState>,
        sql: String,
    },
    Export {
        source_label: String,
        table_name: Option<String>,
        result: QueryResult,
        path: Entity<InputState>,
        format: ExportFormat,
        gzip: bool,
        status: Option<String>,
    },
    ImportSql {
        path: Entity<InputState>,
        running: bool,
        status: Option<String>,
    },
    FavoritesPlist {
        path: Entity<InputState>,
        status: Option<String>,
    },
    ServerTools {
        tab: ServerToolsTab,
        loading: bool,
        processes: Vec<ProcessInfo>,
        variables: Vec<(String, String)>,
        var_filter: Entity<InputState>,
        users: Vec<UserPrivilege>,
        proc_table: Entity<TableState<AceGridDelegate>>,
        var_table: Entity<TableState<AceGridDelegate>>,
        user_table: Entity<TableState<AceGridDelegate>>,
        selected_proc_row: Option<usize>,
        error: Option<String>,
    },
    NewGroup {
        name: Entity<InputState>,
    },
}

pub fn sorted_variables(map: HashMap<String, String>) -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = map.into_iter().collect();
    v.sort_by(|a, b| a.0.cmp(&b.0));
    v
}

#[cfg(test)]
mod tests {
    use super::{LoadingScope, LoadingTracker};

    #[test]
    fn loading_tracker_handles_overlapping_scopes() {
        let mut tracker = LoadingTracker::default();
        assert!(!tracker.is_any());

        tracker.begin(LoadingScope::Structure, "Loading `users`…");
        tracker.begin(LoadingScope::Content, "Loading `users`…");
        assert!(tracker.structure());
        assert!(tracker.content());
        assert!(tracker.is_any());
        assert_eq!(tracker.active_label(), "Loading `users`…");

        tracker.end(LoadingScope::Structure);
        assert!(!tracker.structure());
        assert!(tracker.content());
        assert!(tracker.is_any());

        tracker.end(LoadingScope::Content);
        assert!(!tracker.content());
        assert!(!tracker.is_any());

        // Extra end calls saturate cleanly at 0
        tracker.end(LoadingScope::Content);
        assert!(!tracker.is_any());
    }

    #[test]
    fn loading_tracker_reference_counts_same_scope() {
        let mut tracker = LoadingTracker::default();
        tracker.begin(LoadingScope::Objects, "Req 1");
        tracker.begin(LoadingScope::Objects, "Req 2");
        tracker.end(LoadingScope::Objects);
        assert!(tracker.objects());
        tracker.end(LoadingScope::Objects);
        assert!(!tracker.objects());
    }
}
