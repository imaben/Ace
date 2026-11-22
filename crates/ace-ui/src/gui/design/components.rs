//! Shared Sequel Ace UI primitives. Screens must compose these — not invent styles.

use super::tokens::tokens;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::label::Label;
use gpui_kit::component::{h_flex, v_flex, Selectable as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

fn eid(id: impl Into<SharedString>) -> SharedString {
    id.into()
}

// ─── Text ───────────────────────────────────────────────────────────────────

pub fn text_label(s: impl Into<SharedString>) -> impl IntoElement {
    let t = tokens();
    Label::new(s).text_sm().text_color(t.color.label)
}

pub fn text_muted(s: impl Into<SharedString>) -> impl IntoElement {
    let t = tokens();
    Label::new(s).text_sm().text_color(t.color.muted)
}

pub fn text_disabled(s: impl Into<SharedString>) -> impl IntoElement {
    let t = tokens();
    Label::new(s).text_sm().text_color(t.color.disabled)
}

pub fn text_danger(s: impl Into<SharedString>) -> impl IntoElement {
    let t = tokens();
    Label::new(s).text_sm().text_color(t.color.danger)
}

// ─── Surfaces ───────────────────────────────────────────────────────────────

pub fn window_bg() -> Div {
    let t = tokens();
    div().bg(t.color.window).text_color(t.color.label)
}

pub fn sidebar_panel() -> Div {
    let t = tokens();
    v_flex()
        .w(t.layout.sidebar_width)
        .h_full()
        .bg(t.color.sidebar)
        .border_r_1()
        .border_color(t.color.border)
}

pub fn tables_sidebar_panel() -> Div {
    let t = tokens();
    v_flex()
        .w(t.layout.tables_sidebar_width)
        .h_full()
        .bg(t.color.sidebar)
        .border_r_1()
        .border_color(t.color.border)
}

pub fn card() -> Div {
    let t = tokens();
    v_flex()
        .w(t.layout.connection_card_width)
        .rounded(t.layout.radius_md)
        .bg(t.color.card)
        .border_1()
        .border_color(t.color.border)
        .px(t.space.lg)
        .py(t.space.md)
        .gap(t.space.xs)
}

pub fn panel_header() -> Div {
    let t = tokens();
    h_flex()
        .h(t.layout.header_height)
        .w_full()
        .px(t.space.sm)
        .items_center()
        .bg(t.color.toolbar)
        .border_b_1()
        .border_color(t.color.border)
}

pub fn panel_footer() -> Div {
    let t = tokens();
    h_flex()
        .h(t.layout.footer_height)
        .w_full()
        .px(t.space.sm)
        .gap(t.space.xs)
        .items_center()
        .border_t_1()
        .border_color(t.color.border)
}

// ─── Toolbar ────────────────────────────────────────────────────────────────

pub fn toolbar() -> Div {
    let t = tokens();
    h_flex()
        .h(t.layout.toolbar_height)
        .w_full()
        .px(t.space.md)
        .gap(t.space.md)
        .items_center()
        .bg(t.color.toolbar)
        .border_b_1()
        .border_color(t.color.border)
}

/// Secondary bar under the main toolbar (filters, pagination).
pub fn subtoolbar() -> Div {
    let t = tokens();
    h_flex()
        .h(px(32.))
        .w_full()
        .px(t.space.sm)
        .gap(t.space.sm)
        .items_center()
        .bg(t.color.toolbar)
        .border_b_1()
        .border_color(t.color.border)
}

/// Compact action strip (page controls, row ops).
pub fn action_strip() -> Div {
    let t = tokens();
    h_flex()
        .h(px(28.))
        .w_full()
        .px(t.space.sm)
        .gap(t.space.sm)
        .items_center()
        .border_b_1()
        .border_color(t.color.border)
}

/// Inactive toolbar labels (connection screen).
pub fn toolbar_inactive(items: &[&str]) -> impl IntoElement {
    toolbar().children(items.iter().map(|s| text_disabled((*s).to_string())))
}

/// Active toolbar with selectable mode buttons.
pub fn toolbar_mode_button(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
    selected: bool,
) -> Button {
    Button::new(eid(id))
        .label(label)
        .ghost()
        .compact()
        .selected(selected)
        .when(selected, |b| b.secondary())
}

// ─── Form ───────────────────────────────────────────────────────────────────

pub fn form_row(label: impl Into<SharedString>, field: impl IntoElement) -> impl IntoElement {
    let t = tokens();
    h_flex()
        .w_full()
        .h(t.layout.control_height)
        .gap(t.space.sm)
        .items_center()
        .child(
            div()
                .w(t.layout.form_label_width)
                .child(text_muted(label)),
        )
        .child(div().flex_1().h(px(26.)).child(field))
}

pub fn segmented_tabs() -> Div {
    let t = tokens();
    h_flex()
        .w_full()
        .gap(t.space.xxs)
        .p(t.space.xxs)
        .rounded(t.layout.radius_md)
        .bg(t.color.field)
}

pub fn tab_button(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
    selected: bool,
) -> Button {
    Button::new(eid(id))
        .label(label)
        .ghost()
        .compact()
        .flex_1()
        .selected(selected)
        .when(selected, |b| b.secondary())
}

pub fn checkbox_row(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
    on: bool,
) -> Button {
    let mark = if on { "☑" } else { "☐" };
    Button::new(eid(id))
        .label(format!("{mark}  {}", label.into()))
        .ghost()
        .compact()
        .justify_start()
}

pub fn color_dot(rgb_opt: Option<(u8, u8, u8)>, selected: bool, none: bool) -> AnyElement {
    let t = tokens();
    if none {
        return div()
            .size(px(16.))
            .rounded(px(8.))
            .border_1()
            .border_color(if selected {
                t.color.accent
            } else {
                t.color.muted
            })
            .flex()
            .items_center()
            .justify_center()
            .child(div().w(px(8.)).h(px(1.)).bg(t.color.muted))
            .into_any_element();
    }
    let (r, g, b) = rgb_opt.unwrap_or((180, 180, 180));
    let hex = ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
    div()
        .size(px(16.))
        .rounded(px(8.))
        .bg(rgb(hex))
        .when(selected, |el| el.border_2().border_color(rgb(0xffffff)))
        .when(!selected, |el| el.border_1().border_color(t.color.border))
        .into_any_element()
}

pub fn color_swatch_button(
    id: impl Into<SharedString>,
    rgb_opt: Option<(u8, u8, u8)>,
    selected: bool,
    none: bool,
) -> Button {
    Button::new(eid(id))
        .ghost()
        .compact()
        .child(color_dot(rgb_opt, selected, none))
}

// ─── Buttons ────────────────────────────────────────────────────────────────

/// Blue accent action (Connect). Never use theme `.primary()` for this.
pub fn accent_button(label: impl Into<SharedString>) -> Div {
    let t = tokens();
    div()
        .h(px(32.))
        .min_w(px(120.))
        .px_5()
        .rounded(px(6.))
        .bg(t.color.accent)
        .flex()
        .items_center()
        .justify_center()
        .child(Label::new(label).text_color(t.color.accent_fg))
}

pub fn ghost_icon_button(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Button {
    Button::new(eid(id)).label(label).ghost().compact()
}

// ─── Source list ────────────────────────────────────────────────────────────

pub fn list_row(selected: bool) -> Div {
    let t = tokens();
    h_flex()
        .w_full()
        .h(t.layout.list_row_height)
        .items_center()
        .gap(t.space.sm)
        .px(t.space.sm)
        .rounded(t.layout.radius_sm)
        .when(selected, |el| el.bg(t.color.selected))
}

pub fn list_row_button(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
) -> Button {
    Button::new(eid(id))
        .label(label)
        .ghost()
        .compact()
        .w_full()
        .justify_start()
}

// ─── Data grid ──────────────────────────────────────────────────────────────

pub fn grid_header(columns: &[String]) -> impl IntoElement {
    let t = tokens();
    h_flex()
        .h(t.layout.header_height)
        .px(t.space.sm)
        .gap(t.space.sm)
        .items_center()
        .bg(t.color.header)
        .border_b_1()
        .border_color(t.color.border)
        .children(columns.iter().map(|c| {
            div()
                .w(px(140.))
                .child(text_muted(c.clone()))
        }))
}

pub fn grid_body(columns: &[String], rows: &[Vec<String>]) -> AnyElement {
    let t = tokens();
    if columns.is_empty() {
        return empty_state("No data");
    }
    v_flex()
        .flex_1()
        .bg(t.color.window)
        .child(grid_header(columns))
        .child(
            v_flex().flex_1().children(rows.iter().take(500).enumerate().map(|(i, row)| {
                h_flex()
                    .h(t.layout.row_height)
                    .px(t.space.sm)
                    .gap(t.space.sm)
                    .items_center()
                    .when(i % 2 == 1, |el| el.bg(t.color.zebra))
                    .children(row.iter().map(|cell| {
                        div()
                            .w(px(140.))
                            .overflow_hidden()
                            .child(text_label(truncate(cell, 28)))
                    }))
            })),
        )
        .into_any_element()
}

pub fn empty_state(msg: impl Into<SharedString>) -> AnyElement {
    let t = tokens();
    v_flex()
        .flex_1()
        .items_center()
        .justify_center()
        .bg(t.color.window)
        .child(text_muted(msg))
        .into_any_element()
}

pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() > max {
        format!("{}…", s.chars().take(max).collect::<String>())
    } else {
        s.to_string()
    }
}

// ─── Content helpers ────────────────────────────────────────────────────────

pub fn info_row(key: impl Into<SharedString>, value: impl Into<SharedString>) -> impl IntoElement {
    let t = tokens();
    h_flex()
        .gap(t.space.md)
        .child(div().w(px(120.)).child(text_muted(key)))
        .child(text_label(value))
}

pub fn code_block(body: impl Into<SharedString>) -> impl IntoElement {
    let t = tokens();
    div()
        .p(t.space.md)
        .rounded(t.layout.radius_sm)
        .bg(t.color.card)
        .border_1()
        .border_color(t.color.border)
        .child(Label::new(body).text_sm().text_color(t.color.label))
}

pub fn content_pane() -> Div {
    let t = tokens();
    v_flex().flex_1().h_full().bg(t.color.window)
}

pub fn history_sidebar() -> Div {
    let t = tokens();
    v_flex()
        .w(px(220.))
        .h_full()
        .bg(t.color.sidebar)
        .border_l_1()
        .border_color(t.color.border)
}

pub fn query_editor_pane() -> Div {
    let t = tokens();
    v_flex()
        .h(px(160.))
        .p(t.space.sm)
        .gap(t.space.sm)
        .border_b_1()
        .border_color(t.color.border)
        .bg(t.color.card)
}

pub fn form_hint(msg: impl Into<SharedString>) -> impl IntoElement {
    div().mb(tokens().space.md).child(text_muted(msg))
}

pub fn form_center() -> Div {
    let t = tokens();
    v_flex()
        .flex_1()
        .h_full()
        .bg(t.color.window)
        .items_center()
        .justify_center()
}

// ─── App chrome ─────────────────────────────────────────────────────────────

/// Root shell: toolbar + body. Every screen goes through this.
pub fn app_shell(toolbar: impl IntoElement, body: impl IntoElement) -> impl IntoElement {
    window_bg()
        .size_full()
        .child(toolbar)
        .child(div().flex_1().size_full().child(body))
}

/// Horizontal split: sidebar | main.
pub fn split_h(sidebar: impl IntoElement, main: impl IntoElement) -> impl IntoElement {
    h_flex()
        .flex_1()
        .size_full()
        .child(sidebar)
        .child(main)
}
