//! Sequel Ace Connection screen: hierarchical Favorites sidebar + minimalist Connection form.

use crate::gui::loading::{loading_bar, loading_spinner};
use crate::gui::root::AceApp;
use crate::gui::runtime::demo_connection;
use crate::gui::theme::{favorite_tint, Palette};
use ace_core::{
    favorite_color_count, favorite_color_rgb, ConnectionInfo, ConnectionMethod, EngineKind,
    FavoriteNode, TimeZoneMode,
};
use gpui_kit::assets::IconName as Lucide;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::input::Input;
use gpui_kit::component::resizable::{h_resizable, resizable_panel};
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::{h_flex, v_flex, Icon, Sizable as _, Size};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

impl AceApp {
    pub(crate) fn render_connection_screen(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let p = Palette::for_app(cx);
        let sidebar = self.render_favorites_sidebar(p, cx);
        let form = self.render_connection_form(p, cx);

        div()
            .flex_1()
            .min_h_0()
            .w_full()
            .bg(p.window_bg)
            .child(
                h_resizable("conn-split")
                    .child(
                        resizable_panel()
                            .size(px(230.))
                            .size_range(px(175.)..px(380.))
                            .child(sidebar),
                    )
                    .child(resizable_panel().child(form)),
            )
            .into_any_element()
    }

