use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolName {
    ListConnections,
    ListDatabases,
    ListTables,
    DescribeTable,
    GetTableDdl,
    RunQuery,
    ExplainQuery,
    SampleTable,
    CountRows,
    TableSizes,
    ProcessList,
    KillQuery,
    ListViews,
    ListProcedures,
    ListFunctions,
    ListTriggers,
    GetRoutineDefinition,
    ServerInfo,
    ExportResults,
}

impl ToolName {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ListConnections => "list_connections",
            Self::ListDatabases => "list_databases",
            Self::ListTables => "list_tables",
            Self::DescribeTable => "describe_table",
            Self::GetTableDdl => "get_table_ddl",
            Self::RunQuery => "run_query",
            Self::ExplainQuery => "explain_query",
            Self::SampleTable => "sample_table",
            Self::CountRows => "count_rows",
            Self::TableSizes => "table_sizes",
            Self::ProcessList => "process_list",
            Self::KillQuery => "kill_query",
            Self::ListViews => "list_views",
            Self::ListProcedures => "list_procedures",
            Self::ListFunctions => "list_functions",
            Self::ListTriggers => "list_triggers",
            Self::GetRoutineDefinition => "get_routine_definition",
            Self::ServerInfo => "server_info",
            Self::ExportResults => "export_results",
        }
    }

    pub fn all() -> &'static [ToolName] {
        &[
            Self::ListConnections,
            Self::ListDatabases,
            Self::ListTables,
            Self::DescribeTable,
            Self::GetTableDdl,
            Self::RunQuery,
            Self::ExplainQuery,
            Self::SampleTable,
            Self::CountRows,
            Self::TableSizes,
            Self::ProcessList,
            Self::KillQuery,
            Self::ListViews,
            Self::ListProcedures,
            Self::ListFunctions,
            Self::ListTriggers,
            Self::GetRoutineDefinition,
            Self::ServerInfo,
            Self::ExportResults,
        ]
    }

    pub fn is_mutating(self) -> bool {
        matches!(self, Self::KillQuery | Self::RunQuery | Self::ExportResults)
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::ListConnections => "List open database connections",
            Self::ListDatabases => "List databases on a connection",
            Self::ListTables => "List tables in a database",
            Self::DescribeTable => "Describe table columns",
            Self::GetTableDdl => "Get CREATE TABLE DDL",
            Self::RunQuery => "Run a SQL query (respects read-only mode)",
            Self::ExplainQuery => "EXPLAIN a SQL query",
            Self::SampleTable => "Sample rows from a table",
            Self::CountRows => "Count rows in a table",
            Self::TableSizes => "List table sizes",
            Self::ProcessList => "Show server process list",
            Self::KillQuery => "Kill a running query",
            Self::ListViews => "List views",
            Self::ListProcedures => "List stored procedures",
            Self::ListFunctions => "List functions",
            Self::ListTriggers => "List triggers",
            Self::GetRoutineDefinition => "Get routine definition",
            Self::ServerInfo => "Server version and status",
            Self::ExportResults => "Export query results to a file path",
        }
    }
}