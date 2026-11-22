//! Reusable virtualized Sequel-Ace-style DataTable delegate built on gpui-component DataTable.

use crate::gui::theme::Palette;
use gpui_kit::component::table::{Column, ColumnSort, TableDelegate, TableState};
use gpui_kit::component::{h_flex, ActiveTheme as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct GridCell {
    pub display: String,
    pub is_null: bool,
    pub is_numeric: bool,
}

impl GridCell {
    pub fn from_value(v: &ace_core::Value) -> Self {
        match v {
            ace_core::Value::Null => Self {
                display: "NULL".into(),
                is_null: true,
                is_numeric: false,
            },
            ace_core::Value::I64(_) | ace_core::Value::U64(_) | ace_core::Value::F64(_) => Self {
                display: v.to_string(),
                is_null: false,
                is_numeric: true,
            },
            _ => Self {
                display: v.to_string(),
                is_null: false,
                is_numeric: false,
            },
        }
    }

    pub fn text(s: impl Into<String>) -> Self {
        let s = s.into();
        let is_null = s == "NULL";
        Self {
            display: s,
            is_null,
            is_numeric: false,
        }
    }
}

pub type SortHandler = Rc<dyn Fn(String, ColumnSort, &mut Window, &mut App)>;

pub struct AceGridDelegate {
    columns: Vec<Column>,
    rows: Vec<Vec<GridCell>>,
    sortable: bool,
    empty_label: SharedString,
    on_sort: Option<SortHandler>,
    loading: bool,
}

impl AceGridDelegate {
    pub fn new(empty_label: impl Into<SharedString>, sortable: bool) -> Self {
        Self {
            columns: Vec::new(),
            rows: Vec::new(),
            sortable,
            empty_label: empty_label.into(),
            on_sort: None,
            loading: false,
        }
    }

    pub fn with_on_sort(mut self, handler: SortHandler) -> Self {
        self.on_sort = Some(handler);
        self
    }

    pub fn set_loading(&mut self, loading: bool) {
        self.loading = loading;
    }

    pub fn set_data(
        &mut self,
        headers: Vec<String>,
        rows: Vec<Vec<GridCell>>,
        sort_col: Option<&str>,
        sort_desc: bool,
    ) {
        let col_count = headers.len();
        let mut widths: Vec<f32> = headers
            .iter()
            .map(|h| ((h.chars().count() as f32) * 8.2 + 34.0).clamp(72.0, 260.0))
            .collect();

        for row in rows.iter().take(60) {
            for (ci, cell) in row.iter().enumerate() {
                if ci < col_count {
                    let first_line_len = cell
                        .display
                        .lines()
                        .next()
                        .map(|l| l.chars().count())
                        .unwrap_or(0);
                    let w = (first_line_len as f32 * 7.6 + 24.0).clamp(64.0, 320.0);
                    if w > widths[ci] {
                        widths[ci] = w;
                    }
                }
            }
        }

        self.columns = headers
            .into_iter()
            .enumerate()
            .map(|(idx, name)| {
                let mut col = Column::new(name.clone(), name.clone())
                    .width(px(widths[idx]))
                    .min_width(px(50.))
                    .max_width(px(640.));
                if self.sortable {
                    let sort = match sort_col {
                        Some(sc) if sc == name => {
                            if sort_desc {
                                ColumnSort::Descending
                            } else {
                                ColumnSort::Ascending
                            }
                        }
                        _ => ColumnSort::Default,
                    };
                    col = col.sort(sort);
                }
                col
            })
            .collect();

        self.rows = rows;
    }

    pub fn set_string_data(&mut self, headers: Vec<String>, rows: Vec<Vec<String>>) {
        let cells = rows
            .into_iter()
            .map(|r| r.into_iter().map(GridCell::text).collect())
            .collect();
        self.set_data(headers, cells, None, false);
    }

    pub fn cell(&self, row: usize, col: usize) -> Option<&GridCell> {
        self.rows.get(row).and_then(|r| r.get(col))
    }

    pub fn row(&self, row: usize) -> Option<&[GridCell]> {
        self.rows.get(row).map(|r| r.as_slice())
    }

    pub fn column_names(&self) -> Vec<String> {
        self.columns.iter().map(|c| c.name.to_string()).collect()
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn clear(&mut self) {
        self.columns.clear();
        self.rows.clear();
    }
}

impl TableDelegate for AceGridDelegate {
    fn columns_count(&self, _cx: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _cx: &App) -> usize {
        self.rows.len()
    }

    fn column(&self, col_ix: usize, _cx: &App) -> Column {
        self.columns.get(col_ix).cloned().unwrap_or_default()
    }

    fn perform_sort(
        &mut self,
        col_ix: usize,
        sort: ColumnSort,
        window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) {
        if let Some(col) = self.columns.get(col_ix) {
            let col_name = col.name.to_string();
            for (i, c) in self.columns.iter_mut().enumerate() {
                if i == col_ix {
                    c.sort = Some(sort);
                } else if c.sort.is_some() {
                    c.sort = Some(ColumnSort::Default);
                }
            }
            if let Some(cb) = self.on_sort.clone() {
                cb(col_name, sort, window, cx);
            }
        }
    }

    fn fit_column(
        &mut self,
        col_ix: usize,
        _window: &mut Window,
        _cx: &mut Context<TableState<Self>>,
    ) -> Option<Pixels> {
        let header_len = self
            .columns
            .get(col_ix)
            .map(|c| c.name.chars().count())
            .unwrap_or(4);
        let mut max_chars = header_len;
        for row in self.rows.iter().take(200) {
            if let Some(cell) = row.get(col_ix) {
                let len = cell
                    .display
                    .lines()
                    .next()
                    .map(|l| l.chars().count())
                    .unwrap_or(0);
                if len > max_chars {
                    max_chars = len;
                }
            }
        }
        Some(px((max_chars as f32 * 8.0 + 36.0).clamp(64.0, 560.0)))
    }

    fn render_th(
        &mut self,
        col_ix: usize,
        _window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let p = Palette::for_app(cx);
        let name = self
            .columns
            .get(col_ix)
            .map(|c| c.name.clone())
            .unwrap_or_default();
        div()
            .w_full()
            .overflow_hidden()
            .text_ellipsis()
            .whitespace_nowrap()
            .text_xs()
            .font_weight(FontWeight::MEDIUM)
            .text_color(p.text_secondary)
            .child(name)
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let p = Palette::for_app(cx);
        let cell = self.rows.get(row_ix).and_then(|r| r.get(col_ix));
        let (text, is_null, is_numeric) = match cell {
            Some(c) => {
                let preview = single_line_preview(&c.display, 140);
                (preview, c.is_null, c.is_numeric)
            }
            None => (String::new(), false, false),
        };

        div()
            .w_full()
            .overflow_hidden()
            .text_ellipsis()
            .whitespace_nowrap()
            .text_xs()
            .when(is_null, |el| el.italic().text_color(p.null_fg))
            .when(!is_null, |el| el.text_color(p.text))
            .when(is_numeric, |el| el.text_right())
            .child(text)
    }

    fn cell_label(&self, row_ix: usize, col_ix: usize, _cx: &App) -> Option<SharedString> {
        self.rows
            .get(row_ix)
            .and_then(|r| r.get(col_ix))
            .map(|c| SharedString::from(c.display.clone()))
    }

    fn render_empty(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        h_flex()
            .size_full()
            .items_center()
            .justify_center()
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .child(self.empty_label.clone())
    }

    fn loading(&self, _cx: &App) -> bool {
        self.loading
    }
}

fn single_line_preview(s: &str, max: usize) -> String {
    let mut out = String::with_capacity(s.len().min(max + 4));
    for (idx, ch) in s.chars().enumerate() {
        if idx >= max {
            out.push('…');
            break;
        }
        match ch {
            '\n' => out.push_str("↵ "),
            '\r' => {}
            '\t' => out.push(' '),
            other => out.push(other),
        }
    }
    out
}