    fn render_favorites_sidebar(&self, p: Palette, cx: &mut Context<Self>) -> AnyElement {
        let draft = &self.active_tab().draft;
        let search = draft.inputs.favorites_search.read(cx).value().to_string();
        let search_trimmed = search.trim();
        let selected_fav = draft.selected_favorite;
        let selected_group = draft.selected_group;

        let mut list_items: Vec<AnyElement> = Vec::new();

        // Quick Connect row
        let qc_selected = selected_fav.is_none() && selected_group.is_none();
        list_items.push(
            h_flex()
                .id("fav-quick-connect")
                .h(px(28.))
                .w_full()
                .px(px(10.))
                .gap(px(8.))
                .items_center()
                .rounded(px(5.))
                .cursor_pointer()
                .when(qc_selected, |el| el.bg(p.selected_bg))
                .when(!qc_selected, |el| el.hover(|h| h.bg(p.hover_bg)))
                .on_click(cx.listener(|this, _, window, cx| {
                    this.quick_connect_blank(window, cx);
                }))
                .child(
                    Icon::new(Lucide::Zap)
                        .size(px(13.))
                        .text_color(if qc_selected { p.accent } else { p.text_muted }),
                )
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.text)
                        .child("Quick Connect"),
                )
                .into_any_element(),
        );

        // Separator hairline
        list_items.push(
            div()
                .w_full()
                .h(px(1.))
                .my(px(4.))
                .bg(p.hairline)
                .into_any_element(),
        );

        if search_trimmed.is_empty() {
            // Hierarchical tree with collapsible groups
            for node in &self.favorites.roots {
                self.push_favorite_node(
                    node,
                    0,
                    selected_fav,
                    selected_group,
                    p,
                    cx,
                    &mut list_items,
                );
            }
        } else {
            // Filtered flat search results
            let matches = self.favorites.search(search_trimmed);
            if matches.is_empty() {
                list_items.push(
                    div()
                        .px(px(10.))
                        .py(px(12.))
                        .text_xs()
                        .text_color(p.text_muted)
                        .child("No matching favorites")
                        .into_any_element(),
                );
            } else {
                for fav in matches {
                    list_items.push(self.render_favorite_leaf(fav, 0, selected_fav, p, cx));
                }
            }
        }

        v_flex()
            .size_full()
            .bg(p.sidebar_bg)
            // Search header
            .child(
                div()
                    .px(px(10.))
                    .pt(px(10.))
                    .pb(px(8.))
                    .child(
                        Input::new(&draft.inputs.favorites_search)
                            .small()
                            .cleanable(true)
                            .prefix(
                                Icon::new(Lucide::Search)
                                    .size(px(13.))
                                    .text_color(p.text_muted),
                            ),
                    ),
            )
            // Scrollable favorites list
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .overflow_y_scrollbar()
                    .child(
                        v_flex()
                            .w_full()
                            .px(px(6.))
                            .pb(px(8.))
                            .gap(px(2.))
                            .children(list_items),
                    ),
            )
            // Minimalist Sequel Ace bottom footer bar
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
                        Button::new("fav-add")
                            .ghost()
                            .xsmall()
                            .icon(Icon::new(Lucide::Plus).size(px(13.)))
                            .tooltip("Save current connection to Favorites")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.save_current_favorite(cx);
                            })),
                    )
                    .child(
                        Button::new("fav-add-group")
                            .ghost()
                            .xsmall()
                            .icon(Icon::new(Lucide::FolderPlus).size(px(13.)))
                            .tooltip("Add Favorites Group")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_new_group_modal(window, cx);
                            })),
                    )
                    .child(
                        Button::new("fav-dup")
                            .ghost()
                            .xsmall()
                            .icon(Icon::new(Lucide::Copy).size(px(12.)))
                            .tooltip("Duplicate selected favorite")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.duplicate_selected_favorite(window, cx);
                            })),
                    )
                    .child(
                        Button::new("fav-del")
                            .ghost()
                            .xsmall()
                            .icon(Icon::new(Lucide::Minus).size(px(13.)))
                            .tooltip("Remove selected favorite or group")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.request_delete_selected_favorite(cx);
                            })),
                    )
                    .child(div().flex_1())
                    .child(
                        Button::new("fav-plist")
                            .ghost()
                            .xsmall()
                            .icon(Icon::new(Lucide::Download).size(px(12.)))
                            .tooltip("Import / Export Sequel Ace Favorites (.plist)")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_favorites_plist_modal(window, cx);
                            })),
                    )
                    .child(
                        Button::new("fav-demo")
                            .ghost()
                            .xsmall()
                            .label("Demo")
                            .tooltip("Open bundled demo SQLite database")
                            .on_click(cx.listener(|this, _, window, cx| {
                                let demo = demo_connection();
                                this.load_favorite(&demo, window, cx);
                                this.connect_active_tab(window, cx);
                            })),
                    ),
            )
            .into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn push_favorite_node(
        &self,
        node: &FavoriteNode,
        depth: usize,
        selected_fav: Option<uuid::Uuid>,
        selected_group: Option<uuid::Uuid>,
        p: Palette,
        cx: &mut Context<Self>,
        out: &mut Vec<AnyElement>,
    ) {
        match node {
            FavoriteNode::Group {
                id,
                name,
                children,
                expanded,
            } => {
                let group_id = *id;
                let is_expanded = *expanded;
                let is_selected = selected_group == Some(group_id);
                let indent = px((depth as f32) * 14.0 + 6.0);

                out.push(
                    h_flex()
                        .id(SharedString::from(format!("fav-grp-{group_id}")))
                        .h(px(26.))
                        .w_full()
                        .pl(indent)
                        .pr(px(8.))
                        .gap(px(5.))
                        .items_center()
                        .rounded(px(5.))
                        .cursor_pointer()
                        .when(is_selected, |el| el.bg(p.selected_bg))
                        .when(!is_selected, |el| el.hover(|h| h.bg(p.hover_bg)))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.toggle_favorite_group(group_id, cx);
                        }))
                        .child(
                            Icon::new(if is_expanded {
                                Lucide::ChevronDown
                            } else {
                                Lucide::ChevronRight
                            })
                            .size(px(12.))
                            .text_color(p.text_muted),
                        )
                        .child(
                            Icon::new(if is_expanded {
                                Lucide::FolderOpen
                            } else {
                                Lucide::Folder
                            })
                            .size(px(13.))
                            .text_color(p.text_muted),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .overflow_hidden()
                                .text_ellipsis()
                                .whitespace_nowrap()
                                .text_xs()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.text_secondary)
                                .child(name.clone()),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(p.text_disabled)
                                .child(format!("{}", children.len())),
                        )
                        .into_any_element(),
                );

                if is_expanded {
                    for child in children {
                        self.push_favorite_node(
                            child,
                            depth + 1,
                            selected_fav,
                            selected_group,
                            p,
                            cx,
                            out,
                        );
                    }
                }
            }
            FavoriteNode::Connection { info } => {
                out.push(self.render_favorite_leaf(info, depth, selected_fav, p, cx));
            }
        }
    }

    fn render_favorite_leaf(
        &self,
        info: &ConnectionInfo,
        depth: usize,
        selected_fav: Option<uuid::Uuid>,
        p: Palette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let info_clone = info.clone();
        let selected = selected_fav == Some(info.id);
        let name = if info.name.trim().is_empty() {
            info.display_host().to_string()
        } else {
            info.name.clone()
        };
        let indent = px((depth as f32) * 14.0 + 10.0);
        let tint = favorite_tint(info.color_index);
        let badge = match info.engine {
            EngineKind::Mysql => None,
            EngineKind::MariaDb => Some("MariaDB"),
            EngineKind::Postgres => Some("PG"),
            EngineKind::Sqlite => Some("SQLite"),
        };

        h_flex()
            .id(SharedString::from(format!("fav-item-{}", info.id)))
            .h(px(27.))
            .w_full()
            .pl(indent)
            .pr(px(8.))
            .gap(px(7.))
            .items_center()
            .rounded(px(5.))
            .cursor_pointer()
            .when(selected, |el| el.bg(p.selected_bg))
            .when(!selected, |el| el.hover(|h| h.bg(p.hover_bg)))
            .on_click(cx.listener(move |this, ev: &ClickEvent, window, cx| {
                this.load_favorite(&info_clone, window, cx);
                if ev.click_count() >= 2 {
                    this.connect_active_tab(window, cx);
                }
            }))
            .child(match tint {
                Some(c) => div()
                    .size(px(8.))
                    .rounded(px(4.))
                    .bg(c)
                    .flex_shrink_0()
                    .into_any_element(),
                None => Icon::new(Lucide::Database)
                    .size(px(12.))
                    .text_color(p.text_muted)
                    .into_any_element(),
            })
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .text_ellipsis()
                    .whitespace_nowrap()
                    .text_xs()
                    .text_color(p.text)
                    .child(name),
            )
            .when_some(badge, |el, b| {
                el.child(
                    div()
                        .px(px(4.))
                        .py(px(1.))
                        .rounded(px(3.))
                        .bg(p.surface_subtle)
                        .text_color(p.text_muted)
                        .text_size(px(10.))
                        .child(b),
                )
            })
            .into_any_element()
    }

    fn render_connection_form(&mut self, p: Palette, cx: &mut Context<Self>) -> AnyElement {
        let draft = &self.active_tab().draft;
        let engine = draft.engine;
        let method = draft.method;
        let color_index = draft.color_index;
        let use_ssl = draft.use_ssl;
        let verify_ssl = draft.verify_server_cert;
        let rsa = draft.request_rsa_key;
        let compression = draft.use_compression;
        let local_infile = draft.allow_data_local_infile;
        let cleartext = draft.enable_cleartext_plugin;
        let connecting = draft.connecting;
        let testing = draft.testing;
        let form_error = draft.form_error.clone();
        let test_status = draft.test_status.clone();
        let tz = draft.timezone.clone();
        let is_selected_fav = draft.selected_favorite.is_some();

        let shows_ssl = engine != EngineKind::Sqlite && method != ConnectionMethod::AwsIam;
        let shows_ssl_files = shows_ssl && use_ssl;

        let tz_label = tz.label();
        let is_fixed_tz = matches!(tz, TimeZoneMode::Fixed(_));

        // Build method fields
        let mut method_fields: Vec<AnyElement> = Vec::new();
        if engine == EngineKind::Sqlite {
            method_fields.push(form_row(
                "Database File",
                Input::new(&draft.inputs.database).small(),
                p,
            ));
        } else {
            match method {
                ConnectionMethod::Tcp => {
                    method_fields.push(form_row("Host", Input::new(&draft.inputs.host).small(), p));
                    method_fields.push(form_row("Username", Input::new(&draft.inputs.user).small(), p));
                    method_fields.push(form_row(
                        "Password",
                        Input::new(&draft.inputs.password).small().mask_toggle(),
                        p,
                    ));
                    method_fields.push(form_row(
                        "Database",
                        Input::new(&draft.inputs.database).small(),
                        p,
                    ));
                    method_fields.push(form_row("Port", Input::new(&draft.inputs.port).small(), p));
                }
                ConnectionMethod::Socket => {
                    method_fields.push(form_row(
                        "Socket",
                        Input::new(&draft.inputs.socket).small(),
                        p,
                    ));
                    method_fields.push(form_row("Username", Input::new(&draft.inputs.user).small(), p));
                    method_fields.push(form_row(
                        "Password",
                        Input::new(&draft.inputs.password).small().mask_toggle(),
                        p,
                    ));
                    method_fields.push(form_row(
                        "Database",
                        Input::new(&draft.inputs.database).small(),
                        p,
                    ));
                }
                ConnectionMethod::SshTunnel => {
                    method_fields.push(form_row(
                        "MySQL Host",
                        Input::new(&draft.inputs.host).small(),
                        p,
                    ));
                    method_fields.push(form_row("Username", Input::new(&draft.inputs.user).small(), p));
                    method_fields.push(form_row(
                        "Password",
                        Input::new(&draft.inputs.password).small().mask_toggle(),
                        p,
                    ));
                    method_fields.push(form_row(
                        "Database",
                        Input::new(&draft.inputs.database).small(),
                        p,
                    ));
                    method_fields.push(form_row("Port", Input::new(&draft.inputs.port).small(), p));
                    method_fields.push(
                        div()
                            .w_full()
                            .h(px(1.))
                            .my(px(4.))
                            .bg(p.hairline)
                            .into_any_element(),
                    );
                    method_fields.push(form_row(
                        "SSH Host",
                        Input::new(&draft.inputs.ssh_host).small(),
                        p,
                    ));
                    method_fields.push(form_row(
                        "SSH User",
                        Input::new(&draft.inputs.ssh_user).small(),
                        p,
                    ));
                    method_fields.push(form_row(
                        "SSH Password",
                        Input::new(&draft.inputs.ssh_password).small().mask_toggle(),
                        p,
                    ));
                    method_fields.push(form_row(
                        "SSH Key",
                        Input::new(&draft.inputs.ssh_key).small(),
                        p,
                    ));
                    method_fields.push(form_row(
                        "SSH Port",
                        Input::new(&draft.inputs.ssh_port).small(),
                        p,
                    ));
                    method_fields.push(form_row(
                        "Remote Socket",
                        Input::new(&draft.inputs.ssh_remote_socket).small(),
                        p,
                    ));
                }
                ConnectionMethod::AwsIam => {
                    method_fields.push(form_row("Host", Input::new(&draft.inputs.host).small(), p));
                    method_fields.push(form_row("Username", Input::new(&draft.inputs.user).small(), p));
                    method_fields.push(form_row(
                        "Database",
                        Input::new(&draft.inputs.database).small(),
                        p,
                    ));
                    method_fields.push(form_row("Port", Input::new(&draft.inputs.port).small(), p));
                    method_fields.push(form_row(
                        "AWS Profile",
                        Input::new(&draft.inputs.aws_profile).small(),
                        p,
                    ));
                    method_fields.push(form_row(
                        "Region",
                        Input::new(&draft.inputs.aws_region).small(),
                        p,
                    ));
                }
                ConnectionMethod::Vault => {
                    method_fields.push(form_row("Host", Input::new(&draft.inputs.host).small(), p));
                    method_fields.push(form_row(
                        "Database",
                        Input::new(&draft.inputs.database).small(),
                        p,
                    ));
                    method_fields.push(form_row("Port", Input::new(&draft.inputs.port).small(), p));
                    method_fields.push(form_row(
                        "Vault Host",
                        Input::new(&draft.inputs.vault_host).small(),
                        p,
                    ));
                    method_fields.push(form_row(
                        "Vault Port",
                        Input::new(&draft.inputs.vault_port).small(),
                        p,
                    ));
                    method_fields.push(form_row(
                        "OIDC Mount",
                        Input::new(&draft.inputs.vault_oidc).small(),
                        p,
                    ));
                    method_fields.push(form_row(
                        "Vault Mount",
                        Input::new(&draft.inputs.vault_mount).small(),
                        p,
                    ));
                    method_fields.push(form_row(
                        "Role",
                        Input::new(&draft.inputs.vault_role).small(),
                        p,
                    ));
                }
            }
        }

        // Scrollable centered container so SSH/SSL fields NEVER clip on smaller screens
        div()
            .size_full()
            .bg(p.window_bg)
            .overflow_y_scrollbar()
            .child(
                v_flex()
                    .w_full()
                    .min_h_full()
                    .py(px(28.))
                    .px(px(24.))
                    .items_center()
                    .justify_center()
                    .child(
                        v_flex()
                            .w(px(480.))
                            .max_w_full()
                            .gap(px(12.))
                            // Minimalist header hint + subtle engine selector
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(p.text_muted)
                                            .child("Enter connection details below, or choose a favorite:"),
                                    )
                                    .child(
                                        h_flex()
                                            .gap(px(2.))
                                            .p(px(2.))
                                            .rounded(px(6.))
                                            .bg(p.surface_subtle)
                                            .children(
                                                [
                                                    (EngineKind::Mysql, "MySQL"),
                                                    (EngineKind::MariaDb, "MariaDB"),
                                                    (EngineKind::Postgres, "PG"),
                                                    (EngineKind::Sqlite, "SQLite"),
                                                ]
                                                .into_iter()
                                                .map(|(ek, lbl)| {
                                                    let active = engine == ek;
                                                    div()
                                                        .id(SharedString::from(format!("eng-{lbl}")))
                                                        .px(px(7.))
                                                        .py(px(2.))
                                                        .rounded(px(4.))
                                                        .cursor_pointer()
                                                        .text_size(px(11.))
                                                        .when(active, |el| {
                                                            el.bg(p.card_bg)
                                                                .text_color(p.text)
                                                                .font_weight(FontWeight::MEDIUM)
                                                                .shadow_xs()
                                                        })
                                                        .when(!active, |el| {
                                                            el.text_color(p.text_muted)
                                                                .hover(|h| h.text_color(p.text))
                                                        })
                                                        .on_click(cx.listener(
                                                            move |this, _, window, cx| {
                                                                this.set_engine(ek, window, cx);
                                                            },
                                                        ))
                                                        .child(lbl)
                                                }),
                                            ),
                                    ),
                            )
                            // Main Sequel Ace Connection Card
                            .child(
                                v_flex()
                                    .relative()
                                    .overflow_hidden()
                                    .w_full()
                                    .rounded(px(10.))
                                    .bg(p.card_bg)
                                    .border_1()
                                    .border_color(p.border)
                                    .px(px(20.))
                                    .py(px(16.))
                                    .gap(px(8.))
                                    .when(connecting || testing, |el| {
                                        el.child(
                                            div()
                                                .absolute()
                                                .top_0()
                                                .left_0()
                                                .right_0()
                                                .child(loading_bar("conn-card-loading", p)),
                                        )
                                    })
                                    // Connection method segmented bar (hidden for SQLite)
                                    .when(engine != EngineKind::Sqlite, |el| {
                                        el.child(
                                            h_flex()
                                                .w_full()
                                                .mb(px(4.))
                                                .p(px(2.))
                                                .gap(px(2.))
                                                .rounded(px(7.))
                                                .bg(p.surface_subtle)
                                                .children(ConnectionMethod::all().iter().map(|m| {
                                                    let m_val = *m;
                                                    let active = method == m_val;
                                                    div()
                                                        .id(SharedString::from(format!(
                                                            "meth-{}",
                                                            m.label()
                                                        )))
                                                        .flex_1()
                                                        .h(px(24.))
                                                        .rounded(px(5.))
                                                        .flex()
                                                        .items_center()
                                                        .justify_center()
                                                        .cursor_pointer()
                                                        .text_xs()
                                                        .when(active, |b| {
                                                            b.bg(p.window_bg)
                                                                .text_color(p.text)
                                                                .font_weight(FontWeight::MEDIUM)
                                                                .border_1()
                                                                .border_color(p.border)
                                                        })
                                                        .when(!active, |b| {
                                                            b.text_color(p.text_muted)
                                                                .hover(|h| h.text_color(p.text))
                                                        })
                                                        .on_click(cx.listener(
                                                            move |this, _, _, cx| {
                                                                let d = &mut this.active_tab_mut().draft;
                                                                d.method = m_val;
                                                                if m_val == ConnectionMethod::AwsIam {
                                                                    d.use_ssl = true;
                                                                }
                                                                d.form_error = None;
                                                                cx.notify();
                                                            },
                                                        ))
                                                        .child(m.label())
                                                })),
                                        )
                                    })
                                    // Name
                                    .child(form_row(
                                        "Name",
                                        Input::new(&draft.inputs.name).small(),
                                        p,
                                    ))
                                    // Colour swatches
                                    .child(form_row(
                                        "Colour",
                                        h_flex()
                                            .h(px(26.))
                                            .gap(px(8.))
                                            .items_center()
                                            .child(color_swatch(
                                                "swatch-none",
                                                None,
                                                color_index < 0,
                                                true,
                                                p,
                                                cx.listener(|this, _, _, cx| {
                                                    this.active_tab_mut().draft.color_index = -1;
                                                    cx.notify();
                                                }),
                                            ))
                                            .children((0..favorite_color_count() as i32).map(|idx| {
                                                color_swatch(
                                                    SharedString::from(format!("swatch-{idx}")),
                                                    favorite_color_rgb(idx),
                                                    color_index == idx,
                                                    false,
                                                    p,
                                                    cx.listener(move |this, _, _, cx| {
                                                        this.active_tab_mut().draft.color_index = idx;
                                                        cx.notify();
                                                    }),
                                                )
                                            })),
                                        p,
                                    ))
                                    // Dynamic method fields
                                    .children(method_fields)
                                    // Timezone selector (MySQL/MariaDB/PG)
                                    .when(engine != EngineKind::Sqlite, |el| {
                                        el.child(form_row(
                                            "Time Zone",
                                            h_flex()
                                                .w_full()
                                                .gap(px(6.))
                                                .items_center()
                                                .child(
                                                    h_flex()
                                                        .id("tz-mode")
                                                        .flex_1()
                                                        .h(px(28.))
                                                        .px(px(10.))
                                                        .items_center()
                                                        .justify_between()
                                                        .rounded(px(6.))
                                                        .bg(p.window_bg)
                                                        .border_1()
                                                        .border_color(p.border)
                                                        .cursor_pointer()
                                                        .hover(|h| h.bg(p.hover_bg))
                                                        .on_click(cx.listener(|this, _, _, cx| {
                                                            let d = &mut this.active_tab_mut().draft;
                                                            d.timezone = match &d.timezone {
                                                                TimeZoneMode::Server => {
                                                                    TimeZoneMode::System
                                                                }
                                                                TimeZoneMode::System => {
                                                                    TimeZoneMode::Fixed("UTC".into())
                                                                }
                                                                TimeZoneMode::Fixed(_) => {
                                                                    TimeZoneMode::Server
                                                                }
                                                            };
                                                            cx.notify();
                                                        }))
                                                        .child(
                                                            div()
                                                                .text_xs()
                                                                .text_color(p.text)
                                                                .child(tz_label),
                                                        )
                                                        .child(
                                                            Icon::new(Lucide::ChevronsUpDown)
                                                                .size(px(12.))
                                                                .text_color(p.text_muted),
                                                        ),
                                                )
                                                .when(is_fixed_tz, |row| {
                                                    row.child(
                                                        div().w(px(110.)).child(
                                                            Input::new(&draft.inputs.timezone_fixed)
                                                                .small(),
                                                        ),
                                                    )
                                                }),
                                            p,
                                        ))
                                    })
                                    // Options checkboxes — aligned 2-column grid under inputs
                                    .when(engine != EngineKind::Sqlite, |el| {
                                        el.child(
                                            v_flex()
                                                .w_full()
                                                .pl(px(110.))
                                                .pt(px(6.))
                                                .gap(px(8.))
                                                .when(shows_ssl, |col| {
                                                    col.child(
                                                        h_flex()
                                                            .w_full()
                                                            .items_center()
                                                            .child(
                                                                div().w(px(164.)).flex_shrink_0().child(
                                                                    Checkbox::new("opt-ssl")
                                                                        .small()
                                                                        .label("Require SSL")
                                                                        .checked(use_ssl)
                                                                        .on_click(cx.listener(
                                                                            |this, checked: &bool, _, cx| {
                                                                                this.active_tab_mut()
                                                                                    .draft
                                                                                    .use_ssl = *checked;
                                                                                cx.notify();
                                                                            },
                                                                        )),
                                                                ),
                                                            )
                                                            .child(
                                                                div().flex_1().child(
                                                                    Checkbox::new("opt-rsa")
                                                                        .small()
                                                                        .label("Get Public Key")
                                                                        .checked(rsa)
                                                                        .on_click(cx.listener(
                                                                            |this, checked: &bool, _, cx| {
                                                                                this.active_tab_mut()
                                                                                    .draft
                                                                                    .request_rsa_key =
                                                                                    *checked;
                                                                                cx.notify();
                                                                            },
                                                                        )),
                                                                ),
                                                            ),
                                                    )
                                                    .when(use_ssl, |col| {
                                                        col.child(
                                                            h_flex()
                                                                .w_full()
                                                                .items_center()
                                                                .child(
                                                                    div().w(px(164.)).flex_shrink_0().child(
                                                                        Checkbox::new("opt-ssl-verify")
                                                                            .small()
                                                                            .label("Verify Server Cert")
                                                                            .checked(verify_ssl)
                                                                            .on_click(cx.listener(
                                                                                |this, checked: &bool, _, cx| {
                                                                                    this.active_tab_mut()
                                                                                        .draft
                                                                                        .verify_server_cert =
                                                                                        *checked;
                                                                                    cx.notify();
                                                                                },
                                                                            )),
                                                                    ),
                                                                ),
                                                        )
                                                    })
                                                })
                                                .child(
                                                    h_flex()
                                                        .w_full()
                                                        .items_center()
                                                        .child(
                                                            div().w(px(164.)).flex_shrink_0().child(
                                                                Checkbox::new("opt-compress")
                                                                    .small()
                                                                    .label("Use Compression")
                                                                    .checked(compression)
                                                                    .on_click(cx.listener(
                                                                        |this, checked: &bool, _, cx| {
                                                                            this.active_tab_mut()
                                                                                .draft
                                                                                .use_compression =
                                                                                *checked;
                                                                            cx.notify();
                                                                        },
                                                                    )),
                                                            ),
                                                        )
                                                        .child(
                                                            div().flex_1().child(
                                                                Checkbox::new("opt-local-infile")
                                                                    .small()
                                                                    .label("LOCAL DATA INFILE")
                                                                    .checked(local_infile)
                                                                    .on_click(cx.listener(
                                                                        |this, checked: &bool, _, cx| {
                                                                            this.active_tab_mut()
                                                                                .draft
                                                                                .allow_data_local_infile =
                                                                                *checked;
                                                                            cx.notify();
                                                                        },
                                                                    )),
                                                            ),
                                                        ),
                                                )
                                                .child(
                                                    h_flex().w_full().items_center().child(
                                                        Checkbox::new("opt-cleartext")
                                                            .small()
                                                            .label("Enable Cleartext Plugin (insecure)")
                                                            .checked(cleartext)
                                                            .on_click(cx.listener(
                                                                |this, checked: &bool, _, cx| {
                                                                    this.active_tab_mut()
                                                                        .draft
                                                                        .enable_cleartext_plugin =
                                                                        *checked;
                                                                    cx.notify();
                                                                },
                                                            )),
                                                    ),
                                                ),
                                        )
                                    })
                                    // SSL cert paths when SSL enabled
                                    .when(shows_ssl_files, |el| {
                                        el.child(
                                            v_flex()
                                                .w_full()
                                                .pt(px(4.))
                                                .gap(px(6.))
                                                .child(form_row(
                                                    "CA Cert",
                                                    Input::new(&draft.inputs.ssl_ca).small(),
                                                    p,
                                                ))
                                                .child(form_row(
                                                    "Certificate",
                                                    Input::new(&draft.inputs.ssl_cert).small(),
                                                    p,
                                                ))
                                                .child(form_row(
                                                    "Key File",
                                                    Input::new(&draft.inputs.ssl_key).small(),
                                                    p,
                                                )),
                                        )
                                    })
                                    // Error / Test status banner (wraps cleanly inside card)
                                    .when_some(form_error, |el, err| {
                                        el.child(
                                            h_flex()
                                                .w_full()
                                                .min_w_0()
                                                .mt(px(4.))
                                                .px(px(10.))
                                                .py(px(8.))
                                                .gap(px(8.))
                                                .items_start()
                                                .rounded(px(6.))
                                                .bg(p.danger_subtle)
                                                .border_1()
                                                .border_color(p.danger)
                                                .overflow_hidden()
                                                .child(
                                                    div().pt(px(1.)).flex_shrink_0().child(
                                                        Icon::new(Lucide::CircleAlert)
                                                            .size(px(14.))
                                                            .text_color(p.danger),
                                                    ),
                                                )
                                                .child(
                                                    div()
                                                        .flex_1()
                                                        .min_w_0()
                                                        .overflow_hidden()
                                                        .text_xs()
                                                        .text_color(p.danger)
                                                        .child(err),
                                                ),
                                        )
                                    })
                                    .when_some(test_status, |el, (ok, msg)| {
                                        let fg = if ok { p.success } else { p.danger };
                                        el.child(
                                            h_flex()
                                                .w_full()
                                                .min_w_0()
                                                .mt(px(2.))
                                                .px(px(10.))
                                                .py(px(7.))
                                                .gap(px(8.))
                                                .items_start()
                                                .rounded(px(6.))
                                                .bg(p.surface_subtle)
                                                .overflow_hidden()
                                                .child(
                                                    div().pt(px(1.)).flex_shrink_0().child(
                                                        Icon::new(if ok {
                                                            Lucide::CircleCheck
                                                        } else {
                                                            Lucide::CircleX
                                                        })
                                                        .size(px(14.))
                                                        .text_color(fg),
                                                    ),
                                                )
                                                .child(
                                                    div()
                                                        .flex_1()
                                                        .min_w_0()
                                                        .overflow_hidden()
                                                        .text_xs()
                                                        .text_color(fg)
                                                        .child(msg),
                                                ),
                                        )
                                    })
                                    // Bottom Action Row
                                    .child(
                                        h_flex()
                                            .w_full()
                                            .mt(px(8.))
                                            .pt(px(12.))
                                            .border_t_1()
                                            .border_color(p.hairline)
                                            .items_center()
                                            .gap(px(8.))
                                            .child(
                                                Button::new("btn-test-conn")
                                                    .outline()
                                                    .small()
                                                    .loading(testing)
                                                    .label("Test Connection")
                                                    .on_click(cx.listener(|this, _, window, cx| {
                                                        this.test_active_connection(window, cx);
                                                    })),
                                            )
                                            .child(
                                                Button::new("btn-save-fav")
                                                    .ghost()
                                                    .small()
                                                    .label(if is_selected_fav {
                                                        "Save Changes"
                                                    } else {
                                                        "Add to Favorites"
                                                    })
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.save_current_favorite(cx);
                                                    })),
                                            )
                                            .child(div().flex_1())
                                            .child(
                                                div()
                                                    .id("btn-connect-primary")
                                                    .h(px(30.))
                                                    .min_w(px(106.))
                                                    .px(px(16.))
                                                    .rounded(px(6.))
                                                    .bg(p.accent)
                                                    .hover(|h| h.bg(p.accent_hover))
                                                    .cursor_pointer()
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .gap(px(6.))
                                                    .text_xs()
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(p.accent_fg)
                                                    .on_click(cx.listener(|this, _, window, cx| {
                                                        this.connect_active_tab(window, cx);
                                                    }))
                                                    .when(connecting, |el| {
                                                        el.child(loading_spinner(
                                                            Size::XSmall,
                                                            p.accent_fg,
                                                        ))
                                                    })
                                                    .child(if connecting {
                                                        "Connecting…"
                                                    } else {
                                                        "Connect"
                                                    }),
                                            ),
                                    ),
                            ),
                    ),
            )
            .into_any_element()
    }
}

