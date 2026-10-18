//! User configuration, stored as JSON in the OS config directory.

use serde::{Deserialize, Serialize};
use std::{fs, io, path::Path};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionType {
    /// Launch an application (path, bundle id, exe name or .desktop file).
    App,
    /// Open a URL in the default handler.
    Url,
    /// Open a file or folder.
    File,
    /// Run a shell command.
    Shell,
    /// Send a keyboard shortcut such as `cmd+shift+4`.
    Shortcut,
}

/// Look of the ring. There is one theme today; the setting exists so more
/// can be added later. Unknown names (e.g. older "thor"/"classic") fall back
/// to the default.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RingTheme {
    #[default]
    #[serde(other)]
    Orbit,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrbitItem {
    #[serde(default = "new_id")]
    pub id: String,
    pub title: String,
    #[serde(rename = "type")]
    pub kind: ActionType,
    pub value: String,
    /// Optional 1–2 letter monogram shown instead of the auto-detected icon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

impl OrbitItem {
    pub fn new(title: &str, kind: ActionType, value: &str) -> Self {
        Self { id: new_id(), title: title.into(), kind, value: value.into(), icon: None }
    }

    /// The custom monogram, if it is usable: 1–2 letters or digits.
    pub fn monogram(&self) -> Option<String> {
        let text = self.icon.as_deref()?.trim();
        let ok = !text.is_empty() && text.chars().count() <= 2 && text.chars().all(char::is_alphanumeric);
        ok.then(|| text.to_uppercase())
    }
}

fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OrbitConfig {
    pub enabled: bool,
    /// How long the right button must be held before the orbit opens.
    pub hold_ms: u64,
    /// Outer radius of the ring, in logical pixels.
    pub radius: f64,
    /// Releasing inside this radius cancels.
    pub dead_zone: f64,
    /// Moving further than this before the timer fires = normal right-drag.
    pub drag_threshold: f64,
    pub theme: RingTheme,
    /// A soft tick when an item is chosen.
    pub sound: bool,
    pub items: Vec<OrbitItem>,
}

impl Default for OrbitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            hold_ms: 350,
            radius: 150.0,
            dead_zone: 42.0,
            drag_threshold: 8.0,
            theme: RingTheme::Orbit,
            sound: true,
            items: default_items(),
        }
    }
}

impl OrbitConfig {
    /// Loads the config, writing the defaults if the file does not exist yet.
    pub fn load_or_create(path: &Path) -> io::Result<Self> {
        match fs::read_to_string(path) {
            Ok(text) => {
                let mut cfg: Self = serde_json::from_str(&text).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                cfg.sanitize();
                Ok(cfg)
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                let cfg = Self::default();
                cfg.save(path)?;
                Ok(cfg)
            }
            Err(e) => Err(e),
        }
    }

    /// Drops custom icons that are not a 1–2 letter monogram (older versions
    /// stored emoji, which the ring's font cannot draw).
    pub fn sanitize(&mut self) {
        for item in &mut self.items {
            if item.monogram().is_none() {
                item.icon = None;
            }
        }
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let text = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
        // Write-then-rename so a crash never leaves a half-written config.
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, text)?;
        fs::rename(tmp, path)
    }
}

fn default_items() -> Vec<OrbitItem> {
    use ActionType::*;
    if cfg!(target_os = "macos") {
        vec![
            OrbitItem::new("Finder", App, "/System/Library/CoreServices/Finder.app"),
            OrbitItem::new("Safari", App, "/Applications/Safari.app"),
            OrbitItem::new("Terminal", App, "/System/Applications/Utilities/Terminal.app"),
            OrbitItem::new("Notes", App, "/System/Applications/Notes.app"),
            OrbitItem::new("Screenshot", Shortcut, "cmd+shift+4"),
            OrbitItem::new("Downloads", File, "~/Downloads"),
            OrbitItem::new("Settings", App, "/System/Applications/System Settings.app"),
            OrbitItem::new("Google", Url, "https://www.google.com"),
        ]
    } else if cfg!(target_os = "windows") {
        vec![
            OrbitItem::new("Explorer", App, "explorer.exe"),
            OrbitItem::new("Edge", App, "msedge"),
            OrbitItem::new("Terminal", App, "wt"),
            OrbitItem::new("Notepad", App, "notepad.exe"),
            OrbitItem::new("Snip", Shortcut, "win+shift+s"),
            OrbitItem::new("Downloads", File, "~/Downloads"),
            OrbitItem::new("Settings", Url, "ms-settings:"),
            OrbitItem::new("Google", Url, "https://www.google.com"),
        ]
    } else {
        vec![
            OrbitItem::new("Home", File, "~"),
            OrbitItem::new("Browser", Url, "https://www.google.com"),
            OrbitItem::new("Terminal", Shell, "x-terminal-emulator || gnome-terminal || konsole || xterm"),
            OrbitItem::new("Editor", Shell, "gnome-text-editor || gedit || kate || xdg-open ~"),
            OrbitItem::new("Downloads", File, "~/Downloads"),
            OrbitItem::new("Screenshot", Shortcut, "print"),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_defaults() {
        let dir = std::env::temp_dir().join(format!("orbit-test-{}", new_id()));
        let path = dir.join("config.json");
        let created = OrbitConfig::load_or_create(&path).unwrap();
        assert!(!created.items.is_empty());
        let loaded = OrbitConfig::load_or_create(&path).unwrap();
        assert_eq!(created, loaded);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn emoji_icons_are_dropped_on_load() {
        let dir = std::env::temp_dir().join(format!("orbit-test-{}", new_id()));
        let path = dir.join("config.json");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            &path,
            r#"{"theme":"thor","items":[{"title":"A","type":"url","value":"x","icon":"📸"},{"title":"B","type":"url","value":"y","icon":"vs"}]}"#,
        )
        .unwrap();
        let cfg = OrbitConfig::load_or_create(&path).unwrap();
        assert_eq!(cfg.items[0].icon, None);
        assert_eq!(cfg.items[1].monogram().as_deref(), Some("VS"));
        assert_eq!(cfg.theme, RingTheme::Orbit, "old theme names fall back to Orbit");
        assert!(cfg.sound);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn missing_fields_fall_back_to_defaults() {
        let cfg: OrbitConfig =
            serde_json::from_str(r#"{"holdMs":500,"items":[{"title":"X","type":"url","value":"https://x.com"}]}"#).unwrap();
        assert_eq!(cfg.hold_ms, 500);
        assert_eq!(cfg.radius, 150.0);
        assert!(cfg.enabled);
        assert!(!cfg.items[0].id.is_empty());
    }
}
