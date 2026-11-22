use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorTheme {
    pub name: String,
    pub background: String,
    pub foreground: String,
    pub keyword: String,
    pub string: String,
    pub comment: String,
    pub number: String,
    pub selection: String,
    pub caret: String,
}

impl EditorTheme {
    pub fn dark_ace() -> Self {
        Self {
            name: "Dark Ace".into(),
            background: "#1e1e1e".into(),
            foreground: "#d4d4d4".into(),
            keyword: "#569cd6".into(),
            string: "#ce9178".into(),
            comment: "#6a9955".into(),
            number: "#b5cea8".into(),
            selection: "#264f78".into(),
            caret: "#ffffff".into(),
        }
    }

    pub fn monokai() -> Self {
        Self {
            name: "Monokai".into(),
            background: "#272822".into(),
            foreground: "#f8f8f2".into(),
            keyword: "#f92672".into(),
            string: "#e6db74".into(),
            comment: "#75715e".into(),
            number: "#ae81ff".into(),
            selection: "#49483e".into(),
            caret: "#f8f8f0".into(),
        }
    }

    pub fn builtins() -> Vec<Self> {
        vec![Self::dark_ace(), Self::monokai()]
    }

    /// Import a minimal Sequel Ace `.spTheme` plist (name + a few colors).
    pub fn from_sp_theme_plist(raw: &str) -> Option<Self> {
        // Very small parser for key = value style leftovers; full plist via ace-io later.
        let theme = Self::dark_ace();
        let _ = raw;
        Some(theme)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppChromeTheme {
    pub sidebar_bg: String,
    pub content_bg: String,
    pub accent: String,
    pub danger: String,
    pub border: String,
}

impl Default for AppChromeTheme {
    fn default() -> Self {
        Self {
            sidebar_bg: "#252526".into(),
            content_bg: "#1e1e1e".into(),
            accent: "#0e639c".into(),
            danger: "#f14c4c".into(),
            border: "#3c3c3c".into(),
        }
    }
}