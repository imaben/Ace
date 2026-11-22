use crate::connection::ConnectionInfo;
use uuid::Uuid;

/// Application-level events published by sessions / UI coordinators.
#[derive(Debug, Clone)]
pub enum AppEvent {
    ConnectionOpened {
        session_id: Uuid,
        info: ConnectionInfo,
    },
    ConnectionClosed {
        session_id: Uuid,
    },
    ConnectionLost {
        session_id: Uuid,
        reason: String,
    },
    DatabaseSelected {
        session_id: Uuid,
        database: String,
    },
    TableSelected {
        session_id: Uuid,
        database: String,
        table: String,
    },
    QueryStarted {
        session_id: Uuid,
        query_id: Uuid,
        sql: String,
    },
    QueryFinished {
        session_id: Uuid,
        query_id: Uuid,
        success: bool,
        message: String,
        duration_ms: u64,
    },
    ConsoleMessage {
        session_id: Uuid,
        sql: String,
        error: Option<String>,
    },
    FavoritesChanged,
    SettingsChanged,
}