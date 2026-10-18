//! Global right-button long-press detection.
//!
//! Each OS backend only translates native mouse events into calls on the
//! shared [`Engine`] and knows how to re-inject a right click. All of the
//! "short click vs. hold vs. drag" logic lives in [`engine`].

mod engine;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(all(unix, not(target_os = "macos"), not(target_os = "android"), not(target_os = "ios")))]
mod x11;

pub use engine::{Engine, Output, Point, Replay, Verdict};

/// Marker stamped on events we inject ourselves, so the hook lets them through.
#[allow(dead_code)]
pub(crate) const SYNTHETIC_MARKER: i64 = 0x0B17_0B17;

#[derive(Debug)]
pub enum StartError {
    /// The OS has not granted the permission needed to watch the mouse
    /// (macOS Accessibility).
    PermissionDenied,
    /// Global hooks cannot work in this session (e.g. Linux on Wayland).
    Unsupported(String),
    Failed(String),
}

impl std::fmt::Display for StartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PermissionDenied => write!(f, "permission to monitor the mouse was not granted"),
            Self::Unsupported(why) => write!(f, "not supported: {why}"),
            Self::Failed(why) => write!(f, "{why}"),
        }
    }
}

/// Whether the process may install the hook. On macOS this checks
/// Accessibility access, showing the system prompt when `prompt` is true.
pub fn has_permission(prompt: bool) -> bool {
    #[cfg(target_os = "macos")]
    {
        macos::is_trusted(prompt)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = prompt;
        true
    }
}

/// Installs the global hook on a dedicated thread and returns once it is
/// running (or failed to start). `make_engine` receives the platform's
/// replay implementation and builds the engine the hook will drive.
pub fn start<F>(make_engine: F) -> Result<std::sync::Arc<Engine>, StartError>
where
    F: FnOnce(Box<dyn Replay>) -> std::sync::Arc<Engine>,
{
    #[cfg(target_os = "macos")]
    return macos::start(make_engine);
    #[cfg(target_os = "windows")]
    return windows::start(make_engine);
    #[cfg(all(unix, not(target_os = "macos"), not(target_os = "android"), not(target_os = "ios")))]
    return x11::start(make_engine);
    #[allow(unreachable_code)]
    {
        let _ = make_engine;
        Err(StartError::Unsupported("no global mouse hook on this platform".into()))
    }
}
