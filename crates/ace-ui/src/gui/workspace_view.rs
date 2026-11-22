//! Sequel Ace Workspace screen: Tab bar, Unified Toolbar, Resizable Tables Sidebar,
//! all 6 Database Views (Structure, Content, Relations, Triggers, Table Info, Query),
//! and the collapsible bottom SQL Query Console.

use crate::gui::loading::{loading_inline, loading_pane, loading_pane_empty};
use crate::gui::root::AceApp;
use crate::gui::shortcuts::mod_shortcut;
use crate::gui::state::{QueryDrawerTab, WorkspaceSession};
use crate::gui::theme::{favorite_tint, Palette};
use crate::views::MainViewTab;
use ace_driver::meta::ObjectKind;
use gpui_kit::assets::IconName as Lucide;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Editor, Input};
use gpui_kit::component::resizable::{h_resizable, resizable_panel, v_resizable};
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::select::Select;
use gpui_kit::component::table::DataTable;
use gpui_kit::component::{h_flex, v_flex, Disableable as _, Icon, Sizable as _, TitleBar};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

impl AceApp {
    /// Integrated custom TitleBar + multi-tab bar (transparent macOS traffic lights + cross-platform controls).
    pub(crate) fn render_tab_bar(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = Palette::for_app(cx);
        let active_idx = self.active_tab;
        let can_close = self.tabs.len() > 1;

        TitleBar::new()
            .bg(p.tabbar_bg)
            .border_color(p.border)
            .child(
                h_flex()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .gap(px(4.))
                    .items_center()
                    .when(!cfg!(target_os = "macos"), |el| {
                        el.child(
                            div()
                                .flex()
                                .items_center()
                                .mr(px(4.))
                                .child(
                                    img("icons/ace-64.png")
                                        .size(px(18.))
                                        .rounded(px(4.))
                                        .flex_shrink_0(),
                                ),
                        )
                    })
                    .children(self.tabs.iter().enumerate().map(|(idx, tab)| {
                        let is_active = idx == active_idx;
                        let title = tab.title(cx);
                        let tint = favorite_tint(tab.color_index());
                        let connected = tab.session.is_some();

                        h_flex()
                            .id(SharedString::from(format!("win-tab-{}", tab.id)))
                            .occlude()
                            .h(px(24.))
                            .max_w(px(220.))
                            .min_w(px(110.))
                            .px(px(9.))
                            .gap(px(6.))
                            .items_center()
                            .rounded(px(6.))
                            .cursor_pointer()
                            .when(is_active, |el| {
                                el.bg(p.toolbar_bg)
                                    .border_1()
                                    .border_color(p.border)
                            })
                            .when(!is_active, |el| el.hover(|h| h.bg(p.hover_bg)))
                            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                cx.stop_propagation();
                            })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.switch_tab(idx, cx);
                            }))
                            .child(match tint {
                                Some(c) => div()
                                    .size(px(7.))
                                    .rounded(px(4.))
                                    .bg(c)
                                    .flex_shrink_0()
                                    .into_any_element(),
                                None => div()
                                    .size(px(7.))
                                    .rounded(px(4.))
                                    .bg(if connected { p.success } else { p.text_disabled })
                                    .flex_shrink_0()
                                    .into_any_element(),
                            })
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .whitespace_nowrap()
                                    .text_size(px(11.))
                                    .when(is_active, |el| {
                                        el.font_weight(FontWeight::MEDIUM).text_color(p.text)
                                    })
                                    .when(!is_active, |el| el.text_color(p.text_muted))
                                    .child(title),
                            )
                            .when(can_close, |el| {
                                el.child(
                                    div()
                                        .id(SharedString::from(format!("close-tab-{}", tab.id)))
                                        .size(px(15.))
                                        .rounded(px(3.))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .hover(|h| h.bg(p.hover_bg))
                                        .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                            cx.stop_propagation();
                                        })
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            cx.stop_propagation();
                                            this.close_tab(idx, window, cx);
                                        }))
                                        .child(
                                            Icon::new(Lucide::X)
                                                .size(px(10.))
                                                .text_color(p.text_muted),
                                        ),
                                )
                            })
                    }))
                    .child(
                        div()
                            .occlude()
                            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                cx.stop_propagation();
                            })
                            .child(
                                Button::new("new-win-tab")
                                    .ghost()
                                    .xsmall()
                                    .icon(Icon::new(Lucide::Plus).size(px(12.)))
                                    .tooltip(format!("New Connection Tab ({})", mod_shortcut("T")))
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.new_tab(window, cx);
                                    })),
                            ),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .occlude()
                            .mr(px(4.))
                            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                cx.stop_propagation();
                            })
                            .child(
                                Button::new("global-theme-toggle")
                                    .ghost()
                                    .xsmall()
                                    .icon(
                                        Icon::new(if p.is_dark { Lucide::Sun } else { Lucide::Moon })
                                            .size(px(13.)),
                                    )
                                    .tooltip(if p.is_dark {
                                        "Switch to Light Theme"
                                    } else {
                                        "Switch to Dark Theme"
                                    })
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.toggle_theme(window, cx);
                                    })),
                            ),
                    ),
            )
            .into_any_element()
    }

    /// Inactive minimalist toolbar shown on the Connection screen (matches Sequel Ace chrome).
    pub(crate) fn render_connection_toolbar(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = Palette::for_app(cx);
        h_flex()
            .h(px(40.))
            .w_full()
            .px(px(12.))
            .gap(px(14.))
            .items_center()
            .bg(p.toolbar_bg)
            .border_b_1()
            .border_color(p.border)
            .child(
                h_flex()
                    .w(px(210.))
                    .gap(px(6.))
                    .items_center()
                    .child(
                        Icon::new(Lucide::Database)
                            .size(px(13.))
                            .text_color(p.text_disabled),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(p.text_disabled)
                            .child("Choose Database…"),
                    ),
            )
            .child(div().w(px(1.)).h(px(18.)).bg(p.hairline))
            .children(MainViewTab::all().iter().map(|tab| {
                h_flex()
                    .gap(px(5.))
                    .items_center()
                    .child(
                        Icon::new(tab_icon(*tab))
                            .size(px(13.))
                            .text_color(p.text_disabled),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(p.text_disabled)
                            .child(tab.label()),
                    )
            }))
            .child(div().flex_1())
            .child(
                div()
                    .text_xs()
                    .text_color(p.text_disabled)
                    .child("Ace"),
            )
            .into_any_element()
    }

    pub(crate) fn render_workspace_screen(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let p = Palette::for_app(cx);
        let Some(session) = self.active_tab().session.as_ref() else {
            return div().flex_1().bg(p.window_bg).into_any_element();
        };

        let tint = favorite_tint(session.info.color_index);
        let console_open = session.console_open;

        let toolbar = self.render_workspace_toolbar(session, p, cx);
        let sidebar = self.render_tables_sidebar(session, p, cx);
        let main_view = self.render_active_view(session, p, cx);

        let right_pane: AnyElement = if console_open {
            let console_pane = self.render_console_drawer(session, p, cx);
            v_resizable("workspace-console-split")
                .child(resizable_panel().child(main_view))
                .child(
                    resizable_panel()
                        .size(px(190.))
                        .size_range(px(110.)..px(420.))
                        .child(console_pane),
                )
                .into_any_element()
        } else {
            main_view
        };

        v_flex()
            .flex_1()
            .min_h_0()
            .w_full()
            .bg(p.window_bg)
            .when_some(tint, |el, c| el.child(div().w_full().h(px(2.5)).bg(c)))
            .child(toolbar)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .child(
                        h_resizable("workspace-main-split")
                            .child(
                                resizable_panel()
                                    .size(px(228.))
                                    .size_range(px(160.)..px(380.))
                                    .child(sidebar),
                            )
                            .child(resizable_panel().child(right_pane)),
                    ),
            )
            .into_any_element()
    }

    fn render_workspace_toolbar(
        &self,
        session: &WorkspaceSession,
        p: Palette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let active_tab = session.main_tab;
        let console_open = session.console_open;
        let console_count = session.console.len();
        let is_sqlite = session.info.engine == ace_core::EngineKind::Sqlite;

        h_flex()
            .h(px(42.))
            .w_full()
            .px(px(10.))
            .gap(px(8.))
            .items_center()
            .bg(p.toolbar_bg)
            .border_b_1()
            .border_color(p.border)
            // Left: Choose Database dropdown + Add/Refresh DB
            .child(
                h_flex()
                    .w(px(218.))
                    .flex_shrink_0()
                    .gap(px(4.))
                    .items_center()
                    .child(
                        div().flex_1().min_w_0().child(
                            Select::new(&session.db_select)
                                .small()
                                .placeholder("Choose Database…")
                                .menu_width(px(220.)),
                        ),
                    )
                    .when(!is_sqlite, |el| {
                        el.child(
                            Button::new("tb-add-db")
                                .ghost()
                                .xsmall()
                                .icon(Icon::new(Lucide::Plus).size(px(12.)))
                                .tooltip("Add Database")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.open_add_database_modal(window, cx);
                                })),
                        )
                        .child(
                            Button::new("tb-drop-db")
                                .ghost()
                                .xsmall()
                                .icon(Icon::new(Lucide::Minus).size(px(12.)))
                                .tooltip("Drop Current Database")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.request_drop_current_database(cx);
                                })),
                        )
                    })
                    .child(
                        Button::new("tb-refresh-dbs")
                            .ghost()
                            .xsmall()
                            .loading(session.loading.objects())
                            .icon(Icon::new(Lucide::RefreshCw).size(px(12.)))
                            .tooltip("Refresh Databases & Tables")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.refresh_databases_and_tables(window, cx);
                            })),
                    ),
            )
            .child(div().w(px(1.)).h(px(20.)).bg(p.hairline))
            // Center: 6 Sequel Ace view buttons
            .child(
                h_flex()
                    .gap(px(2.))
                    .items_center()
                    .children(MainViewTab::all().iter().enumerate().map(|(i, tab)| {
                        let mode = *tab;
                        let selected = active_tab == mode;
                        h_flex()
                            .id(SharedString::from(format!("view-tab-{}", mode.label())))
                            .h(px(28.))
                            .px(px(9.))
                            .gap(px(5.))
                            .items_center()
                            .rounded(px(6.))
                            .cursor_pointer()
                            .when(selected, |el| {
                                el.bg(p.accent_subtle)
                                    .text_color(p.accent)
                                    .font_weight(FontWeight::MEDIUM)
                            })
                            .when(!selected, |el| {
                                el.text_color(p.text_secondary)
                                    .hover(|h| h.bg(p.hover_bg).text_color(p.text))
                            })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.set_main_tab(mode, cx);
                            }))
                            .child(
                                Icon::new(tab_icon(mode))
                                    .size(px(13.))
                                    .text_color(if selected { p.accent } else { p.text_muted }),
                            )
                            .child(div().text_xs().child(mode.label()))
                            .when(false, |el| el.child(format!("{}", i + 1)))
                    })),
            )
            .child(div().flex_1())
            // Status summary / active loading pill
            .child(if session.loading.is_any() {
                loading_inline(session.loading.active_label().to_string(), p)
            } else {
                div()
                    .max_w(px(240.))
                    .overflow_hidden()
                    .text_ellipsis()
                    .whitespace_nowrap()
                    .text_size(px(11.))
                    .text_color(p.text_muted)
                    .child(session.status.clone())
                    .into_any_element()
            })
            .child(div().w(px(1.)).h(px(20.)).bg(p.hairline))
            // Right utility icons (Console, Export, Import, Server Tools, Disconnect)
            .child(
                h_flex()
                    .gap(px(2.))
                    .items_center()
                    .child(
                        Button::new("tb-console")
                            .ghost()
                            .xsmall()
                            .toggled(console_open)
                            .icon(Icon::new(Lucide::SquareTerminal).size(px(13.)))
                            .label(if console_count > 0 {
                                format!("{console_count}")
                            } else {
                                String::new()
                            })
                            .tooltip(format!(
                                "Toggle SQL Console ({})",
                                mod_shortcut("Shift+C")
                            ))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.toggle_console(cx);
                            })),
                    )
                    .child(
                        Button::new("tb-export")
                            .ghost()
                            .xsmall()
                            .icon(Icon::new(Lucide::Download).size(px(13.)))
                            .tooltip("Export Table / Results (SQL, CSV, JSON, XML)")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_export_modal(window, cx);
                            })),
                    )
                    .child(
                        Button::new("tb-import")
                            .ghost()
                            .xsmall()
                            .icon(Icon::new(Lucide::Upload).size(px(13.)))
                            .tooltip("Import SQL File")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_import_modal(window, cx);
                            })),
                    )
                    .child(
                        Button::new("tb-server-tools")
                            .ghost()
                            .xsmall()
                            .icon(Icon::new(Lucide::Activity).size(px(13.)))
                            .tooltip("Server Processlist, Variables, Users & Info")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_server_tools_modal(window, cx);
                            })),
                    )
                    .child(
                        Button::new("tb-disconnect")
                            .ghost()
                            .xsmall()
                            .icon(Icon::new(Lucide::LogOut).size(px(13.)))
                            .tooltip("Disconnect")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.disconnect_active_tab(cx);
                            })),
                    ),
            )
            .into_any_element()
    }

    fn render_tables_sidebar(
        &self,
        session: &WorkspaceSession,
        p: Palette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let filter = session
            .object_filter
            .read(cx)
            .value()
            .to_string()
            .to_lowercase();
        let db = session.current_db.clone();
        let selected_table = session.selected_table.as_deref();

        let filtered: Vec<_> = session
            .objects
            .iter()
            .filter(|o| filter.is_empty() || o.name.to_lowercase().contains(&filter))
            .collect();

        let mut pinned_objs = Vec::new();
        let mut tables = Vec::new();
        let mut views = Vec::new();
        let mut routines = Vec::new();

        for obj in &filtered {
            let is_pinned = self
                .pinned_tables
                .iter()
                .any(|(d, t)| d == &db && t == &obj.name);
            if is_pinned {
                pinned_objs.push(*obj);
            }
            match obj.kind {
                ObjectKind::Table => tables.push(*obj),
                ObjectKind::View => views.push(*obj),
                ObjectKind::Procedure | ObjectKind::Function => routines.push(*obj),
                _ => tables.push(*obj),
            }
        }

        let mut rows: Vec<AnyElement> = Vec::new();

        let mut push_section = |title: &'static str,
                                items: &[&ace_driver::meta::DatabaseObject],
                                prefix: &'static str| {
            if items.is_empty() {
                return;
            }
            rows.push(
                div()
                    .px(px(10.))
                    .pt(px(8.))
                    .pb(px(3.))
                    .text_size(px(10.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(p.text_muted)
                    .child(format!("{title} ({})", items.len()))
                    .into_any_element(),
            );
            for obj in items {
                let name = obj.name.clone();
                let is_sel = selected_table == Some(name.as_str());
                let is_pin = self
                    .pinned_tables
                    .iter()
                    .any(|(d, t)| d == &db && t == &name);
                let icon = match obj.kind {
                    ObjectKind::View => Lucide::Eye,
                    ObjectKind::Procedure | ObjectKind::Function => Lucide::Code,
                    _ => Lucide::Table2,
                };
                let rows_hint = obj.rows_estimate.map(format_compact_count);
                let click_name = name.clone();

                rows.push(
                    h_flex()
                        .id(SharedString::from(format!("obj-{prefix}-{name}")))
                        .h(px(25.))
                        .w_full()
                        .px(px(9.))
                        .gap(px(7.))
                        .items_center()
                        .rounded(px(5.))
                        .cursor_pointer()
                        .when(is_sel, |el| el.bg(p.selected_bg))
                        .when(!is_sel, |el| el.hover(|h| h.bg(p.hover_bg)))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.select_table(click_name.clone(), window, cx);
                        }))
                        .child(
                            Icon::new(icon)
                                .size(px(12.))
                                .text_color(if is_sel { p.accent } else { p.text_muted }),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .overflow_hidden()
                                .text_ellipsis()
                                .whitespace_nowrap()
                                .text_xs()
                                .when(is_sel, |el| {
                                    el.font_weight(FontWeight::MEDIUM).text_color(p.text)
                                })
                                .when(!is_sel, |el| el.text_color(p.text_secondary))
                                .child(name),
                        )
                        .when(is_pin && prefix != "pin", |el| {
                            el.child(
                                Icon::new(Lucide::Pin)
                                    .size(px(10.))
                                    .text_color(p.text_disabled),
                            )
                        })
                        .when_some(rows_hint, |el, cnt| {
                            el.child(
                                div()
                                    .text_size(px(10.))
                                    .text_color(p.text_disabled)
                                    .child(cnt),
                            )
                        })
                        .into_any_element(),
                );
            }
        };

        push_section("PINNED", &pinned_objs, "pin");
        push_section("TABLES", &tables, "tbl");
        push_section("VIEWS", &views, "vw");
        push_section("ROUTINES", &routines, "fn");

        if rows.is_empty() {
            rows.push(
                div()
                    .px(px(12.))
                    .py(px(20.))
                    .text_xs()
                    .text_color(p.text_muted)
                    .child(if session.current_db.is_empty() {
                        "Choose a database above"
                    } else if filter.is_empty() {
                        "No tables in this database"
                    } else {
                        "No matching objects"
                    })
                    .into_any_element(),
            );
        }

        let is_selected_pinned = selected_table
            .map(|t| self.pinned_tables.iter().any(|(d, n)| d == &db && n == t))
            .unwrap_or(false);

        v_flex()
            .size_full()
            .bg(p.sidebar_bg)
            // Filter box
            .child(
                div().px(px(8.)).py(px(8.)).child(
                    Input::new(&session.object_filter)
                        .small()
                        .cleanable(true)
                        .prefix(
                            Icon::new(Lucide::Search)
                                .size(px(12.))
                                .text_color(p.text_muted),
                        ),
                ),
            )
            // Scrollable objects list (with scoped non-blocking loading indicator)
            .child(
                div().flex_1().min_h_0().w_full().child(loading_pane(
                    "sidebar-objects-loading",
                    session.loading.objects(),
                    session.objects.is_empty(),
                    "Loading tables…",
                    p,
                    div()
                        .size_full()
                        .overflow_y_scrollbar()
                        .child(
                            v_flex()
                                .w_full()
                                .px(px(5.))
                                .pb(px(10.))
                                .gap(px(1.))
                                .children(rows),
                        ),
                )),
            )
            // Bottom action bar (+ Add Table, - Drop Table, Pin, Refresh, count)
            .child(
                h_flex()
                    .h(px(30.))
                    .w_full()
                    .px(px(6.))
                    .gap(px(2.))
                    .items_center()
                    .bg(p.sidebar_bg)
                    .border_t_1()
                    .border_color(p.border)
                    .child(
                        Button::new("tbl-add")
                            .ghost()
                            .xsmall()
                            .icon(Icon::new(Lucide::Plus).size(px(13.)))
                            .tooltip("New Table")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_add_table_modal(window, cx);
                            })),
                    )
                    .child(
                        Button::new("tbl-drop")
                            .ghost()
                            .xsmall()
                            .icon(Icon::new(Lucide::Minus).size(px(13.)))
                            .tooltip("Drop Selected Table")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.request_drop_selected_table(cx);
                            })),
                    )
                    .child(
                        Button::new("tbl-pin")
                            .ghost()
                            .xsmall()
                            .toggled(is_selected_pinned)
                            .icon(
                                Icon::new(if is_selected_pinned {
                                    Lucide::PinOff
                                } else {
                                    Lucide::Pin
                                })
                                .size(px(12.)),
                            )
                            .tooltip("Pin / Unpin Selected Table")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.toggle_pin_selected_table(cx);
                            })),
                    )
                    .child(
                        Button::new("tbl-refresh")
                            .ghost()
                            .xsmall()
                            .loading(session.loading.objects())
                            .icon(Icon::new(Lucide::RefreshCw).size(px(12.)))
                            .tooltip("Refresh Tables")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.refresh_current_database_objects(window, cx);
                            })),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .pr(px(4.))
                            .text_size(px(11.))
                            .text_color(p.text_muted)
                            .child(format!("{} items", session.objects.len())),
                    ),
            )
            .into_any_element()
    }

    fn render_active_view(
        &self,
        session: &WorkspaceSession,
        p: Palette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match session.main_tab {
            MainViewTab::Structure => self.render_structure_view(session, p, cx),
            MainViewTab::Content => self.render_content_view(session, p, cx),
            MainViewTab::Relations => self.render_relations_view(session, p, cx),
            MainViewTab::Triggers => self.render_triggers_view(session, p, cx),
            MainViewTab::TableInfo => self.render_table_info_view(session, p, cx),
            MainViewTab::CustomQuery => self.render_query_view(session, p, cx),
        }
    }

    // ─── View 1: Structure ──────────────────────────────────────────────────

    fn render_structure_view(
        &self,
        session: &WorkspaceSession,
        p: Palette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if session.selected_table.is_none() {
            return empty_view("Select a table from the sidebar to inspect its structure", p);
        }
        let col_count = session
            .table_meta
            .as_ref()
            .map(|m| m.columns.len())
            .unwrap_or(0);
        let idx_count = session
            .table_meta
            .as_ref()
            .map(|m| m.indexes.len())
            .unwrap_or(0);

        let struct_loading = session.loading.structure();

        let columns_pane = v_flex()
            .size_full()
            .bg(p.window_bg)
            .child(section_header("Columns", Some(format!("{col_count}")), p))
            .child(
                div().flex_1().min_h_0().w_full().child(loading_pane(
                    "structure-cols-loading",
                    struct_loading,
                    col_count == 0,
                    "Loading structure…",
                    p,
                    DataTable::new(&session.structure_table)
                        .stripe(true)
                        .bordered(false)
                        .small(),
                )),
            )
            .child(
                h_flex()
                    .h(px(30.))
                    .w_full()
                    .px(px(8.))
                    .gap(px(4.))
                    .items_center()
                    .bg(p.toolbar_bg)
                    .border_t_1()
                    .border_color(p.border)
                    .child(
                        Button::new("str-add-col")
                            .ghost()
                            .xsmall()
                            .icon(Icon::new(Lucide::Plus).size(px(12.)))
                            .label("Add Column")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_add_column_modal(window, cx);
                            })),
                    )
                    .child(
                        Button::new("str-edit-col")
                            .ghost()
                            .xsmall()
                            .icon(Icon::new(Lucide::Pencil).size(px(12.)))
                            .label("Edit")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_edit_selected_column_modal(window, cx);
                            })),
                    )
                    .child(
                        Button::new("str-drop-col")
                            .ghost()
                            .xsmall()
                            .icon(Icon::new(Lucide::Minus).size(px(12.)))
                            .label("Drop")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.request_drop_selected_column(cx);
                            })),
                    )
                    .child(div().flex_1())
                    .child(
                        Button::new("str-refresh")
                            .ghost()
                            .xsmall()
                            .loading(struct_loading)
                            .icon(Icon::new(Lucide::RefreshCw).size(px(12.)))
                            .tooltip("Reload Table Structure")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.reload_selected_table(window, cx);
                            })),
                    ),
            );

        let indexes_pane = v_flex()
            .size_full()
            .bg(p.window_bg)
            .child(section_header("Indexes", Some(format!("{idx_count}")), p))
            .child(
                div().flex_1().min_h_0().w_full().child(loading_pane(
                    "structure-idx-loading",
                    struct_loading,
                    idx_count == 0,
                    "Loading indexes…",
                    p,
                    DataTable::new(&session.index_table)
                        .stripe(true)
                        .bordered(false)
                        .small(),
                )),
            )
            .child(
                h_flex()
                    .h(px(30.))
                    .w_full()
                    .px(px(8.))
                    .gap(px(4.))
                    .items_center()
                    .bg(p.toolbar_bg)
                    .border_t_1()
                    .border_color(p.border)
                    .child(
                        Button::new("idx-add")
                            .ghost()
                            .xsmall()
                            .icon(Icon::new(Lucide::Plus).size(px(12.)))
                            .label("Add Index")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_add_index_modal(window, cx);
                            })),
                    )
                    .child(
                        Button::new("idx-drop")
                            .ghost()
                            .xsmall()
                            .icon(Icon::new(Lucide::Minus).size(px(12.)))
                            .label("Drop")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.request_drop_selected_index(cx);
                            })),
                    ),
            );

        div()
            .size_full()
            .child(
                v_resizable("structure-split")
                    .child(resizable_panel().child(columns_pane))
                    .child(
                        resizable_panel()
                            .size(px(210.))
                            .size_range(px(110.)..px(420.))
                            .child(indexes_pane),
                    ),
            )
            .into_any_element()
    }

    // ─── View 2: Content ────────────────────────────────────────────────────

    fn render_content_view(
        &self,
        session: &WorkspaceSession,
        p: Palette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if session.selected_table.is_none() {
            return empty_view("Select a table from the sidebar to browse rows", p);
        }

        let offset = session.page_offset;
        let page = session.page_size;
        let loaded = session.content_table.read(cx).delegate().row_count();
        let total = session.row_count.unwrap_or(loaded as u64) as usize;
        let has_prev = offset > 0;
        let has_next = offset + loaded < total;
        let has_filter = session.where_clause.is_some();
        let content_loading = session.loading.content();

        let selected_cell_label = session.selected_cell.and_then(|(r, c)| {
            let col_name = session.content_columns.get(c)?;
            Some(format!("Row {} • {col_name}", offset + r + 1))
        });

        v_flex()
            .size_full()
            .bg(p.window_bg)
            // Sequel Ace Filter Bar: [Column] [Operator] [Value] [Filter] [Reset]
            .child(
                h_flex()
                    .h(px(36.))
                    .w_full()
                    .px(px(10.))
                    .gap(px(6.))
                    .items_center()
                    .bg(p.toolbar_bg)
                    .border_b_1()
                    .border_color(p.border)
                    .child(
                        Icon::new(Lucide::Funnel)
                            .size(px(12.))
                            .text_color(if has_filter { p.accent } else { p.text_muted }),
                    )
                    .child(
                        div().w(px(155.)).child(
                            Select::new(&session.filter_col_select)
                                .small()
                                .placeholder("Column / WHERE")
                                .menu_width(px(180.)),
                        ),
                    )
                    .child(
                        div().w(px(112.)).child(
                            Select::new(&session.filter_op_select)
                                .small()
                                .menu_width(px(130.)),
                        ),
                    )
                    .child(
                        div().flex_1().min_w(px(120.)).child(
                            Input::new(&session.content_filter_value)
                                .small()
                                .cleanable(true),
                        ),
                    )
                    .child(
                        Button::new("cnt-filter-apply")
                            .primary()
                            .xsmall()
                            .loading(content_loading)
                            .label("Filter")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.apply_content_filter(window, cx);
                            })),
                    )
                    .when(has_filter, |el| {
                        el.child(
                            Button::new("cnt-filter-reset")
                                .ghost()
                                .xsmall()
                                .icon(Icon::new(Lucide::RotateCcw).size(px(12.)))
                                .label("Reset")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.clear_content_filter(window, cx);
                                })),
                        )
                    }),
            )
            // Main Virtualized DataTable (scoped non-blocking loading indicator)
            .child(
                div().flex_1().min_h_0().w_full().child(loading_pane(
                    "content-rows-loading",
                    content_loading,
                    loaded == 0,
                    "Loading rows…",
                    p,
                    DataTable::new(&session.content_table)
                        .stripe(true)
                        .bordered(false)
                        .small(),
                )),
            )
            // Inline Cell Inspector / Editor bar (appears when a cell is selected — full width!)
            .when_some(selected_cell_label, |el, label| {
                el.child(
                    h_flex()
                        .h(px(34.))
                        .w_full()
                        .px(px(10.))
                        .gap(px(6.))
                        .items_center()
                        .bg(p.surface_subtle)
                        .border_t_1()
                        .border_color(p.border)
                        .child(
                            div()
                                .max_w(px(170.))
                                .overflow_hidden()
                                .text_ellipsis()
                                .whitespace_nowrap()
                                .text_size(px(11.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.text_secondary)
                                .child(label),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(140.))
                                .child(Input::new(&session.cell_edit_input).small()),
                        )
                        .child(
                            Button::new("cell-save")
                                .primary()
                                .xsmall()
                                .label("Save")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.save_inline_cell_edit(false, window, cx);
                                })),
                        )
                        .child(
                            Button::new("cell-null")
                                .outline()
                                .xsmall()
                                .label("NULL")
                                .tooltip("Set selected cell to SQL NULL")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.save_inline_cell_edit(true, window, cx);
                                })),
                        )
                        .child(
                            Button::new("cell-expand")
                                .ghost()
                                .xsmall()
                                .icon(Icon::new(Lucide::Maximize2).size(px(12.)))
                                .tooltip("Open Multi-line Field Editor (or double-click cell)")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.open_selected_cell_editor(window, cx);
                                })),
                        )
                        .child(
                            Button::new("cell-copy")
                                .ghost()
                                .xsmall()
                                .icon(Icon::new(Lucide::Copy).size(px(12.)))
                                .tooltip("Copy cell value")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.copy_selected_cell(cx);
                                })),
                        ),
                )
            })
            // Minimalist Sequel Ace Content Footer: Row ops + Pagination
            .child(
                h_flex()
                    .h(px(30.))
                    .w_full()
                    .px(px(8.))
                    .gap(px(4.))
                    .items_center()
                    .bg(p.toolbar_bg)
                    .border_t_1()
                    .border_color(p.border)
                    .child(
                        Button::new("cnt-add-row")
                            .ghost()
                            .xsmall()
                            .icon(Icon::new(Lucide::Plus).size(px(13.)))
                            .tooltip("Insert Row")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_insert_row_modal(window, cx);
                            })),
                    )
                    .child(
                        Button::new("cnt-del-row")
                            .ghost()
                            .xsmall()
                            .icon(Icon::new(Lucide::Minus).size(px(13.)))
                            .tooltip("Delete Selected Row")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.request_delete_selected_row(cx);
                            })),
                    )
                    .child(
                        Button::new("cnt-dup-row")
                            .ghost()
                            .xsmall()
                            .icon(Icon::new(Lucide::Copy).size(px(12.)))
                            .tooltip("Duplicate Selected Row")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.duplicate_selected_row(window, cx);
                            })),
                    )
                    .child(
                        Button::new("cnt-copy-insert")
                            .ghost()
                            .xsmall()
                            .label("Copy SQL")
                            .tooltip("Copy selected row as SQL INSERT statement")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.copy_selected_row_as_insert(cx);
                            })),
                    )
                    .child(
                        Button::new("cnt-refresh")
                            .ghost()
                            .xsmall()
                            .loading(content_loading)
                            .icon(Icon::new(Lucide::RefreshCw).size(px(12.)))
                            .tooltip("Reload Content")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.load_content_page(offset, window, cx);
                            })),
                    )
                    .child(div().flex_1())
                    // Page size selector
                    .child(
                        Button::new("cnt-limit")
                            .ghost()
                            .xsmall()
                            .label(format!("Limit: {page}"))
                            .tooltip("Cycle page size (100 / 500 / 1000)")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.cycle_page_size(window, cx);
                            })),
                    )
                    .child(div().w(px(1.)).h(px(14.)).bg(p.hairline))
                    // Pagination controls
                    .child(
                        Button::new("cnt-prev")
                            .ghost()
                            .xsmall()
                            .disabled(!has_prev || content_loading)
                            .icon(Icon::new(Lucide::ChevronLeft).size(px(13.)))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.load_content_page(offset.saturating_sub(page), window, cx);
                            })),
                    )
                    .child(
                        div()
                            .px(px(6.))
                            .text_size(px(11.))
                            .text_color(p.text_secondary)
                            .child(if loaded == 0 {
                                format!("0 of {total} rows")
                            } else {
                                format!("{}–{} of {total} rows", offset + 1, offset + loaded)
                            }),
                    )
                    .child(
                        Button::new("cnt-next")
                            .ghost()
                            .xsmall()
                            .disabled(!has_next || content_loading)
                            .icon(Icon::new(Lucide::ChevronRight).size(px(13.)))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.load_content_page(offset + page, window, cx);
                            })),
                    ),
            )
            .into_any_element()
    }

    // ─── View 3: Relations ──────────────────────────────────────────────────

    fn render_relations_view(
        &self,
        session: &WorkspaceSession,
        p: Palette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if session.selected_table.is_none() {
            return empty_view("Select a table from the sidebar to view foreign key relations", p);
        }
        let count = session
            .table_meta
            .as_ref()
            .map(|m| m.foreign_keys.len())
            .unwrap_or(0);
        let struct_loading = session.loading.structure();

        v_flex()
            .size_full()
            .bg(p.window_bg)
            .child(section_header("Foreign Key Relations", Some(format!("{count}")), p))
            .child(
                div().flex_1().min_h_0().w_full().child(loading_pane(
                    "relations-loading",
                    struct_loading,
                    count == 0,
                    "Loading relations…",
                    p,
                    DataTable::new(&session.relation_table)
                        .stripe(true)
                        .bordered(false)
                        .small(),
                )),
            )
            .child(
                h_flex()
                    .h(px(30.))
                    .w_full()
                    .px(px(8.))
                    .gap(px(4.))
                    .items_center()
                    .bg(p.toolbar_bg)
                    .border_t_1()
                    .border_color(p.border)
                    .child(
                        Button::new("rel-add")
                            .ghost()
                            .xsmall()
                            .icon(Icon::new(Lucide::Plus).size(px(12.)))
                            .label("Add Relation")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_add_foreign_key_modal(window, cx);
                            })),
                    )
                    .child(
                        Button::new("rel-drop")
                            .ghost()
                            .xsmall()
                            .icon(Icon::new(Lucide::Minus).size(px(12.)))
                            .label("Drop")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.request_drop_selected_relation(cx);
                            })),
                    )
                    .child(div().flex_1())
                    .child(
                        Button::new("rel-refresh")
                            .ghost()
                            .xsmall()
                            .loading(struct_loading)
                            .icon(Icon::new(Lucide::RefreshCw).size(px(12.)))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.reload_selected_table(window, cx);
                            })),
                    ),
            )
            .into_any_element()
    }

    // ─── View 4: Triggers ───────────────────────────────────────────────────

    fn render_triggers_view(
        &self,
        session: &WorkspaceSession,
        p: Palette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if session.selected_table.is_none() {
            return empty_view("Select a table from the sidebar to view triggers", p);
        }
        let selected_stmt = session
            .selected_trigger_row
            .and_then(|i| session.triggers.get(i))
            .map(|t| t.statement.clone())
            .unwrap_or_else(|| "Select a trigger above to view its full definition.".into());
        let struct_loading = session.loading.structure();

        let top = v_flex()
            .size_full()
            .bg(p.window_bg)
            .child(section_header(
                "Triggers",
                Some(format!("{}", session.triggers.len())),
                p,
            ))
            .child(
                div().flex_1().min_h_0().w_full().child(loading_pane(
                    "triggers-loading",
                    struct_loading,
                    session.triggers.is_empty(),
                    "Loading triggers…",
                    p,
                    DataTable::new(&session.trigger_table)
                        .stripe(true)
                        .bordered(false)
                        .small(),
                )),
            )
            .child(
                h_flex()
                    .h(px(30.))
                    .w_full()
                    .px(px(8.))
                    .gap(px(4.))
                    .items_center()
                    .bg(p.toolbar_bg)
                    .border_t_1()
                    .border_color(p.border)
                    .child(
                        Button::new("trg-add")
                            .ghost()
                            .xsmall()
                            .icon(Icon::new(Lucide::Plus).size(px(12.)))
                            .label("Add Trigger")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_add_trigger_modal(window, cx);
                            })),
                    )
                    .child(
                        Button::new("trg-drop")
                            .ghost()
                            .xsmall()
                            .icon(Icon::new(Lucide::Minus).size(px(12.)))
                            .label("Drop")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.request_drop_selected_trigger(cx);
                            })),
                    )
                    .child(div().flex_1())
                    .child(
                        Button::new("trg-refresh")
                            .ghost()
                            .xsmall()
                            .loading(struct_loading)
                            .icon(Icon::new(Lucide::RefreshCw).size(px(12.)))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.reload_selected_table(window, cx);
                            })),
                    ),
            );

        let preview = v_flex()
            .size_full()
            .bg(p.window_bg)
            .child(section_header("Trigger Statement", None, p))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .p(px(12.))
                    .bg(p.code_bg)
                    .overflow_y_scrollbar()
                    .child(
                        div()
                            .text_xs()
                            .font_family("Menlo")
                            .text_color(p.text)
                            .child(selected_stmt),
                    ),
            );

        div()
            .size_full()
            .child(
                v_resizable("triggers-split")
                    .child(resizable_panel().child(top))
                    .child(
                        resizable_panel()
                            .size(px(170.))
                            .size_range(px(90.)..px(360.))
                            .child(preview),
                    ),
            )
            .into_any_element()
    }

    // ─── View 5: Table Info ─────────────────────────────────────────────────

    fn render_table_info_view(
        &self,
        session: &WorkspaceSession,
        p: Palette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let struct_loading = session.loading.structure();
        let Some(meta) = &session.table_meta else {
            if session.selected_table.is_some() && struct_loading {
                return loading_pane_empty("table-info-loading", "Loading table info…", p);
            }
            return empty_view("Select a table from the sidebar to view table info", p);
        };
        let is_sqlite = session.info.engine == ace_core::EngineKind::Sqlite;
        let create_sql = meta
            .create_sql
            .clone()
            .unwrap_or_else(|| "—".into());
        let create_sql_copy = create_sql.clone();
        let row_count_text = session
            .row_count
            .map(|n| n.to_string())
            .unwrap_or_else(|| "—".into());

        let content = div()
            .size_full()
            .bg(p.window_bg)
            .overflow_y_scrollbar()
            .child(
                v_flex()
                    .w_full()
                    .max_w(px(860.))
                    .p(px(20.))
                    .gap(px(16.))
                    // Header + quick maintenance actions
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .justify_between()
                            .pb(px(10.))
                            .border_b_1()
                            .border_color(p.hairline)
                            .child(
                                v_flex()
                                    .gap(px(2.))
                                    .child(
                                        div()
                                            .text_base()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(p.text)
                                            .child(meta.name.clone()),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(p.text_muted)
                                            .child(format!(
                                                "{}.{}",
                                                meta.database, meta.name
                                            )),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .gap(px(6.))
                                    .child(
                                        Button::new("info-rename")
                                            .outline()
                                            .xsmall()
                                            .label("Rename…")
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.open_rename_table_modal(window, cx);
                                            })),
                                    )
                                    .when(!is_sqlite, |el| {
                                        el.child(
                                            Button::new("info-optimize")
                                                .outline()
                                                .xsmall()
                                                .label("Optimize")
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    this.run_table_maintenance("OPTIMIZE TABLE", window, cx);
                                                })),
                                        )
                                        .child(
                                            Button::new("info-analyze")
                                                .outline()
                                                .xsmall()
                                                .label("Analyze")
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    this.run_table_maintenance("ANALYZE TABLE", window, cx);
                                                })),
                                        )
                                        .child(
                                            Button::new("info-check")
                                                .outline()
                                                .xsmall()
                                                .label("Check")
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    this.run_table_maintenance("CHECK TABLE", window, cx);
                                                })),
                                        )
                                    })
                                    .child(
                                        Button::new("info-truncate")
                                            .danger()
                                            .xsmall()
                                            .label("Truncate…")
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.request_truncate_selected_table(cx);
                                            })),
                                    ),
                            ),
                    )
                    // Metadata card
                    .child(
                        v_flex()
                            .w_full()
                            .rounded(px(8.))
                            .bg(p.card_bg)
                            .border_1()
                            .border_color(p.border)
                            .p(px(14.))
                            .gap(px(8.))
                            .child(info_kv("Engine", meta.engine.as_deref().unwrap_or("—"), p))
                            .child(info_kv("Encoding", meta.charset.as_deref().unwrap_or("—"), p))
                            .child(info_kv("Collation", meta.collation.as_deref().unwrap_or("—"), p))
                            .child(info_kv("Row Count", &row_count_text, p))
                            .child(info_kv(
                                "Columns",
                                &meta.columns.len().to_string(),
                                p,
                            ))
                            .child(info_kv(
                                "Indexes",
                                &meta.indexes.len().to_string(),
                                p,
                            ))
                            .child(info_kv(
                                "Auto Increment",
                                &meta
                                    .auto_increment
                                    .map(|n| n.to_string())
                                    .unwrap_or_else(|| "—".into()),
                                p,
                            ))
                            .when(!is_sqlite, |el| {
                                el.child(
                                    h_flex()
                                        .w_full()
                                        .pt(px(4.))
                                        .gap(px(8.))
                                        .items_center()
                                        .child(
                                            div()
                                                .w(px(120.))
                                                .text_xs()
                                                .text_color(p.text_muted)
                                                .child("Comment"),
                                        )
                                        .child(
                                            div()
                                                .flex_1()
                                                .child(Input::new(&session.table_comment_input).small()),
                                        )
                                        .child(
                                            Button::new("info-save-comment")
                                                .outline()
                                                .xsmall()
                                                .label("Save")
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    this.save_table_comment(window, cx);
                                                })),
                                        ),
                                )
                            }),
                    )
                    // CREATE TABLE syntax box
                    .child(
                        v_flex()
                            .w_full()
                            .gap(px(6.))
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.text_secondary)
                                            .child("Table Create Syntax"),
                                    )
                                    .child(
                                        Button::new("info-copy-ddl")
                                            .ghost()
                                            .xsmall()
                                            .icon(Icon::new(Lucide::Copy).size(px(12.)))
                                            .label("Copy Syntax")
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                cx.write_to_clipboard(ClipboardItem::new_string(
                                                    create_sql_copy.clone(),
                                                ));
                                                this.set_session_status(
                                                    "Copied CREATE syntax to clipboard",
                                                    cx,
                                                );
                                            })),
                                    ),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .p(px(14.))
                                    .rounded(px(8.))
                                    .bg(p.code_bg)
                                    .border_1()
                                    .border_color(p.border)
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_family("Menlo")
                                            .text_color(p.text)
                                            .child(create_sql),
                                    ),
                            ),
                    ),
            );

        loading_pane(
            "table-info-refresh",
            struct_loading,
            false,
            "Refreshing table info…",
            p,
            content,
        )
    }

    // ─── View 6: Custom Query ───────────────────────────────────────────────

    fn render_query_view(
        &self,
        session: &WorkspaceSession,
        p: Palette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let drawer_open = session.query_drawer_open;
        let drawer_tab = session.query_drawer;
        let result_count = session.query_results.len();
        let active_res = session.active_result_idx;
        let query_error = session.query_error.clone();
        let query_msg = session.query_message.clone();
        let query_loading = session.loading.query();

        let editor_pane = v_flex()
            .size_full()
            .bg(p.window_bg)
            // Query Toolbar
            .child(
                h_flex()
                    .h(px(34.))
                    .w_full()
                    .px(px(8.))
                    .gap(px(4.))
                    .items_center()
                    .bg(p.toolbar_bg)
                    .border_b_1()
                    .border_color(p.border)
                    .child(
                        Button::new("qry-run-sel")
                            .primary()
                            .xsmall()
                            .loading(query_loading)
                            .icon(Icon::new(Lucide::Play).size(px(11.)))
                            .label("Run Current")
                            .tooltip(if cfg!(target_os = "macos") {
                                format!(
                                    "Run selected SQL or statement at cursor ({} / {})",
                                    mod_shortcut("Enter"),
                                    mod_shortcut("R"),
                                )
                            } else {
                                format!(
                                    "Run selected SQL or statement at cursor ({} / {} / F5)",
                                    mod_shortcut("Enter"),
                                    mod_shortcut("R"),
                                )
                            })
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.run_custom_query(true, window, cx);
                            })),
                    )
                    .child(
                        Button::new("qry-run-all")
                            .outline()
                            .xsmall()
                            .disabled(query_loading)
                            .icon(Icon::new(Lucide::CirclePlay).size(px(12.)))
                            .label("Run All")
                            .tooltip(format!(
                                "Run all queries in editor ({} / {})",
                                mod_shortcut("Shift+R"),
                                mod_shortcut("Shift+Enter"),
                            ))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.run_custom_query(false, window, cx);
                            })),
                    )
                    .child(div().w(px(1.)).h(px(16.)).bg(p.hairline))
                    .child(
                        Button::new("qry-format")
                            .ghost()
                            .xsmall()
                            .icon(Icon::new(Lucide::Sparkles).size(px(12.)))
                            .label("Format")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.format_query_editor(window, cx);
                            })),
                    )
                    .child(
                        Button::new("qry-explain")
                            .ghost()
                            .xsmall()
                            .label("EXPLAIN")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.explain_query(window, cx);
                            })),
                    )
                    .child(
                        Button::new("qry-save-fav")
                            .ghost()
                            .xsmall()
                            .icon(Icon::new(Lucide::Bookmark).size(px(12.)))
                            .label("Save")
                            .tooltip("Save query to Query Favorites")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_save_query_favorite_modal(window, cx);
                            })),
                    )
                    .child(
                        Button::new("qry-clear")
                            .ghost()
                            .xsmall()
                            .icon(Icon::new(Lucide::Trash).size(px(12.)))
                            .tooltip("Clear Editor")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.clear_query_editor(window, cx);
                            })),
                    )
                    .child(div().flex_1())
                    .child(if query_loading {
                        loading_inline("Running query…", p)
                    } else {
                        div()
                            .text_size(px(11.))
                            .text_color(if query_error.is_some() {
                                p.danger
                            } else {
                                p.text_muted
                            })
                            .child(query_msg)
                            .into_any_element()
                    })
                    .child(
                        Button::new("qry-drawer-toggle")
                            .ghost()
                            .xsmall()
                            .toggled(drawer_open)
                            .icon(Icon::new(Lucide::Clock).size(px(12.)))
                            .label("History")
                            .on_click(cx.listener(|this, _, _, cx| {
                                if let Some(s) = this.active_tab_mut().session.as_mut() {
                                    s.query_drawer_open = !s.query_drawer_open;
                                }
                                cx.notify();
                            })),
                    ),
            )
            // Multi-line SQL Code Editor with syntax highlighting & line numbers
            .child(
                div().flex_1().min_h_0().w_full().child(
                    Editor::new(&session.query_editor)
                        .h_full()
                        .bordered(false)
                        .small(),
                ),
            );

        let result_pane = v_flex()
            .size_full()
            .bg(p.window_bg)
            // Multiple result set switcher if >1 result set
            .when(result_count > 1, |el| {
                el.child(
                    h_flex()
                        .h(px(26.))
                        .w_full()
                        .px(px(8.))
                        .gap(px(4.))
                        .items_center()
                        .bg(p.toolbar_bg)
                        .border_b_1()
                        .border_color(p.border)
                        .children((0..result_count).map(|i| {
                            let is_act = i == active_res;
                            let rows_n = session.query_results[i].rows.len();
                            div()
                                .id(SharedString::from(format!("qres-tab-{i}")))
                                .px(px(8.))
                                .py(px(2.))
                                .rounded(px(4.))
                                .cursor_pointer()
                                .text_size(px(11.))
                                .when(is_act, |b| {
                                    b.bg(p.accent_subtle)
                                        .text_color(p.accent)
                                        .font_weight(FontWeight::MEDIUM)
                                })
                                .when(!is_act, |b| {
                                    b.text_color(p.text_muted).hover(|h| h.bg(p.hover_bg))
                                })
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.select_query_result_set(i, cx);
                                }))
                                .child(format!("Result {} ({rows_n})", i + 1))
                        })),
                )
            })
            .child(if let Some(err) = query_error {
                v_flex()
                    .flex_1()
                    .p(px(16.))
                    .gap(px(8.))
                    .child(
                        h_flex()
                            .p(px(12.))
                            .gap(px(8.))
                            .items_start()
                            .rounded(px(8.))
                            .bg(p.danger_subtle)
                            .border_1()
                            .border_color(p.danger)
                            .child(
                                Icon::new(Lucide::CircleAlert)
                                    .size(px(15.))
                                    .text_color(p.danger),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .text_xs()
                                    .font_family("Menlo")
                                    .text_color(p.danger)
                                    .child(err),
                            ),
                    )
                    .into_any_element()
            } else {
                div().flex_1().min_h_0().w_full().child(loading_pane(
                    "query-result-loading",
                    query_loading,
                    result_count == 0,
                    "Running query…",
                    p,
                    DataTable::new(&session.query_result_table)
                        .stripe(true)
                        .bordered(false)
                        .small(),
                ))
                .into_any_element()
            });

        let main_query_split = v_resizable("query-editor-split")
            .child(
                resizable_panel()
                    .size(px(230.))
                    .size_range(px(100.)..px(520.))
                    .child(editor_pane),
            )
            .child(resizable_panel().child(result_pane));

        if !drawer_open {
            return div().size_full().child(main_query_split).into_any_element();
        }

        let drawer = v_flex()
            .size_full()
            .bg(p.sidebar_bg)
            // Drawer tabs: History | Favorites
            .child(
                h_flex()
                    .h(px(34.))
                    .w_full()
                    .px(px(8.))
                    .gap(px(4.))
                    .items_center()
                    .bg(p.toolbar_bg)
                    .border_b_1()
                    .border_color(p.border)
                    .child(
                        Button::new("qdr-hist")
                            .ghost()
                            .xsmall()
                            .toggled(drawer_tab == QueryDrawerTab::History)
                            .label("History")
                            .on_click(cx.listener(|this, _, _, cx| {
                                if let Some(s) = this.active_tab_mut().session.as_mut() {
                                    s.query_drawer = QueryDrawerTab::History;
                                }
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("qdr-favs")
                            .ghost()
                            .xsmall()
                            .toggled(drawer_tab == QueryDrawerTab::Favorites)
                            .label("Favorites")
                            .on_click(cx.listener(|this, _, _, cx| {
                                if let Some(s) = this.active_tab_mut().session.as_mut() {
                                    s.query_drawer = QueryDrawerTab::Favorites;
                                }
                                cx.notify();
                            })),
                    )
                    .child(div().flex_1())
                    .when(drawer_tab == QueryDrawerTab::History && !self.query_history.is_empty(), |el| {
                        el.child(
                            Button::new("qdr-clear-hist")
                                .ghost()
                                .xsmall()
                                .icon(Icon::new(Lucide::Trash).size(px(11.)))
                                .tooltip("Clear Query History")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.clear_query_history(cx);
                                })),
                        )
                    }),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .overflow_y_scrollbar()
                    .child(v_flex().w_full().p(px(6.)).gap(px(4.)).children(
                        match drawer_tab {
                            QueryDrawerTab::History => self
                                .query_history
                                .iter()
                                .take(50)
                                .map(|entry| {
                                    let sql = entry.sql.clone();
                                    let preview = one_line_trim(&sql, 80);
                                    let ok = entry.success;
                                    let ms = entry.duration_ms;
                                    let entry_id = entry.id;
                                    v_flex()
                                        .id(SharedString::from(format!("qhist-{}", entry.id)))
                                        .w_full()
                                        .p(px(7.))
                                        .gap(px(2.))
                                        .rounded(px(5.))
                                        .bg(p.card_bg)
                                        .border_1()
                                        .border_color(p.hairline)
                                        .cursor_pointer()
                                        .hover(|h| h.border_color(p.accent))
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.load_sql_into_editor(sql.clone(), window, cx);
                                        }))
                                        .child(
                                            h_flex()
                                                .w_full()
                                                .gap(px(5.))
                                                .items_center()
                                                .child(
                                                    Icon::new(if ok {
                                                        Lucide::CircleCheck
                                                    } else {
                                                        Lucide::CircleX
                                                    })
                                                    .size(px(11.))
                                                    .text_color(if ok {
                                                        p.success
                                                    } else {
                                                        p.danger
                                                    }),
                                                )
                                                .child(
                                                    div()
                                                        .flex_1()
                                                        .text_size(px(10.))
                                                        .text_color(p.text_muted)
                                                        .child(format!("{ms} ms")),
                                                )
                                                .child(
                                                    div()
                                                        .id(SharedString::from(format!(
                                                            "qhist-del-{entry_id}"
                                                        )))
                                                        .p(px(2.))
                                                        .rounded(px(3.))
                                                        .hover(|h| h.bg(p.hover_bg))
                                                        .on_click(cx.listener(
                                                            move |this, _, _, cx| {
                                                                cx.stop_propagation();
                                                                this.delete_query_history_entry(
                                                                    entry_id, cx,
                                                                );
                                                            },
                                                        ))
                                                        .child(
                                                            Icon::new(Lucide::Trash)
                                                                .size(px(10.))
                                                                .text_color(p.text_muted),
                                                        ),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(11.))
                                                .font_family("Menlo")
                                                .text_color(p.text)
                                                .overflow_hidden()
                                                .text_ellipsis()
                                                .whitespace_nowrap()
                                                .child(preview),
                                        )
                                        .into_any_element()
                                })
                                .collect::<Vec<_>>(),
                            QueryDrawerTab::Favorites => self
                                .query_favorites
                                .iter()
                                .map(|fav| {
                                    let sql = fav.sql.clone();
                                    let fav_id = fav.id;
                                    let preview = one_line_trim(&sql, 70);
                                    v_flex()
                                        .id(SharedString::from(format!("qfav-{}", fav.id)))
                                        .w_full()
                                        .p(px(7.))
                                        .gap(px(2.))
                                        .rounded(px(5.))
                                        .bg(p.card_bg)
                                        .border_1()
                                        .border_color(p.hairline)
                                        .cursor_pointer()
                                        .hover(|h| h.border_color(p.accent))
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.load_sql_into_editor(sql.clone(), window, cx);
                                        }))
                                        .child(
                                            h_flex()
                                                .w_full()
                                                .items_center()
                                                .justify_between()
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .font_weight(FontWeight::MEDIUM)
                                                        .text_color(p.text)
                                                        .child(fav.name.clone()),
                                                )
                                                .child(
                                                    div()
                                                        .id(SharedString::from(format!(
                                                            "qfav-del-{fav_id}"
                                                        )))
                                                        .p(px(2.))
                                                        .rounded(px(3.))
                                                        .hover(|h| h.bg(p.hover_bg))
                                                        .on_click(cx.listener(
                                                            move |this, _, _, cx| {
                                                                cx.stop_propagation();
                                                                this.delete_query_favorite(
                                                                    fav_id, cx,
                                                                );
                                                            },
                                                        ))
                                                        .child(
                                                            Icon::new(Lucide::Trash)
                                                                .size(px(11.))
                                                                .text_color(p.text_muted),
                                                        ),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(11.))
                                                .font_family("Menlo")
                                                .text_color(p.text_muted)
                                                .overflow_hidden()
                                                .text_ellipsis()
                                                .whitespace_nowrap()
                                                .child(preview),
                                        )
                                        .into_any_element()
                                })
                                .collect::<Vec<_>>(),
                        },
                    )),
            );

        div()
            .size_full()
            .child(
                h_resizable("query-drawer-split")
                    .child(resizable_panel().child(main_query_split))
                    .child(
                        resizable_panel()
                            .size(px(250.))
                            .size_range(px(180.)..px(420.))
                            .child(drawer),
                    ),
            )
            .into_any_element()
    }

    // ─── Bottom Collapsible SQL Console (Sequel Ace Signature Feature) ──────

    fn render_console_drawer(
        &self,
        session: &WorkspaceSession,
        p: Palette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let filter = session
            .console_filter
            .read(cx)
            .value()
            .to_string()
            .to_lowercase();
        let entries = session.console.snapshot();
        let filtered: Vec<_> = entries
            .into_iter()
            .rev()
            .filter(|e| {
                filter.is_empty()
                    || e.sql.to_lowercase().contains(&filter)
                    || e.database.to_lowercase().contains(&filter)
                    || e.error
                        .as_deref()
                        .map(|er| er.to_lowercase().contains(&filter))
                        .unwrap_or(false)
            })
            .collect();

        v_flex()
            .size_full()
            .bg(p.code_bg)
            .child(
                h_flex()
                    .h(px(30.))
                    .w_full()
                    .px(px(8.))
                    .gap(px(6.))
                    .items_center()
                    .bg(p.toolbar_bg)
                    .border_b_1()
                    .border_color(p.border)
                    .child(
                        Icon::new(Lucide::SquareTerminal)
                            .size(px(12.))
                            .text_color(p.text_muted),
                    )
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.text_secondary)
                            .child(format!("SQL Console ({})", filtered.len())),
                    )
                    .child(
                        div()
                            .w(px(200.))
                            .child(Input::new(&session.console_filter).xsmall().cleanable(true)),
                    )
                    .child(div().flex_1())
                    .child(
                        Button::new("con-copy")
                            .ghost()
                            .xsmall()
                            .icon(Icon::new(Lucide::Copy).size(px(12.)))
                            .label("Copy")
                            .on_click(cx.listener(|this, _, _, cx| {
                                if let Some(s) = this.active_tab().session.as_ref() {
                                    cx.write_to_clipboard(ClipboardItem::new_string(
                                        s.console.as_text(),
                                    ));
                                }
                            })),
                    )
                    .child(
                        Button::new("con-clear")
                            .ghost()
                            .xsmall()
                            .icon(Icon::new(Lucide::Trash).size(px(12.)))
                            .label("Clear")
                            .on_click(cx.listener(|this, _, _, cx| {
                                if let Some(s) = this.active_tab().session.as_ref() {
                                    s.console.clear();
                                }
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("con-close")
                            .ghost()
                            .xsmall()
                            .icon(Icon::new(Lucide::X).size(px(12.)))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.toggle_console(cx);
                            })),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .overflow_y_scrollbar()
                    .child(
                        v_flex()
                            .w_full()
                            .p(px(8.))
                            .gap(px(4.))
                            .children(filtered.into_iter().map(|entry| {
                                let ts = entry.timestamp.format("%H:%M:%S").to_string();
                                let has_err = entry.error.is_some();
                                let sql_click = entry.sql.clone();
                                v_flex()
                                    .id(SharedString::from(format!("con-row-{}", entry.id)))
                                    .w_full()
                                    .px(px(8.))
                                    .py(px(5.))
                                    .gap(px(2.))
                                    .rounded(px(4.))
                                    .when(has_err, |el| el.bg(p.danger_subtle))
                                    .hover(|h| h.bg(p.hover_bg))
                                    .cursor_pointer()
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.load_sql_into_editor(sql_click.clone(), window, cx);
                                        this.set_main_tab(MainViewTab::CustomQuery, cx);
                                    }))
                                    .child(
                                        h_flex()
                                            .gap(px(8.))
                                            .items_center()
                                            .child(
                                                div()
                                                    .text_size(px(10.))
                                                    .text_color(p.text_disabled)
                                                    .child(ts),
                                            )
                                            .when(!entry.database.is_empty(), |el| {
                                                el.child(
                                                    div()
                                                        .text_size(px(10.))
                                                        .text_color(p.accent)
                                                        .child(entry.database),
                                                )
                                            })
                                            .child(
                                                div()
                                                    .text_size(px(10.))
                                                    .text_color(p.text_muted)
                                                    .child(format!("{} ms", entry.duration_ms)),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.))
                                            .font_family("Menlo")
                                            .text_color(p.text)
                                            .child(entry.sql),
                                    )
                                    .when_some(entry.error, |el, err| {
                                        el.child(
                                            div()
                                                .text_size(px(11.))
                                                .text_color(p.danger)
                                                .child(err),
                                        )
                                    })
                            })),
                    ),
            )
            .into_any_element()
    }
}

