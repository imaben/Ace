//! Clean Sequel Ace modal overlays & sheets.

use crate::gui::root::AceApp;
use crate::gui::state::{ModalState, ServerToolsTab};
use crate::gui::theme::Palette;
use ace_io::ExportFormat;
use gpui_kit::assets::IconName as Lucide;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::input::{Input, Textarea};
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::table::DataTable;
use gpui_kit::component::{h_flex, v_flex, Icon, Sizable as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

impl AceApp {
    pub(crate) fn render_modal_overlay(&mut self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let modal = self.modal.as_ref()?;
        let p = Palette::for_app(cx);

        let card: AnyElement = match modal {
            ModalState::Confirm {
                title,
                message,
                confirm_label,
                ..
            } => modal_shell(
                title.clone(),
                px(400.),
                p,
                cx,
                div()
                    .text_xs()
                    .text_color(p.text_secondary)
                    .child(message.clone())
                    .into_any_element(),
                h_flex()
                    .w_full()
                    .justify_end()
                    .gap(px(8.))
                    .child(
                        Button::new("mdl-cancel")
                            .outline()
                            .small()
                            .label("Cancel")
                            .on_click(cx.listener(|this, _, _, cx| this.close_modal(cx))),
                    )
                    .child(
                        Button::new("mdl-confirm")
                            .danger()
                            .small()
                            .label(confirm_label.clone())
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.execute_confirm_modal(window, cx);
                            })),
                    )
                    .into_any_element(),
            ),

            ModalState::FieldEditor {
                column_name,
                is_null,
                textarea,
                ..
            } => {
                let null_on = *is_null;
                let ta = textarea.clone();
                modal_shell(
                    format!("Edit Field — {column_name}"),
                    px(620.),
                    p,
                    cx,
                    v_flex()
                        .gap(px(8.))
                        .child(
                            h_flex()
                                .gap(px(8.))
                                .items_center()
                                .child(
                                    Checkbox::new("fe-null")
                                        .small()
                                        .label("SQL NULL")
                                        .checked(null_on)
                                        .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                            if let Some(ModalState::FieldEditor {
                                                is_null, ..
                                            }) = this.modal.as_mut()
                                            {
                                                *is_null = *checked;
                                            }
                                            cx.notify();
                                        })),
                                )
                                .child(div().flex_1())
                                .child(
                                    Button::new("fe-pretty-json")
                                        .ghost()
                                        .xsmall()
                                        .icon(Icon::new(Lucide::Braces).size(px(12.)))
                                        .label("Pretty JSON")
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.pretty_format_field_editor_json(window, cx);
                                        })),
                                )
                                .child(
                                    Button::new("fe-copy")
                                        .ghost()
                                        .xsmall()
                                        .icon(Icon::new(Lucide::Copy).size(px(12.)))
                                        .label("Copy")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            if let Some(ModalState::FieldEditor {
                                                textarea, ..
                                            }) = &this.modal
                                            {
                                                let val = textarea.read(cx).value().to_string();
                                                cx.write_to_clipboard(ClipboardItem::new_string(
                                                    val,
                                                ));
                                            }
                                        })),
                                ),
                        )
                        .child(
                            div().h(px(280.)).w_full().child(
                                Textarea::new(&ta)
                                    .h(px(280.))
                                    .disabled(null_on)
                                    .small(),
                            ),
                        )
                        .into_any_element(),
                    h_flex()
                        .w_full()
                        .justify_end()
                        .gap(px(8.))
                        .child(
                            Button::new("fe-cancel")
                                .outline()
                                .small()
                                .label("Cancel")
                                .on_click(cx.listener(|this, _, _, cx| this.close_modal(cx))),
                        )
                        .child(
                            Button::new("fe-save")
                                .primary()
                                .small()
                                .label("Save")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.commit_field_editor(window, cx);
                                })),
                        )
                        .into_any_element(),
                )
            }

            ModalState::InsertRow {
                table,
                columns,
                inputs,
                null_flags,
            } => {
                let rows: Vec<AnyElement> = columns
                    .iter()
                    .enumerate()
                    .map(|(idx, col)| {
                        let is_null = null_flags.get(idx).copied().unwrap_or(false);
                        let inp = inputs[idx].clone();
                        h_flex()
                            .w_full()
                            .gap(px(8.))
                            .items_center()
                            .child(
                                div()
                                    .w(px(130.))
                                    .flex_shrink_0()
                                    .text_right()
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .whitespace_nowrap()
                                    .text_xs()
                                    .text_color(p.text_secondary)
                                    .child(col.clone()),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .child(Input::new(&inp).small().disabled(is_null)),
                            )
                            .child(
                                Checkbox::new(SharedString::from(format!("ins-null-{idx}")))
                                    .small()
                                    .label("NULL")
                                    .checked(is_null)
                                    .on_click(cx.listener(move |this, checked: &bool, _, cx| {
                                        if let Some(ModalState::InsertRow { null_flags, .. }) =
                                            this.modal.as_mut()
                                        {
                                            if let Some(f) = null_flags.get_mut(idx) {
                                                *f = *checked;
                                            }
                                        }
                                        cx.notify();
                                    })),
                            )
                            .into_any_element()
                    })
                    .collect();

                modal_shell(
                    format!("Insert Row — {table}"),
                    px(520.),
                    p,
                    cx,
                    div()
                        .max_h(px(360.))
                        .w_full()
                        .overflow_y_scrollbar()
                        .child(v_flex().w_full().gap(px(7.)).pr(px(6.)).children(rows))
                        .into_any_element(),
                    h_flex()
                        .w_full()
                        .justify_end()
                        .gap(px(8.))
                        .child(
                            Button::new("ins-cancel")
                                .outline()
                                .small()
                                .label("Cancel")
                                .on_click(cx.listener(|this, _, _, cx| this.close_modal(cx))),
                        )
                        .child(
                            Button::new("ins-submit")
                                .primary()
                                .small()
                                .label("Insert Row")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.commit_insert_row(window, cx);
                                })),
                        )
                        .into_any_element(),
                )
            }

            ModalState::AddDatabase {
                name,
                charset,
                collation,
            } => modal_shell(
                "Add Database".into(),
                px(420.),
                p,
                cx,
                v_flex()
                    .gap(px(8.))
                    .child(modal_row("Name", Input::new(name).small(), p))
                    .child(modal_row("Encoding", Input::new(charset).small(), p))
                    .child(modal_row("Collation", Input::new(collation).small(), p))
                    .into_any_element(),
                h_flex()
                    .w_full()
                    .justify_end()
                    .gap(px(8.))
                    .child(
                        Button::new("adb-cancel")
                            .outline()
                            .small()
                            .label("Cancel")
                            .on_click(cx.listener(|this, _, _, cx| this.close_modal(cx))),
                    )
                    .child(
                        Button::new("adb-ok")
                            .primary()
                            .small()
                            .label("Create Database")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.commit_add_database(window, cx);
                            })),
                    )
                    .into_any_element(),
            ),

            ModalState::AddTable {
                name,
                engine,
                charset,
            } => modal_shell(
                "Add Table".into(),
                px(420.),
                p,
                cx,
                v_flex()
                    .gap(px(8.))
                    .child(modal_row("Table Name", Input::new(name).small(), p))
                    .child(modal_row("Engine", Input::new(engine).small(), p))
                    .child(modal_row("Encoding", Input::new(charset).small(), p))
                    .child(
                        div()
                            .pl(px(108.))
                            .text_size(px(11.))
                            .text_color(p.text_muted)
                            .child("Creates table with an auto-increment primary key `id`."),
                    )
                    .into_any_element(),
                h_flex()
                    .w_full()
                    .justify_end()
                    .gap(px(8.))
                    .child(
                        Button::new("atbl-cancel")
                            .outline()
                            .small()
                            .label("Cancel")
                            .on_click(cx.listener(|this, _, _, cx| this.close_modal(cx))),
                    )
                    .child(
                        Button::new("atbl-ok")
                            .primary()
                            .small()
                            .label("Create Table")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.commit_add_table(window, cx);
                            })),
                    )
                    .into_any_element(),
            ),

            ModalState::RenameTable { old_name, new_name } => modal_shell(
                format!("Rename Table — {old_name}"),
                px(400.),
                p,
                cx,
                modal_row("New Name", Input::new(new_name).small(), p),
                h_flex()
                    .w_full()
                    .justify_end()
                    .gap(px(8.))
                    .child(
                        Button::new("rnt-cancel")
                            .outline()
                            .small()
                            .label("Cancel")
                            .on_click(cx.listener(|this, _, _, cx| this.close_modal(cx))),
                    )
                    .child(
                        Button::new("rnt-ok")
                            .primary()
                            .small()
                            .label("Rename")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.commit_rename_table(window, cx);
                            })),
                    )
                    .into_any_element(),
            ),

            ModalState::EditColumn {
                table,
                editing_old_name,
                name,
                data_type,
                nullable,
                auto_increment,
                default_val,
                comment,
            } => {
                let is_edit = editing_old_name.is_some();
                let null_on = *nullable;
                let ai_on = *auto_increment;
                modal_shell(
                    if is_edit {
                        format!("Edit Column — {table}")
                    } else {
                        format!("Add Column — {table}")
                    },
                    px(460.),
                    p,
                    cx,
                    v_flex()
                        .gap(px(8.))
                        .child(modal_row("Field Name", Input::new(name).small(), p))
                        .child(modal_row("Type", Input::new(data_type).small(), p))
                        .child(modal_row("Default", Input::new(default_val).small(), p))
                        .child(modal_row("Comment", Input::new(comment).small(), p))
                        .child(
                            h_flex()
                                .pl(px(108.))
                                .gap(px(16.))
                                .child(
                                    Checkbox::new("ecol-null")
                                        .small()
                                        .label("Allow NULL")
                                        .checked(null_on)
                                        .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                            if let Some(ModalState::EditColumn {
                                                nullable, ..
                                            }) = this.modal.as_mut()
                                            {
                                                *nullable = *checked;
                                            }
                                            cx.notify();
                                        })),
                                )
                                .child(
                                    Checkbox::new("ecol-ai")
                                        .small()
                                        .label("Auto Increment")
                                        .checked(ai_on)
                                        .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                            if let Some(ModalState::EditColumn {
                                                auto_increment,
                                                ..
                                            }) = this.modal.as_mut()
                                            {
                                                *auto_increment = *checked;
                                            }
                                            cx.notify();
                                        })),
                                ),
                        )
                        .into_any_element(),
                    h_flex()
                        .w_full()
                        .justify_end()
                        .gap(px(8.))
                        .child(
                            Button::new("ecol-cancel")
                                .outline()
                                .small()
                                .label("Cancel")
                                .on_click(cx.listener(|this, _, _, cx| this.close_modal(cx))),
                        )
                        .child(
                            Button::new("ecol-ok")
                                .primary()
                                .small()
                                .label(if is_edit { "Save Column" } else { "Add Column" })
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.commit_edit_column(window, cx);
                                })),
                        )
                        .into_any_element(),
                )
            }

            ModalState::AddIndex {
                table,
                name,
                columns,
                unique,
            } => {
                let uniq = *unique;
                modal_shell(
                    format!("Add Index — {table}"),
                    px(430.),
                    p,
                    cx,
                    v_flex()
                        .gap(px(8.))
                        .child(modal_row("Index Name", Input::new(name).small(), p))
                        .child(modal_row("Columns", Input::new(columns).small(), p))
                        .child(
                            div().pl(px(108.)).child(
                                Checkbox::new("aidx-uniq")
                                    .small()
                                    .label("Unique Index")
                                    .checked(uniq)
                                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                        if let Some(ModalState::AddIndex { unique, .. }) =
                                            this.modal.as_mut()
                                        {
                                            *unique = *checked;
                                        }
                                        cx.notify();
                                    })),
                            ),
                        )
                        .into_any_element(),
                    h_flex()
                        .w_full()
                        .justify_end()
                        .gap(px(8.))
                        .child(
                            Button::new("aidx-cancel")
                                .outline()
                                .small()
                                .label("Cancel")
                                .on_click(cx.listener(|this, _, _, cx| this.close_modal(cx))),
                        )
                        .child(
                            Button::new("aidx-ok")
                                .primary()
                                .small()
                                .label("Add Index")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.commit_add_index(window, cx);
                                })),
                        )
                        .into_any_element(),
                )
            }

            ModalState::AddForeignKey {
                table,
                name,
                column,
                ref_table,
                ref_column,
                on_update,
                on_delete,
            } => modal_shell(
                format!("Add Foreign Key — {table}"),
                px(460.),
                p,
                cx,
                v_flex()
                    .gap(px(8.))
                    .child(modal_row("Constraint", Input::new(name).small(), p))
                    .child(modal_row("Column(s)", Input::new(column).small(), p))
                    .child(modal_row("Ref Table", Input::new(ref_table).small(), p))
                    .child(modal_row("Ref Column(s)", Input::new(ref_column).small(), p))
                    .child(modal_row("On Update", Input::new(on_update).small(), p))
                    .child(modal_row("On Delete", Input::new(on_delete).small(), p))
                    .into_any_element(),
                h_flex()
                    .w_full()
                    .justify_end()
                    .gap(px(8.))
                    .child(
                        Button::new("afk-cancel")
                            .outline()
                            .small()
                            .label("Cancel")
                            .on_click(cx.listener(|this, _, _, cx| this.close_modal(cx))),
                    )
                    .child(
                        Button::new("afk-ok")
                            .primary()
                            .small()
                            .label("Add Relation")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.commit_add_foreign_key(window, cx);
                            })),
                    )
                    .into_any_element(),
            ),

            ModalState::AddTrigger {
                table,
                name,
                timing,
                event,
                body,
            } => {
                let timing_val = timing.clone();
                let event_val = event.clone();
                let body_ent = body.clone();
                modal_shell(
                    format!("Add Trigger — {table}"),
                    px(520.),
                    p,
                    cx,
                    v_flex()
                        .gap(px(8.))
                        .child(modal_row("Name", Input::new(name).small(), p))
                        .child(modal_row(
                            "Timing",
                            h_flex()
                                .gap(px(6.))
                                .children(["BEFORE", "AFTER"].into_iter().map(|t| {
                                    Button::new(SharedString::from(format!("tr-t-{t}")))
                                        .outline()
                                        .xsmall()
                                        .toggled(timing_val == t)
                                        .label(t)
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            if let Some(ModalState::AddTrigger { timing, .. }) =
                                                this.modal.as_mut()
                                            {
                                                *timing = t.into();
                                            }
                                            cx.notify();
                                        }))
                                })),
                            p,
                        ))
                        .child(modal_row(
                            "Event",
                            h_flex()
                                .gap(px(6.))
                                .children(["INSERT", "UPDATE", "DELETE"].into_iter().map(|e| {
                                    Button::new(SharedString::from(format!("tr-e-{e}")))
                                        .outline()
                                        .xsmall()
                                        .toggled(event_val == e)
                                        .label(e)
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            if let Some(ModalState::AddTrigger { event, .. }) =
                                                this.modal.as_mut()
                                            {
                                                *event = e.into();
                                            }
                                            cx.notify();
                                        }))
                                })),
                            p,
                        ))
                        .child(
                            v_flex()
                                .gap(px(4.))
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(p.text_secondary)
                                        .child("Statement Body"),
                                )
                                .child(Textarea::new(&body_ent).h(px(140.)).small()),
                        )
                        .into_any_element(),
                    h_flex()
                        .w_full()
                        .justify_end()
                        .gap(px(8.))
                        .child(
                            Button::new("atrg-cancel")
                                .outline()
                                .small()
                                .label("Cancel")
                                .on_click(cx.listener(|this, _, _, cx| this.close_modal(cx))),
                        )
                        .child(
                            Button::new("atrg-ok")
                                .primary()
                                .small()
                                .label("Create Trigger")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.commit_add_trigger(window, cx);
                                })),
                        )
                        .into_any_element(),
                )
            }

            ModalState::SaveQueryFavorite { name, .. } => modal_shell(
                "Save Query Favorite".into(),
                px(400.),
                p,
                cx,
                modal_row("Name", Input::new(name).small(), p),
                h_flex()
                    .w_full()
                    .justify_end()
                    .gap(px(8.))
                    .child(
                        Button::new("sqf-cancel")
                            .outline()
                            .small()
                            .label("Cancel")
                            .on_click(cx.listener(|this, _, _, cx| this.close_modal(cx))),
                    )
                    .child(
                        Button::new("sqf-ok")
                            .primary()
                            .small()
                            .label("Save")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.commit_save_query_favorite(cx);
                            })),
                    )
                    .into_any_element(),
            ),

            ModalState::NewGroup { name } => modal_shell(
                "New Favorites Group".into(),
                px(380.),
                p,
                cx,
                modal_row("Group Name", Input::new(name).small(), p),
                h_flex()
                    .w_full()
                    .justify_end()
                    .gap(px(8.))
                    .child(
                        Button::new("ng-cancel")
                            .outline()
                            .small()
                            .label("Cancel")
                            .on_click(cx.listener(|this, _, _, cx| this.close_modal(cx))),
                    )
                    .child(
                        Button::new("ng-ok")
                            .primary()
                            .small()
                            .label("Create Group")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.commit_new_group(cx);
                            })),
                    )
                    .into_any_element(),
            ),

            ModalState::Export {
                source_label,
                result,
                path,
                format,
                gzip,
                status,
                ..
            } => {
                let fmt_val = *format;
                let gz = *gzip;
                let row_count = result.rows.len();
                modal_shell(
                    format!("Export — {source_label} ({row_count} rows)"),
                    px(490.),
                    p,
                    cx,
                    v_flex()
                        .gap(px(10.))
                        .child(modal_row(
                            "Format",
                            h_flex()
                                .gap(px(4.))
                                .children(
                                    [
                                        (ExportFormat::Sql, "SQL"),
                                        (ExportFormat::Csv, "CSV"),
                                        (ExportFormat::Json, "JSON"),
                                        (ExportFormat::Xml, "XML"),
                                        (ExportFormat::Html, "HTML"),
                                    ]
                                    .into_iter()
                                    .map(|(f, lbl)| {
                                        Button::new(SharedString::from(format!("exp-fmt-{lbl}")))
                                            .outline()
                                            .xsmall()
                                            .toggled(fmt_val == f)
                                            .label(lbl)
                                            .on_click(cx.listener(move |this, _, window, cx| {
                                                this.set_export_format(f, window, cx);
                                            }))
                                    }),
                                ),
                            p,
                        ))
                        .child(modal_row("Output Path", Input::new(path).small(), p))
                        .child(
                            div().pl(px(108.)).child(
                                Checkbox::new("exp-gz")
                                    .small()
                                    .label("Compress with Gzip (.gz)")
                                    .checked(gz)
                                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                        if let Some(ModalState::Export { gzip, .. }) =
                                            this.modal.as_mut()
                                        {
                                            *gzip = *checked;
                                        }
                                        cx.notify();
                                    })),
                            ),
                        )
                        .when_some(status.clone(), |el, msg| {
                            el.child(
                                div()
                                    .pl(px(108.))
                                    .text_xs()
                                    .text_color(p.text_secondary)
                                    .child(msg),
                            )
                        })
                        .into_any_element(),
                    h_flex()
                        .w_full()
                        .justify_end()
                        .gap(px(8.))
                        .child(
                            Button::new("exp-close")
                                .outline()
                                .small()
                                .label("Close")
                                .on_click(cx.listener(|this, _, _, cx| this.close_modal(cx))),
                        )
                        .child(
                            Button::new("exp-run")
                                .primary()
                                .small()
                                .label("Export")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.commit_export(window, cx);
                                })),
                        )
                        .into_any_element(),
                )
            }

            ModalState::ImportSql {
                path,
                running,
                status,
            } => modal_shell(
                "Import SQL File".into(),
                px(470.),
                p,
                cx,
                v_flex()
                    .gap(px(8.))
                    .child(modal_row("SQL File Path", Input::new(path).small(), p))
                    .when_some(status.clone(), |el, msg| {
                        el.child(
                            div()
                                .pl(px(108.))
                                .text_xs()
                                .text_color(p.text_secondary)
                                .child(msg),
                        )
                    })
                    .into_any_element(),
                h_flex()
                    .w_full()
                    .justify_end()
                    .gap(px(8.))
                    .child(
                        Button::new("imp-close")
                            .outline()
                            .small()
                            .label("Close")
                            .on_click(cx.listener(|this, _, _, cx| this.close_modal(cx))),
                    )
                    .child(
                        Button::new("imp-run")
                            .primary()
                            .small()
                            .loading(*running)
                            .label("Import")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.commit_import_sql(window, cx);
                            })),
                    )
                    .into_any_element(),
            ),

            ModalState::FavoritesPlist { path, status } => modal_shell(
                "Sequel Ace Favorites (.plist)".into(),
                px(480.),
                p,
                cx,
                v_flex()
                    .gap(px(8.))
                    .child(modal_row("Plist Path", Input::new(path).small(), p))
                    .when_some(status.clone(), |el, msg| {
                        el.child(
                            div()
                                .pl(px(108.))
                                .text_xs()
                                .text_color(p.text_secondary)
                                .child(msg),
                        )
                    })
                    .into_any_element(),
                h_flex()
                    .w_full()
                    .justify_end()
                    .gap(px(8.))
                    .child(
                        Button::new("pl-close")
                            .outline()
                            .small()
                            .label("Close")
                            .on_click(cx.listener(|this, _, _, cx| this.close_modal(cx))),
                    )
                    .child(
                        Button::new("pl-export")
                            .outline()
                            .small()
                            .label("Export .plist")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.commit_export_plist(cx);
                            })),
                    )
                    .child(
                        Button::new("pl-import")
                            .primary()
                            .small()
                            .label("Import .plist")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.commit_import_plist(cx);
                            })),
                    )
                    .into_any_element(),
            ),

            ModalState::ServerTools {
                tab,
                loading,
                var_filter,
                proc_table,
                var_table,
                user_table,
                error,
                ..
            } => {
                let active = *tab;
                let is_loading = *loading;
                let server_info = self
                    .active_tab()
                    .session
                    .as_ref()
                    .and_then(|s| s.server_info.clone());

                let body: AnyElement = match active {
                    ServerToolsTab::Processes => v_flex()
                        .h(px(370.))
                        .w_full()
                        .gap(px(8.))
                        .child(
                            h_flex()
                                .gap(px(6.))
                                .items_center()
                                .child(
                                    Button::new("st-kill-q")
                                        .danger()
                                        .xsmall()
                                        .label("Kill Query")
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.kill_selected_process(false, window, cx);
                                        })),
                                )
                                .child(
                                    Button::new("st-kill-c")
                                        .outline()
                                        .xsmall()
                                        .label("Kill Connection")
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.kill_selected_process(true, window, cx);
                                        })),
                                )
                                .child(div().flex_1())
                                .child(
                                    Button::new("st-ref-p")
                                        .ghost()
                                        .xsmall()
                                        .loading(is_loading)
                                        .icon(Icon::new(Lucide::RefreshCw).size(px(12.)))
                                        .label("Refresh")
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.refresh_server_tools(window, cx);
                                        })),
                                ),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_h_0()
                                .w_full()
                                .border_1()
                                .border_color(p.border)
                                .rounded(px(6.))
                                .overflow_hidden()
                                .child(
                                    DataTable::new(proc_table)
                                        .stripe(true)
                                        .bordered(false)
                                        .small(),
                                ),
                        )
                        .into_any_element(),
                    ServerToolsTab::Variables => v_flex()
                        .h(px(370.))
                        .w_full()
                        .gap(px(8.))
                        .child(
                            h_flex()
                                .gap(px(8.))
                                .items_center()
                                .child(
                                    div().w(px(260.)).child(
                                        Input::new(var_filter)
                                            .small()
                                            .cleanable(true)
                                            .prefix(
                                                Icon::new(Lucide::Search)
                                                    .size(px(12.))
                                                    .text_color(p.text_muted),
                                            ),
                                    ),
                                )
                                .child(
                                    Button::new("st-var-apply")
                                        .outline()
                                        .xsmall()
                                        .label("Filter")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.filter_server_variables(cx);
                                        })),
                                )
                                .child(div().flex_1())
                                .child(
                                    Button::new("st-ref-v")
                                        .ghost()
                                        .xsmall()
                                        .loading(is_loading)
                                        .icon(Icon::new(Lucide::RefreshCw).size(px(12.)))
                                        .label("Refresh")
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.refresh_server_tools(window, cx);
                                        })),
                                ),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_h_0()
                                .w_full()
                                .border_1()
                                .border_color(p.border)
                                .rounded(px(6.))
                                .overflow_hidden()
                                .child(
                                    DataTable::new(var_table)
                                        .stripe(true)
                                        .bordered(false)
                                        .small(),
                                ),
                        )
                        .into_any_element(),
                    ServerToolsTab::Users => v_flex()
                        .h(px(370.))
                        .w_full()
                        .gap(px(8.))
                        .child(
                            div()
                                .flex_1()
                                .min_h_0()
                                .w_full()
                                .border_1()
                                .border_color(p.border)
                                .rounded(px(6.))
                                .overflow_hidden()
                                .child(
                                    DataTable::new(user_table)
                                        .stripe(true)
                                        .bordered(false)
                                        .small(),
                                ),
                        )
                        .into_any_element(),
                    ServerToolsTab::ServerInfo => v_flex()
                        .h(px(370.))
                        .w_full()
                        .p(px(12.))
                        .gap(px(8.))
                        .rounded(px(6.))
                        .bg(p.surface_subtle)
                        .children(match server_info {
                            Some(s) => vec![
                                kv_line("Engine", &s.engine_name, p),
                                kv_line("Version", &s.version, p),
                                kv_line("Distribution", &s.version_comment, p),
                                kv_line("Host", &format!("{}:{}", s.host, s.port), p),
                                kv_line("Current User", &s.current_user, p),
                                kv_line(
                                    "Current Database",
                                    s.current_database.as_deref().unwrap_or("—"),
                                    p,
                                ),
                                kv_line("Connection Charset", &s.charset, p),
                            ],
                            None => vec![div()
                                .text_xs()
                                .text_color(p.text_muted)
                                .child("Server info unavailable")
                                .into_any_element()],
                        })
                        .into_any_element(),
                };

                modal_shell(
                    "Server Tools".into(),
                    px(760.),
                    p,
                    cx,
                    v_flex()
                        .gap(px(10.))
                        .child(
                            h_flex()
                                .gap(px(4.))
                                .children(
                                    [
                                        (ServerToolsTab::Processes, "Processlist", Lucide::Activity),
                                        (
                                            ServerToolsTab::Variables,
                                            "Variables",
                                            Lucide::SlidersHorizontal,
                                        ),
                                        (ServerToolsTab::Users, "Users", Lucide::Users),
                                        (ServerToolsTab::ServerInfo, "Server Info", Lucide::Server),
                                    ]
                                    .into_iter()
                                    .map(|(t, lbl, ic)| {
                                        Button::new(SharedString::from(format!("st-tab-{lbl}")))
                                            .ghost()
                                            .small()
                                            .toggled(active == t)
                                            .icon(Icon::new(ic).size(px(12.)))
                                            .label(lbl)
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                if let Some(ModalState::ServerTools {
                                                    tab, ..
                                                }) = this.modal.as_mut()
                                                {
                                                    *tab = t;
                                                }
                                                cx.notify();
                                            }))
                                    }),
                                ),
                        )
                        .when_some(error.clone(), |el, err| {
                            el.child(div().text_xs().text_color(p.danger).child(err))
                        })
                        .child(body)
                        .into_any_element(),
                    h_flex()
                        .w_full()
                        .justify_end()
                        .child(
                            Button::new("st-close")
                                .outline()
                                .small()
                                .label("Done")
                                .on_click(cx.listener(|this, _, _, cx| this.close_modal(cx))),
                        )
                        .into_any_element(),
                )
            }
        };

        // Backdrop + centered modal
        Some(
            div()
                .id("modal-backdrop")
                .absolute()
                .inset_0()
                .bg(rgba(0x00000066))
                .flex()
                .items_center()
                .justify_center()
                .p(px(24.))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.close_modal(cx);
                }))
                .child(card)
                .into_any_element(),
        )
    }
}

