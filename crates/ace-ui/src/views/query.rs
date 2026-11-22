use ace_driver::result::QueryResult;
use ace_sql::complete::CompletionItem;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct QueryTabState {
    pub id: Uuid,
    pub title: String,
    pub sql: String,
    pub cursor: usize,
    pub results: Vec<QueryResult>,
    pub active_result: usize,
    pub running: bool,
    pub error: Option<String>,
    pub duration_ms: Option<u64>,
    pub completions: Vec<CompletionItem>,
    pub editable_table: Option<String>,
}

impl Default for QueryTabState {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            title: "Query 1".into(),
            sql: String::new(),
            cursor: 0,
            results: Vec::new(),
            active_result: 0,
            running: false,
            error: None,
            duration_ms: None,
            completions: Vec::new(),
            editable_table: None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct QueryWorkspaceState {
    pub tabs: Vec<QueryTabState>,
    pub active_tab: usize,
}

impl QueryWorkspaceState {
    pub fn ensure_tab(&mut self) {
        if self.tabs.is_empty() {
            self.tabs.push(QueryTabState::default());
            self.active_tab = 0;
        }
    }

    pub fn active_mut(&mut self) -> &mut QueryTabState {
        self.ensure_tab();
        let idx = self.active_tab.min(self.tabs.len() - 1);
        &mut self.tabs[idx]
    }

    pub fn active(&self) -> Option<&QueryTabState> {
        self.tabs.get(self.active_tab)
    }

    pub fn new_tab(&mut self) {
        let n = self.tabs.len() + 1;
        self.tabs.push(QueryTabState {
            title: format!("Query {n}"),
            ..Default::default()
        });
        self.active_tab = self.tabs.len() - 1;
    }
}