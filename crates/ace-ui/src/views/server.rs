use ace_driver::meta::{ProcessInfo, UserPrivilege};
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct ProcessListState {
    pub processes: Vec<ProcessInfo>,
    pub auto_refresh_secs: Option<u64>,
    pub filter: String,
    pub selected: Option<usize>,
}

#[derive(Debug, Clone, Default)]
pub struct ServerVariablesState {
    pub variables: HashMap<String, String>,
    pub filter: String,
}

impl ServerVariablesState {
    pub fn filtered(&self) -> Vec<(&String, &String)> {
        let mut items: Vec<_> = self
            .variables
            .iter()
            .filter(|(k, v)| {
                self.filter.is_empty()
                    || k.to_lowercase().contains(&self.filter.to_lowercase())
                    || v.to_lowercase().contains(&self.filter.to_lowercase())
            })
            .collect();
        items.sort_by(|a, b| a.0.cmp(b.0));
        items
    }
}

#[derive(Debug, Clone, Default)]
pub struct UserManagerState {
    pub users: Vec<UserPrivilege>,
    pub selected: Option<usize>,
}

#[derive(Debug, Clone, Default)]
pub struct NavigatorState {
    pub databases: Vec<String>,
    pub expanded_db: Option<String>,
    pub tables: Vec<String>,
    pub expanded_table: Option<String>,
    pub columns: Vec<String>,
    pub filter: String,
}