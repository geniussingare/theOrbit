//! Runs the action behind an orbit item.

use orbit_core::{ActionType, OrbitItem};
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// Runs `item`. Call it from the main thread: on macOS, enigo's keyboard
/// layout lookups for shortcuts must happen there.
pub fn run(item: &OrbitItem) -> Result<(), String> {
    let value = item.value.trim();
    if value.is_empty() {
        return Err(format!("\"{}\" has nothing to run", item.title));
    }
    match item.kind {
        ActionType::Url => open::that_detached(value).map_err(|e| e.to_string()),
        ActionType::File => open::that_detached(expand_tilde(value)).map_err(|e| e.to_string()),
        ActionType::App => launch_app(value),
        ActionType::Shell => run_shell(value),
        ActionType::Shortcut => send_shortcut(&parse_shortcut(value)?),
    }
}

pub fn expand_tilde(path: &str) -> PathBuf {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"));
    match (path.strip_prefix('~'), home) {
        (Some(rest), Some(home)) => {
            let mut p = PathBuf::from(home);
            let rest = rest.trim_start_matches(['/', '\\']);
            if !rest.is_empty() {
                p.push(rest);
            }
            p
        }
        _ => PathBuf::from(path),
    }
}

fn spawn(mut cmd: Command) -> Result<(), String> {
    cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd.spawn().map(|_| ()).map_err(|e| e.to_string())
}

fn launch_app(value: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let mut cmd = Command::new("open");
        if value.starts_with('/') || value.starts_with('~') {
            cmd.arg(expand_tilde(value));
        } else if value.contains('.') && !value.contains(' ') && !value.ends_with(".app") {
            cmd.args(["-b", value]); // bundle identifier, e.g. com.apple.Safari
        } else {
            cmd.args(["-a", value]); // application name, e.g. "Visual Studio Code"
        }
        spawn(cmd)
    }
    #[cfg(target_os = "windows")]
    {
        // `start` resolves exe names, App Paths entries (msedge) and .lnk files.
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", "start", ""]).arg(expand_tilde(value));
        spawn(cmd)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        if value.ends_with(".desktop") {
            let path = expand_tilde(value);
            if path.is_absolute() && path.exists() {
                let mut cmd = Command::new("gio");
                cmd.arg("launch").arg(path);
                return spawn(cmd);
            }
            let mut cmd = Command::new("gtk-launch");
            cmd.arg(value);
            return spawn(cmd);
        }
        run_shell(value)
    }
}

fn run_shell(command: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", command]);
        spawn(cmd)
    }
    #[cfg(not(target_os = "windows"))]
    {
        // A login shell picks up the user's PATH even when we were started from the Dock.
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
        let mut cmd = Command::new(shell);
        cmd.args(["-lc", command]);
        spawn(cmd)
    }
}

#[derive(Debug, Clone)]
pub struct Combo {
    modifiers: Vec<enigo::Key>,
    key: enigo::Key,
}

/// Parses strings like `cmd+shift+4`, `ctrl+alt+t`, `win+shift+s` or `f5`.
pub fn parse_shortcut(text: &str) -> Result<Combo, String> {
    use enigo::Key;
    let mut modifiers = Vec::new();
    let mut key = None;
    for part in text.split('+').map(|p| p.trim().to_lowercase()).filter(|p| !p.is_empty()) {
        let k = match part.as_str() {
            "cmd" | "command" | "meta" | "super" | "win" | "windows" => {
                modifiers.push(Key::Meta);
                continue;
            }
            "ctrl" | "control" => {
                modifiers.push(Key::Control);
                continue;
            }
            "alt" | "opt" | "option" => {
                modifiers.push(Key::Alt);
                continue;
            }
            "shift" => {
                modifiers.push(Key::Shift);
                continue;
            }
            "enter" | "return" => Key::Return,
            "tab" => Key::Tab,
            "space" => Key::Space,
            "esc" | "escape" => Key::Escape,
            "backspace" => Key::Backspace,
            "delete" | "del" => Key::Delete,
            "up" => Key::UpArrow,
            "down" => Key::DownArrow,
            "left" => Key::LeftArrow,
            "right" => Key::RightArrow,
            "home" => Key::Home,
            "end" => Key::End,
            "pageup" => Key::PageUp,
            "pagedown" => Key::PageDown,
            #[cfg(not(target_os = "macos"))]
            "print" | "printscreen" | "prtsc" => Key::PrintScr,
            "f1" => Key::F1,
            "f2" => Key::F2,
            "f3" => Key::F3,
            "f4" => Key::F4,
            "f5" => Key::F5,
            "f6" => Key::F6,
            "f7" => Key::F7,
            "f8" => Key::F8,
            "f9" => Key::F9,
            "f10" => Key::F10,
            "f11" => Key::F11,
            "f12" => Key::F12,
            "plus" => Key::Unicode('+'),
            single if single.chars().count() == 1 => Key::Unicode(single.chars().next().unwrap()),
            other => return Err(format!("unknown key \"{other}\" in shortcut \"{text}\"")),
        };
        if key.replace(k).is_some() {
            return Err(format!("shortcut \"{text}\" has more than one non-modifier key"));
        }
    }
    let key = key.ok_or_else(|| format!("shortcut \"{text}\" has no key"))?;
    Ok(Combo { modifiers, key })
}

fn send_shortcut(combo: &Combo) -> Result<(), String> {
    use enigo::{Direction, Enigo, Keyboard, Settings};
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
    for m in &combo.modifiers {
        enigo.key(*m, Direction::Press).map_err(|e| e.to_string())?;
    }
    let result = enigo.key(combo.key, Direction::Click).map_err(|e| e.to_string());
    for m in combo.modifiers.iter().rev() {
        let _ = enigo.key(*m, Direction::Release);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_shortcuts() {
        let c = parse_shortcut("cmd+shift+4").unwrap();
        assert_eq!(c.modifiers.len(), 2);
        assert_eq!(c.key, enigo::Key::Unicode('4'));
        assert_eq!(parse_shortcut("F5").unwrap().key, enigo::Key::F5);
        assert!(parse_shortcut("ctrl+shift").is_err());
        assert!(parse_shortcut("a+b").is_err());
        assert!(parse_shortcut("ctrl+banana").is_err());
    }

    #[test]
    fn expands_home() {
        std::env::set_var("HOME", "/home/me");
        assert_eq!(expand_tilde("~/Downloads"), PathBuf::from("/home/me/Downloads"));
        assert_eq!(expand_tilde("~"), PathBuf::from("/home/me"));
        assert_eq!(expand_tilde("/tmp"), PathBuf::from("/tmp"));
    }
}
