//! Sequel Ace design tokens — single source of truth for every screen.

use gpui_kit::{px, rgb, Pixels, Rgba};

/// Colour tokens (macOS dark / Sequel Ace).
#[derive(Debug, Clone, Copy)]
pub struct ColorTokens {
    pub window: Rgba,
    pub sidebar: Rgba,
    pub toolbar: Rgba,
    pub card: Rgba,
    pub field: Rgba,
    pub border: Rgba,
    pub divider: Rgba,
    pub label: Rgba,
    pub muted: Rgba,
    pub disabled: Rgba,
    pub accent: Rgba,
    pub accent_fg: Rgba,
    pub danger: Rgba,
    pub selected: Rgba,
    pub zebra: Rgba,
    pub header: Rgba,
}

impl ColorTokens {
    pub fn dark() -> Self {
        Self {
            window: rgb(0x1e1e1e),
            sidebar: rgb(0x252526),
            toolbar: rgb(0x2a2a2c),
            card: rgb(0x2c2c2e),
            field: rgb(0x3a3a3c),
            border: rgb(0x3f3f42),
            divider: rgb(0x3f3f42),
            label: rgb(0xe5e5ea),
            muted: rgb(0x8e8e93),
            disabled: rgb(0x636366),
            accent: rgb(0x0a84ff),
            accent_fg: rgb(0xffffff),
            danger: rgb(0xff453a),
            selected: rgb(0x3a3a3c),
            zebra: rgb(0x232325),
            header: rgb(0x333336),
        }
    }
}

/// Spacing scale.
#[derive(Debug, Clone, Copy)]
pub struct SpaceTokens {
    pub xxs: Pixels,
    pub xs: Pixels,
    pub sm: Pixels,
    pub md: Pixels,
    pub lg: Pixels,
    pub xl: Pixels,
}

impl SpaceTokens {
    pub fn dark() -> Self {
        Self {
            xxs: px(2.),
            xs: px(4.),
            sm: px(8.),
            md: px(12.),
            lg: px(16.),
            xl: px(24.),
        }
    }
}

/// Layout metrics matching Sequel Ace XIB defaults.
#[derive(Debug, Clone, Copy)]
pub struct LayoutTokens {
    pub sidebar_width: Pixels,
    pub tables_sidebar_width: Pixels,
    pub connection_card_width: Pixels,
    pub form_label_width: Pixels,
    pub toolbar_height: Pixels,
    pub row_height: Pixels,
    pub list_row_height: Pixels,
    pub header_height: Pixels,
    pub footer_height: Pixels,
    pub control_height: Pixels,
    pub radius_sm: Pixels,
    pub radius_md: Pixels,
}

impl LayoutTokens {
    pub fn dark() -> Self {
        Self {
            sidebar_width: px(200.),
            tables_sidebar_width: px(214.),
            connection_card_width: px(446.),
            form_label_width: px(98.),
            toolbar_height: px(40.),
            row_height: px(22.),
            list_row_height: px(24.),
            header_height: px(26.),
            footer_height: px(26.),
            control_height: px(28.),
            radius_sm: px(4.),
            radius_md: px(8.),
        }
    }
}

/// Global design tokens used by every component.
#[derive(Debug, Clone, Copy)]
pub struct Tokens {
    pub color: ColorTokens,
    pub space: SpaceTokens,
    pub layout: LayoutTokens,
}

impl Tokens {
    pub fn dark() -> Self {
        Self {
            color: ColorTokens::dark(),
            space: SpaceTokens::dark(),
            layout: LayoutTokens::dark(),
        }
    }
}

/// Active tokens for the running UI (always dark SA for now).
pub fn tokens() -> Tokens {
    Tokens::dark()
}
