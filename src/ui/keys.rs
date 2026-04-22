// OS-aware keycap labels. Mac renders ⌘·⇧·⌃·⌥ glyphs; other platforms
// fall back to spelled-out "Ctrl" / "Shift" / "Alt" like standard CLI help.

#[cfg(target_os = "macos")]
pub fn meta(key: &str) -> String {
    format!("⌘{key}")
}

#[cfg(not(target_os = "macos"))]
pub fn meta(key: &str) -> String {
    format!("Ctrl+{key}")
}

#[cfg(target_os = "macos")]
pub fn ctrl(key: &str) -> String {
    format!("⌃{key}")
}

#[cfg(not(target_os = "macos"))]
pub fn ctrl(key: &str) -> String {
    format!("Ctrl+{key}")
}

pub fn enter() -> &'static str {
    "↵"
}

#[cfg(target_os = "macos")]
pub fn up_meta() -> String {
    "⌘↑".to_string()
}

#[cfg(not(target_os = "macos"))]
pub fn up_meta() -> String {
    "Ctrl+↑".to_string()
}
