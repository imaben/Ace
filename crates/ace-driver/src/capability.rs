use serde::{Deserialize, Serialize};

/// Feature flags advertised by a dialect / connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Capabilities {
    pub schemas: bool,
    pub users: bool,
    pub triggers: bool,
    pub views: bool,
    pub routines: bool,
    pub foreign_keys: bool,
    pub indexes: bool,
    pub explain: bool,
    pub kill_query: bool,
    pub process_list: bool,
    pub server_variables: bool,
    pub streaming_results: bool,
    pub multiple_statements: bool,
    pub transactions: bool,
    pub geometry: bool,
    pub json_type: bool,
    pub fulltext: bool,
    pub events: bool,
}

impl Capabilities {
    pub fn mysql_full() -> Self {
        Self {
            schemas: true,
            users: true,
            triggers: true,
            views: true,
            routines: true,
            foreign_keys: true,
            indexes: true,
            explain: true,
            kill_query: true,
            process_list: true,
            server_variables: true,
            streaming_results: true,
            multiple_statements: true,
            transactions: true,
            geometry: true,
            json_type: true,
            fulltext: true,
            events: true,
        }
    }

    pub fn postgres_full() -> Self {
        Self {
            schemas: true,
            users: true,
            triggers: true,
            views: true,
            routines: true,
            foreign_keys: true,
            indexes: true,
            explain: true,
            kill_query: true,
            process_list: true,
            server_variables: true,
            streaming_results: true,
            multiple_statements: false,
            transactions: true,
            geometry: false,
            json_type: true,
            fulltext: false,
            events: false,
        }
    }

    pub fn sqlite_full() -> Self {
        Self {
            schemas: false,
            users: false,
            triggers: true,
            views: true,
            routines: false,
            foreign_keys: true,
            indexes: true,
            explain: true,
            kill_query: false,
            process_list: false,
            server_variables: false,
            streaming_results: true,
            multiple_statements: true,
            transactions: true,
            geometry: false,
            json_type: true,
            fulltext: true,
            events: false,
        }
    }
}