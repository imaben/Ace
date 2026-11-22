//! Sequel Ace minimalist design system — Light & Dark macOS palettes.

use gpui_kit::component::{ActiveTheme as _, Theme, ThemeMode};
use gpui_kit::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    pub is_dark: bool,
    pub window_bg: Rgba,
    pub sidebar_bg: Rgba,
    pub toolbar_bg: Rgba,
    pub tabbar_bg: Rgba,
    pub card_bg: Rgba,
    pub surface_subtle: Rgba,
    pub border: Rgba,
    pub hairline: Rgba,
    pub text: Rgba,
    pub text_secondary: Rgba,
    pub text_muted: Rgba,
    pub text_disabled: Rgba,
    pub accent: Rgba,
    pub accent_hover: Rgba,
    pub accent_fg: Rgba,
    pub accent_subtle: Rgba,
    pub selected_bg: Rgba,
    pub hover_bg: Rgba,
    pub zebra_bg: Rgba,
    pub header_bg: Rgba,
    pub danger: Rgba,
    pub danger_subtle: Rgba,
    pub success: Rgba,
    pub warning: Rgba,
    pub null_fg: Rgba,
    pub code_bg: Rgba,
}

impl Palette {
    pub fn dark() -> Self {
        Self {
            is_dark: true,
            window_bg: rgb(0x1c1c1e),
            sidebar_bg: rgb(0x232326),
            toolbar_bg: rgb(0x262629),
            tabbar_bg: rgb(0x1f1f22),
            card_bg: rgb(0x232326),
            surface_subtle: rgb(0x2d2d30),
            border: rgb(0x3a3a3d),
            hairline: rgb(0x323235),
            text: rgb(0xebebf0),
            text_secondary: rgb(0xbcbcc2),
            text_muted: rgb(0x8e8e93),
            text_disabled: rgb(0x5c5c60),
            accent: rgb(0x0a84ff),
            accent_hover: rgb(0x2894ff),
            accent_fg: rgb(0xffffff),
            accent_subtle: rgba(0x0a84ff24),
            selected_bg: rgba(0x0a84ff33),
            hover_bg: rgba(0xffffff0d),
            zebra_bg: rgb(0x212124),
            header_bg: rgb(0x28282b),
            danger: rgb(0xff453a),
            danger_subtle: rgba(0xff453a20),
            success: rgb(0x30d158),
            warning: rgb(0xffd60a),
            null_fg: rgb(0x787880),
            code_bg: rgb(0x18181a),
        }
    }

    pub fn light() -> Self {
        Self {
            is_dark: false,
            window_bg: rgb(0xffffff),
            sidebar_bg: rgb(0xf5f5f7),
            toolbar_bg: rgb(0xf6f6f7),
            tabbar_bg: rgb(0xededef),
            card_bg: rgb(0xfafafb),
            surface_subtle: rgb(0xf0f0f2),
            border: rgb(0xdcdcde),
            hairline: rgb(0xe7e7e9),
            text: rgb(0x1d1d1f),
            text_secondary: rgb(0x48484a),
            text_muted: rgb(0x6e6e73),
            text_disabled: rgb(0xaeaeb2),
            accent: rgb(0x007aff),
            accent_hover: rgb(0x0066d6),
            accent_fg: rgb(0xffffff),
            accent_subtle: rgba(0x007aff1a),
            selected_bg: rgba(0x007aff24),
            hover_bg: rgba(0x00000008),
            zebra_bg: rgb(0xf9f9fb),
            header_bg: rgb(0xf3f3f5),
            danger: rgb(0xd70015),
            danger_subtle: rgba(0xd7001516),
            success: rgb(0x248a3d),
            warning: rgb(0xb25000),
            null_fg: rgb(0x98989d),
            code_bg: rgb(0xf5f5f7),
        }
    }

    pub fn for_app(cx: &App) -> Self {
        if cx.theme().is_dark() {
            Self::dark()
        } else {
            Self::light()
        }
    }
}

pub fn apply_theme(dark: bool, window: Option<&mut Window>, cx: &mut App) {
    let mode = if dark {
        ThemeMode::Dark
    } else {
        ThemeMode::Light
    };
    Theme::change(mode, window, cx);
    // Crisp macOS-style input & checkbox borders on cards (via Theme::update so gpui-base stays in sync)
    let input_border: Hsla = if dark {
        rgb(0x52525a).into()
    } else {
        rgb(0xc7c7cc).into()
    };
    Theme::update(cx, |theme| {
        theme.colors.input = input_border;
        theme.tokens.input = input_border.into();
    });
}

pub fn favorite_tint(color_index: i32) -> Option<Rgba> {
    ace_core::favorite_color_rgb(color_index)
        .map(|(r, g, b)| rgb(((r as u32) << 16) | ((g as u32) << 8) | (b as u32)))
}