fn tab_icon(tab: MainViewTab) -> Lucide {
    match tab {
        MainViewTab::Structure => Lucide::Columns3,
        MainViewTab::Content => Lucide::Table,
        MainViewTab::Relations => Lucide::Link2,
        MainViewTab::Triggers => Lucide::Zap,
        MainViewTab::TableInfo => Lucide::Info,
        MainViewTab::CustomQuery => Lucide::Terminal,
    }
}

fn section_header(title: &'static str, count: Option<String>, p: Palette) -> AnyElement {
    h_flex()
        .h(px(28.))
        .w_full()
        .px(px(10.))
        .gap(px(6.))
        .items_center()
        .bg(p.header_bg)
        .border_b_1()
        .border_color(p.border)
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.text_secondary)
                .child(title),
        )
        .when_some(count, |el, c| {
            el.child(
                div()
                    .px(px(5.))
                    .py(px(1.))
                    .rounded(px(4.))
                    .bg(p.surface_subtle)
                    .text_size(px(10.))
                    .text_color(p.text_muted)
                    .child(c),
            )
        })
        .into_any_element()
}

fn empty_view(msg: &'static str, p: Palette) -> AnyElement {
    v_flex()
        .size_full()
        .items_center()
        .justify_center()
        .gap(px(8.))
        .bg(p.window_bg)
        .child(
            Icon::new(Lucide::Table2)
                .size(px(24.))
                .text_color(p.text_disabled),
        )
        .child(div().text_xs().text_color(p.text_muted).child(msg))
        .into_any_element()
}

fn info_kv(k: &'static str, v: &str, p: Palette) -> AnyElement {
    h_flex()
        .w_full()
        .gap(px(8.))
        .items_center()
        .child(
            div()
                .w(px(120.))
                .text_xs()
                .text_color(p.text_muted)
                .child(k),
        )
        .child(
            div()
                .flex_1()
                .text_xs()
                .text_color(p.text)
                .child(v.to_string()),
        )
        .into_any_element()
}

fn format_compact_count(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn one_line_trim(s: &str, max: usize) -> String {
    let flat: String = s
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if flat.chars().count() > max {
        format!("{}…", flat.chars().take(max).collect::<String>())
    } else {
        flat
    }
}