fn modal_shell(
    title: String,
    width: Pixels,
    p: Palette,
    cx: &mut Context<AceApp>,
    body: AnyElement,
    footer: AnyElement,
) -> AnyElement {
    v_flex()
        .id("modal-card")
        .w(width)
        .max_w_full()
        .rounded(px(10.))
        .bg(p.card_bg)
        .border_1()
        .border_color(p.border)
        .shadow_lg()
        .on_click(|_, _, cx| {
            cx.stop_propagation();
        })
        // Header
        .child(
            h_flex()
                .h(px(40.))
                .w_full()
                .px(px(16.))
                .items_center()
                .justify_between()
                .border_b_1()
                .border_color(p.hairline)
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(p.text)
                        .child(title),
                )
                .child(
                    Button::new("mdl-x")
                        .ghost()
                        .xsmall()
                        .icon(Icon::new(Lucide::X).size(px(13.)))
                        .on_click(cx.listener(|this, _, _, cx| this.close_modal(cx))),
                ),
        )
        // Body
        .child(div().w_full().p(px(16.)).child(body))
        // Footer
        .child(
            div()
                .w_full()
                .px(px(16.))
                .py(px(12.))
                .border_t_1()
                .border_color(p.hairline)
                .child(footer),
        )
        .into_any_element()
}

fn modal_row(label: &'static str, field: impl IntoElement, p: Palette) -> AnyElement {
    h_flex()
        .w_full()
        .gap(px(10.))
        .items_center()
        .child(
            div()
                .w(px(98.))
                .flex_shrink_0()
                .text_right()
                .text_xs()
                .text_color(p.text_secondary)
                .child(label),
        )
        .child(div().flex_1().min_w_0().child(field))
        .into_any_element()
}

fn kv_line(k: &'static str, v: &str, p: Palette) -> AnyElement {
    h_flex()
        .w_full()
        .gap(px(12.))
        .child(
            div()
                .w(px(140.))
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
