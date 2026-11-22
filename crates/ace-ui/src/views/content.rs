use ace_core::Value;
use ace_driver::meta::ColumnMeta;
use ace_driver::result::ResultColumn;
use ace_sql::filter::FilterNode;

#[derive(Debug, Clone, Default)]
pub struct ContentViewState {
    pub columns: Vec<ResultColumn>,
    pub column_meta: Vec<ColumnMeta>,
    pub rows: Vec<Vec<Value>>,
    pub total_rows: Option<u64>,
    pub page: usize,
    pub page_size: usize,
    pub sort_column: Option<String>,
    pub sort_desc: bool,
    pub filter: Option<FilterNode>,
    pub filter_sql: String,
    pub loading: bool,
    pub error: Option<String>,
    pub selected_rows: Vec<usize>,
    pub editing_cell: Option<(usize, usize)>,
    pub edit_draft: String,
    pub dirty: bool,
}

impl ContentViewState {
    pub fn new(page_size: usize) -> Self {
        Self {
            page_size,
            ..Default::default()
        }
    }

    pub fn offset(&self) -> usize {
        self.page.saturating_mul(self.page_size)
    }

    pub fn order_by_sql(&self) -> Option<String> {
        self.sort_column.as_ref().map(|c| {
            format!(
                "`{}` {}",
                c.replace('`', "``"),
                if self.sort_desc { "DESC" } else { "ASC" }
            )
        })
    }

    pub fn where_sql(&self) -> Option<String> {
        if !self.filter_sql.trim().is_empty() {
            return Some(self.filter_sql.clone());
        }
        self.filter
            .as_ref()
            .map(|f| f.to_where_clause(&|c| format!("`{}`", c.replace('`', "``"))))
            .filter(|s| !s.is_empty())
    }

    pub fn cell_preview(&self, row: usize, col: usize, max_len: usize) -> String {
        self.rows
            .get(row)
            .and_then(|r| r.get(col))
            .map(|v| v.display_preview(max_len))
            .unwrap_or_default()
    }
}