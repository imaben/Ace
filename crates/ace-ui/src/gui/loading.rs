//! Generic, non-blocking loading components for Ace.
//!
//! Design rule: loading feedback is always scoped to the specific panel or control
//! that is fetching data — it never dims or blocks the entire application window.

use crate::gui::theme::Palette;
use gpui_kit::assets::IconName as Lucide;
use gpui_kit::component::skeleton::Skeleton;
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::{h_flex, v_flex, Icon, Sizable as _, Size};
use gpui_kit::*;
use std::time::Duration;

/// Animated rotating spinner icon that respects system reduced-motion settings.
pub fn loading_spinner(size: Size, color: Rgba) -> AnyElement {
    Spinner::new()
        .icon(Icon::new(Lucide::LoaderCircle))
        .with_size(size)
        .color(color.into())
        .into_any_element()
}

/// Sleek 2px indeterminate animated progress bar for the top edge of a card or panel.
pub fn loading_bar(id: impl Into<ElementId>, p: Palette) -> AnyElement {
    let anim_id: ElementId = id.into();
    div()
        .w_full()
        .h(px(2.))
        .overflow_hidden()
        .bg(p.accent_subtle)
        .child(
            div()
                .h_full()
                .w(relative(0.32))
                .rounded(px(1.))
                .bg(p.accent)
                .with_animation(
                    anim_id,
                    Animation::new(Duration::from_millis(1200))
                        .repeat()
                        .with_easing(ease_in_out),
                    |this, delta| this.ml(relative(-0.32 + delta * 1.32)),
                ),
        )
        .into_any_element()
}

/// Compact inline spinner + label (for toolbars, footers, buttons, and modal status lines).
pub fn loading_inline(label: impl Into<SharedString>, p: Palette) -> AnyElement {
    let text: SharedString = label.into();
    h_flex()
        .gap(px(6.))
        .items_center()
        .px(px(8.))
        .py(px(3.))
        .rounded(px(6.))
        .bg(p.accent_subtle)
        .child(loading_spinner(Size::XSmall, p.accent))
        .child(
            div()
                .text_size(px(11.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.accent)
                .whitespace_nowrap()
                .child(text),
        )
        .into_any_element()
}

/// Non-blocking floating pill anchored at the top-right corner of a loading panel.
/// Keeps existing rows/content readable and never blocks the rest of the window.
pub fn loading_floating_badge(label: impl Into<SharedString>, p: Palette) -> AnyElement {
    let text: SharedString = label.into();
    h_flex()
        .gap(px(6.))
        .items_center()
        .px(px(10.))
        .py(px(4.))
        .rounded(px(999.))
        .bg(p.card_bg)
        .border_1()
        .border_color(p.border)
        .shadow_md()
        .child(loading_spinner(Size::XSmall, p.accent))
        .child(
            div()
                .text_size(px(11.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.text_secondary)
                .whitespace_nowrap()
                .child(text),
        )
        .into_any_element()
}

/// Scoped placeholder shown ONLY inside a specific empty panel while its initial data loads.
pub fn loading_pane_empty(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    p: Palette,
) -> AnyElement {
    let bar_id: ElementId = id.into();
    let text: SharedString = label.into();

    v_flex()
        .relative()
        .size_full()
        .min_h_0()
        .min_w_0()
        .bg(p.window_bg)
        .overflow_hidden()
        .child(loading_bar(bar_id, p))
        .child(
            v_flex()
                .flex_1()
                .min_h_0()
                .w_full()
                .p(px(14.))
                .gap(px(10.))
                .opacity(0.45)
                .children([0.85_f32, 0.72, 0.92, 0.64, 0.78].into_iter().map(|ratio| {
                    h_flex()
                        .w_full()
                        .gap(px(12.))
                        .items_center()
                        .child(Skeleton::new().h(px(11.)).w(relative(0.18)).rounded(px(4.)))
                        .child(
                            Skeleton::new()
                                .secondary()
                                .h(px(11.))
                                .w(relative(ratio * 0.45))
                                .rounded(px(4.)),
                        )
                        .child(
                            Skeleton::new()
                                .secondary()
                                .h(px(11.))
                                .w(relative(0.20))
                                .rounded(px(4.)),
                        )
                })),
        )
        .child(
            div()
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
                .child(loading_floating_badge(text, p)),
        )
        .into_any_element()
}

/// Universal scoped loading wrapper for any content/list/table/result panel.
///
/// - When `!is_loading`: renders `content` directly.
/// - When `is_loading && is_empty`: renders a subtle skeleton + centered pill inside ONLY this panel.
/// - When `is_loading && !is_empty`: keeps `content` visible and overlays only a 2px top progress
///   bar plus a compact top-right floating pill (never covers or freezes the whole window).
pub fn loading_pane(
    id: impl Into<ElementId>,
    is_loading: bool,
    is_empty: bool,
    label: impl Into<SharedString>,
    p: Palette,
    content: impl IntoElement,
) -> AnyElement {
    if !is_loading {
        return div()
            .size_full()
            .min_h_0()
            .min_w_0()
            .child(content)
            .into_any_element();
    }

    if is_empty {
        return loading_pane_empty(id, label, p);
    }

    div()
        .relative()
        .size_full()
        .min_h_0()
        .min_w_0()
        .child(content)
        .child(
            div()
                .absolute()
                .top_0()
                .left_0()
                .right_0()
                .child(loading_bar(id, p)),
        )
        .child(
            div()
                .absolute()
                .top(px(8.))
                .right(px(12.))
                .child(loading_floating_badge(label, p)),
        )
        .into_any_element()
}
