//! The click-through, always-on-top window that shows the ring.
//!
//! Each OS gets its own small presenter because per-pixel transparency
//! without focus stealing needs native window flags on every platform. A
//! presenter only displays finished frames; drawing happens in orbit-render.

use orbit_input::Point;
use orbit_render::Pixmap;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(all(unix, not(target_os = "macos")))]
mod x11;

pub trait Presenter {
    /// Device pixels per logical pixel on the monitor containing `at`
    /// (hook coordinates).
    fn scale_at(&self, at: Point) -> f64;
    /// Centres the window on `at` and shows `frame` (rendered at `scale`)
    /// without activating the window.
    fn show(&mut self, at: Point, frame: &Pixmap, scale: f64);
    /// Replaces the visible frame (same size).
    fn update(&mut self, frame: &Pixmap);
    fn hide(&mut self);
}

pub fn create() -> Option<Box<dyn Presenter>> {
    #[cfg(target_os = "macos")]
    return macos::MacPresenter::new().map(|p| Box::new(p) as Box<dyn Presenter>);
    #[cfg(target_os = "windows")]
    return windows::WinPresenter::new().map(|p| Box::new(p) as Box<dyn Presenter>);
    #[cfg(all(unix, not(target_os = "macos")))]
    return x11::X11Presenter::new().map(|p| Box::new(p) as Box<dyn Presenter>);
}
