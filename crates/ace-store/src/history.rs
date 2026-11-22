use crate::paths::{ensure_dirs, history_db_path};
use ace_core::{AceError, AceResult};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub id: i64,
    pub session_id: Option<String>,
    pub sql: String,
    pub success: bool,
    pub duration_ms: u64,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct QueryFavorite {
    pub id: Uuid,
    pub name: String,
    pub sql: String,
    pub created_at: DateTime<Utc>,
}

pub struct HistoryStore {
    conn: Connection,
}

impl HistoryStore {
    pub fn open() -> AceResult<Self> {
        ensure_dirs().map_err(|e| AceError::Io(e.to_string()))?;
        let conn = Connection::open(history_db_path())
            .map_err(|e| AceError::Io(e.to_string()))?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS query_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT,
                sql TEXT NOT NULL,
                success INTEGER NOT NULL,
                duration_ms INTEGER NOT NULL,
                error TEXT,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS query_favorites (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                sql TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS pinned_tables (
                database_name TEXT NOT NULL,
                table_name TEXT NOT NULL,
                PRIMARY KEY (database_name, table_name)
            );
            CREATE INDEX IF NOT EXISTS idx_history_created ON query_history(created_at DESC);
            ",
        )
        .map_err(|e| AceError::Io(e.to_string()))?;
        Ok(Self { conn })
    }

    pub fn add_history(
        &self,
        session_id: Option<&str>,
        sql: &str,
        success: bool,
        duration_ms: u64,
        error: Option<&str>,
    ) -> AceResult<()> {
        self.conn
            .execute(
                "INSERT INTO query_history (session_id, sql, success, duration_ms, error, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    session_id,
                    sql,
                    success as i32,
                    duration_ms as i64,
                    error,
                    Utc::now().to_rfc3339()
                ],
            )
            .map_err(|e| AceError::Io(e.to_string()))?;
        Ok(())
    }

    pub fn recent(&self, limit: usize) -> AceResult<Vec<HistoryEntry>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, session_id, sql, success, duration_ms, error, created_at
                 FROM query_history ORDER BY id DESC LIMIT ?1",
            )
            .map_err(|e| AceError::Io(e.to_string()))?;
        let rows = stmt
            .query_map(params![limit as i64], |row| {
                let created: String = row.get(6)?;
                Ok(HistoryEntry {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    sql: row.get(2)?,
                    success: row.get::<_, i32>(3)? != 0,
                    duration_ms: row.get::<_, i64>(4)? as u64,
                    error: row.get(5)?,
                    created_at: DateTime::parse_from_rfc3339(&created)
                        .map(|d| d.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })
            .map_err(|e| AceError::Io(e.to_string()))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| AceError::Io(e.to_string()))?);
        }
        Ok(out)
    }

    pub fn delete_history(&self, id: i64) -> AceResult<()> {
        self.conn
            .execute("DELETE FROM query_history WHERE id = ?1", params![id])
            .map_err(|e| AceError::Io(e.to_string()))?;
        Ok(())
    }

    pub fn clear_history(&self) -> AceResult<()> {
        self.conn
            .execute("DELETE FROM query_history", [])
            .map_err(|e| AceError::Io(e.to_string()))?;
        Ok(())
    }

    pub fn add_favorite(&self, name: &str, sql: &str) -> AceResult<QueryFavorite> {
        let fav = QueryFavorite {
            id: Uuid::new_v4(),
            name: name.to_string(),
            sql: sql.to_string(),
            created_at: Utc::now(),
        };
        self.conn
            .execute(
                "INSERT INTO query_favorites (id, name, sql, created_at) VALUES (?1, ?2, ?3, ?4)",
                params![
                    fav.id.to_string(),
                    fav.name,
                    fav.sql,
                    fav.created_at.to_rfc3339()
                ],
            )
            .map_err(|e| AceError::Io(e.to_string()))?;
        Ok(fav)
    }

    pub fn list_favorites(&self) -> AceResult<Vec<QueryFavorite>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, name, sql, created_at FROM query_favorites ORDER BY created_at DESC",
            )
            .map_err(|e| AceError::Io(e.to_string()))?;
        let rows = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let created: String = row.get(3)?;
                Ok(QueryFavorite {
                    id: Uuid::parse_str(&id).unwrap_or_else(|_| Uuid::nil()),
                    name: row.get(1)?,
                    sql: row.get(2)?,
                    created_at: DateTime::parse_from_rfc3339(&created)
                        .map(|d| d.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })
            .map_err(|e| AceError::Io(e.to_string()))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| AceError::Io(e.to_string()))?);
        }
        Ok(out)
    }

    pub fn delete_favorite(&self, id: Uuid) -> AceResult<()> {
        self.conn
            .execute(
                "DELETE FROM query_favorites WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(|e| AceError::Io(e.to_string()))?;
        Ok(())
    }

    pub fn pin_table(&self, database: &str, table: &str) -> AceResult<()> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO pinned_tables (database_name, table_name) VALUES (?1, ?2)",
                params![database, table],
            )
            .map_err(|e| AceError::Io(e.to_string()))?;
        Ok(())
    }

    pub fn unpin_table(&self, database: &str, table: &str) -> AceResult<()> {
        self.conn
            .execute(
                "DELETE FROM pinned_tables WHERE database_name = ?1 AND table_name = ?2",
                params![database, table],
            )
            .map_err(|e| AceError::Io(e.to_string()))?;
        Ok(())
    }

    pub fn pinned_tables(&self) -> AceResult<Vec<(String, String)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT database_name, table_name FROM pinned_tables")
            .map_err(|e| AceError::Io(e.to_string()))?;
        let rows = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| AceError::Io(e.to_string()))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| AceError::Io(e.to_string()))?);
        }
        Ok(out)
    }
}