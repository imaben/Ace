use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct ConsoleEntry {
    pub at: DateTime<Utc>,
    pub sql: String,
    pub error: Option<String>,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Default)]
pub struct ConsoleViewState {
    pub entries: Vec<ConsoleEntry>,
    pub filter: String,
    pub show_errors_only: bool,
}

impl ConsoleViewState {
    pub fn push(&mut self, sql: impl Into<String>, error: Option<String>, duration_ms: Option<u64>) {
        self.entries.push(ConsoleEntry {
            at: Utc::now(),
            sql: sql.into(),
            error,
            duration_ms,
        });
        if self.entries.len() > 5000 {
            self.entries.drain(0..self.entries.len() - 4000);
        }
    }

    pub fn visible(&self) -> Vec<&ConsoleEntry> {
        self.entries
            .iter()
            .filter(|e| {
                if self.show_errors_only && e.error.is_none() {
                    return false;
                }
                if self.filter.is_empty() {
                    return true;
                }
                e.sql.to_lowercase().contains(&self.filter.to_lowercase())
            })
            .collect()
    }
}