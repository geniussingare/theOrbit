//! Linux backend for X11 sessions.
//!
//! A passive grab on button 3 of the root window gives us every right press
//! before applications see it. Clicks we decide not to keep are re-injected
//! through the XTEST extension with the grab temporarily lifted.
//!
//! Wayland does not allow global pointer grabs, so it is rejected up front.

use crate::{Engine, Point, Replay, StartError};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use x11rb::connection::Connection;
use x11rb::protocol::xinput::{self, ConnectionExt as _};
use x11rb::protocol::xproto::{
    ButtonIndex, ConnectionExt as _, EventMask, GrabMode, ModMask, Window, BUTTON_PRESS_EVENT, BUTTON_RELEASE_EVENT,
};
use x11rb::protocol::xtest::ConnectionExt as _;
use x11rb::protocol::Event;
use x11rb::rust_connection::RustConnection;
use x11rb::{CURRENT_TIME, NONE};

static ENGINE: OnceLock<Arc<Engine>> = OnceLock::new();

struct X11 {
    conn: RustConnection,
    root: Window,
    /// Set after we handed a right-drag to apps; the next physical release
    /// must be mirrored by an XTEST release.
    forwarding: AtomicBool,
}

impl X11 {
    fn grab(&self) {
        let mask = EventMask::BUTTON_PRESS | EventMask::BUTTON_RELEASE | EventMask::BUTTON_MOTION;
        let _ = self.conn.grab_button(
            false,
            self.root,
            mask,
            GrabMode::ASYNC,
            GrabMode::ASYNC,
            NONE,
            NONE,
            ButtonIndex::M3,
            ModMask::ANY,
        );
        let _ = self.conn.flush();
    }

    fn ungrab(&self) {
        let _ = self.conn.ungrab_pointer(CURRENT_TIME);
        let _ = self.conn.ungrab_button(ButtonIndex::M3, self.root, ModMask::ANY);
        let _ = self.conn.flush();
    }

    fn fake(&self, kind: u8) {
        let _ = self.conn.xtest_fake_input(kind, 3, CURRENT_TIME, self.root, 0, 0, 0);
    }

    fn sync(&self) {
        // A round trip guarantees the server processed everything before we re-grab.
        let _ = self.conn.get_input_focus().map(|c| c.reply());
    }
}

struct X11Replay(Arc<X11>);

impl Replay for X11Replay {
    fn click(&self, _at: Point) {
        let x = &self.0;
        x.ungrab();
        x.fake(BUTTON_PRESS_EVENT);
        x.fake(BUTTON_RELEASE_EVENT);
        x.sync();
        x.grab();
    }
    fn press(&self, _at: Point) {
        let x = &self.0;
        x.ungrab();
        x.fake(BUTTON_PRESS_EVENT);
        x.forwarding.store(true, Ordering::Release);
        x.sync();
    }
}

pub fn start<F>(make_engine: F) -> Result<Arc<Engine>, StartError>
where
    F: FnOnce(Box<dyn Replay>) -> Arc<Engine>,
{
    if let Some(engine) = ENGINE.get() {
        return Ok(engine.clone());
    }
    let on_wayland = std::env::var("XDG_SESSION_TYPE").map(|s| s == "wayland").unwrap_or(false)
        || std::env::var_os("WAYLAND_DISPLAY").is_some();
    if on_wayland {
        return Err(StartError::Unsupported(
            "Wayland sessions do not allow global mouse hooks; log in with an X11 session".into(),
        ));
    }

    let (conn, screen) = RustConnection::connect(None).map_err(|e| StartError::Failed(e.to_string()))?;
    let root = conn.setup().roots[screen].root;
    conn.xtest_get_version(2, 2)
        .map_err(|e| StartError::Failed(e.to_string()))?
        .reply()
        .map_err(|_| StartError::Unsupported("the XTEST extension is missing".into()))?;
    // XInput2 raw events still arrive while another client owns the pointer,
    // which is how we notice the end of a forwarded right-drag.
    let xi2 = conn
        .xinput_xi_query_version(2, 0)
        .ok()
        .and_then(|c| c.reply().ok())
        .is_some();
    if xi2 {
        let _ = conn.xinput_xi_select_events(
            root,
            &[xinput::EventMask {
                deviceid: xinput::Device::ALL.into(),
                mask: vec![xinput::XIEventMask::RAW_BUTTON_RELEASE],
            }],
        );
    }

    let x = Arc::new(X11 { conn, root, forwarding: AtomicBool::new(false) });
    let engine = ENGINE.get_or_init(|| make_engine(Box::new(X11Replay(x.clone())))).clone();
    x.grab();

    let e = engine.clone();
    std::thread::Builder::new()
        .name("orbit-x11-grab".into())
        .spawn(move || loop {
            let Ok(event) = x.conn.wait_for_event() else {
                log::error!("lost the X11 connection; the orbit is disabled");
                return;
            };
            match event {
                Event::ButtonPress(ev) if ev.detail == 3 => {
                    e.on_down(Point::new(ev.root_x as f64, ev.root_y as f64));
                }
                Event::MotionNotify(ev) => {
                    e.on_drag(Point::new(ev.root_x as f64, ev.root_y as f64));
                }
                Event::ButtonRelease(ev) if ev.detail == 3 => {
                    if !x.forwarding.load(Ordering::Acquire) {
                        e.on_up(Point::new(ev.root_x as f64, ev.root_y as f64));
                    }
                }
                Event::XinputRawButtonRelease(ev) if ev.detail == 3 && x.forwarding.swap(false, Ordering::AcqRel) => {
                    x.fake(BUTTON_RELEASE_EVENT);
                    x.sync();
                    x.grab();
                    e.on_up(Point::new(0.0, 0.0));
                }
                _ => {}
            }
        })
        .map_err(|e| StartError::Failed(e.to_string()))?;

    Ok(engine)
}
