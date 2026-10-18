//! Hook status, shared with the settings window through a small JSON file.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HookStatus {
    pub running: bool,
    pub needs_permission: bool,
    pub error: Option<String>,
    /// Process id of the resident app, so settings can tell if it is alive.
    pub pid: u32,
}

impl HookStatus {
    pub fn write(&self) {
        let path = crate::paths::status_file();
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(text) = serde_json::to_string(self) {
            let _ = std::fs::write(path, text);
        }
    }

    pub fn read() -> Option<Self> {
        let text = std::fs::read_to_string(crate::paths::status_file()).ok()?;
        serde_json::from_str(&text).ok()
    }
}
