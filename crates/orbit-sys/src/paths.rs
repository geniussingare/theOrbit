//! Where TheOrbit keeps its files.

use std::path::PathBuf;

/// `~/Library/Application Support/TheOrbit` on macOS, `%APPDATA%\TheOrbit` on
/// Windows, `~/.config/TheOrbit` on Linux.
pub fn config_dir() -> PathBuf {
    dirs::config_dir().unwrap_or_else(std::env::temp_dir).join("TheOrbit")
}

pub fn config_file() -> PathBuf {
    config_dir().join("config.json")
}

/// Written by the resident app, read by settings.
pub fn status_file() -> PathBuf {
    config_dir().join("status.json")
}

pub fn lock_file(name: &str) -> PathBuf {
    config_dir().join(format!("{name}.lock"))
}

/// A sibling executable shipped next to the current one (e.g. the settings
/// binary inside `TheOrbit.app/Contents/MacOS`).
pub fn sibling_exe(name: &str) -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let file = if cfg!(windows) { format!("{name}.exe") } else { name.to_string() };
    let path = exe.parent()?.join(file);
    path.exists().then_some(path)
}
