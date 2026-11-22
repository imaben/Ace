//! Thread-safe SQL Query Console log (Sequel Ace Console).

use chrono::{DateTime, Local};
use parking_lot::RwLock;
use std::sync::Arc;

const MAX_CONSOLE_ENTRIES: usize = 500;

#[derive(Debug, Clone)]
pub struct ConsoleEntry {
    pub id: u64,
    pub timestamp: DateTime<Local>,
    pub database: String,
    pub sql: String,
    pub duration_ms: u64,
    pub error: Option<String>,
}

#[derive(Clone, Default)]
pub struct ConsoleLog {
    inner: Arc<RwLock<ConsoleInner>>,
}

#[derive(Default)]
struct ConsoleInner {
    next_id: u64,
    entries: Vec<ConsoleEntry>,
}

impl ConsoleLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(
        &self,
        database: impl Into<String>,
        sql: impl Into<String>,
        duration_ms: u64,
        error: Option<String>,
    ) {
        let mut guard = self.inner.write();
        guard.next_id += 1;
        let id = guard.next_id;
        guard.entries.push(ConsoleEntry {
            id,
            timestamp: Local::now(),
            database: database.into(),
            sql: sql.into(),
            duration_ms,
            error,
        });
        if guard.entries.len() > MAX_CONSOLE_ENTRIES {
            let drain = guard.entries.len() - MAX_CONSOLE_ENTRIES;
            guard.entries.drain(0..drain);
        }
    }

    pub fn snapshot(&self) -> Vec<ConsoleEntry> {
        self.inner.read().entries.clone()
    }

    pub fn clear(&self) {
        self.inner.write().entries.clear();
    }

    pub fn len(&self) -> usize {
        self.inner.read().entries.len()
    }

    pub fn as_text(&self) -> String {
        self.inner
            .read()
            .entries
            .iter()
            .map(|e| {
                let ts = e.timestamp.format("%H:%M:%S");
                match &e.error {
                    Some(err) => format!("[{ts}] /* {} — ERROR: {err} */\n{};", e.database, e.sql),
                    None => format!(
                        "[{ts}] /* {} ({} ms) */\n{};",
                        e.database, e.duration_ms, e.sql
                    ),
                }
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}
