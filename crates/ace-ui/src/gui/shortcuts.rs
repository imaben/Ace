//! Cross-platform keyboard shortcut helpers (macOS `Cmd` vs Windows/Linux `Ctrl`).

use gpui_kit::{Action, KeyBinding};

/// Human-readable primary modifier label for tooltips and placeholders.
#[cfg(target_os = "macos")]
pub const MOD_LABEL: &str = "Cmd";

/// Human-readable primary modifier label for tooltips and placeholders.
#[cfg(not(target_os = "macos"))]
pub const MOD_LABEL: &str = "Ctrl";

/// Format a primary-modifier shortcut for UI display (e.g. `Cmd+R` on macOS, `Ctrl+R` on Windows/Linux).
pub fn mod_shortcut(key: &str) -> String {
    format!("{MOD_LABEL}+{key}")
}

/// Build a platform-aware [`KeyBinding`] using explicit `cmd-` on macOS and `ctrl-` on Windows/Linux.
///
/// Using explicit platform keys avoids `KeyBinding::new("secondary-...")`, which panics because
/// `KeyBinding::new` parses without a host OS context.
pub fn mod_binding<A: Action>(
    mac_key: &'static str,
    other_key: &'static str,
    action: A,
    context: Option<&str>,
) -> KeyBinding {
    let key = if cfg!(target_os = "macos") {
        mac_key
    } else {
        other_key
    };
    KeyBinding::new(key, action, context)
}

#[cfg(test)]
mod tests {
    use super::{mod_shortcut, MOD_LABEL};

    #[test]
    fn formats_platform_shortcut_label() {
        #[cfg(target_os = "macos")]
        {
            assert_eq!(MOD_LABEL, "Cmd");
            assert_eq!(mod_shortcut("R"), "Cmd+R");
            assert_eq!(mod_shortcut("Shift+R"), "Cmd+Shift+R");
        }
        #[cfg(not(target_os = "macos"))]
        {
            assert_eq!(MOD_LABEL, "Ctrl");
            assert_eq!(mod_shortcut("R"), "Ctrl+R");
            assert_eq!(mod_shortcut("Shift+R"), "Ctrl+Shift+R");
        }
    }
}