fn form_row(label: &'static str, field: impl IntoElement, p: Palette) -> AnyElement {
    h_flex()
        .w_full()
        .min_h(px(28.))
        .gap(px(10.))
        .items_center()
        .child(
            div()
                .w(px(100.))
                .flex_shrink_0()
                .text_right()
                .text_xs()
                .text_color(p.text_secondary)
                .child(label),
        )
        .child(div().flex_1().min_w_0().child(field))
        .into_any_element()
}

fn color_swatch(
    id: impl Into<SharedString>,
    rgb_opt: Option<(u8, u8, u8)>,
    selected: bool,
    none: bool,
    p: Palette,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let dot = if none {
        div()
            .size(px(15.))
            .rounded(px(8.))
            .border_1()
            .border_color(if selected { p.accent } else { p.text_muted })
            .flex()
            .items_center()
            .justify_center()
            .child(div().w(px(7.)).h(px(1.5)).bg(p.text_muted))
    } else {
        let (r, g, b) = rgb_opt.unwrap_or((160, 160, 160));
        let hex = ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
        div()
            .size(px(15.))
            .rounded(px(8.))
            .bg(rgb(hex))
            .when(selected, |el| el.border_2().border_color(p.text))
            .when(!selected, |el| el.border_1().border_color(p.border))
    };

    div()
        .id(id.into())
        .p(px(2.))
        .rounded(px(10.))
        .cursor_pointer()
        .when(selected, |el| el.bg(p.accent_subtle))
        .on_click(on_click)
        .child(dot)
        .into_any_element()
}
