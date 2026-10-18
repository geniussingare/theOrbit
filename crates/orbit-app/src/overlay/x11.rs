//! Linux/X11 presenter: an override-redirect window with a 32-bit ARGB visual
//! (a compositor provides the transparency) and an empty input shape so
//! clicks pass through.

use super::Presenter;
use orbit_input::Point;
use orbit_render::Pixmap;
use x11rb::connection::{Connection, RequestConnection as _};
use x11rb::protocol::shape::{self, ConnectionExt as _};
use x11rb::protocol::xproto::{
    ClipOrdering, ColormapAlloc, ConfigureWindowAux, ConnectionExt as _, CreateGCAux, CreateWindowAux, EventMask,
    Gcontext, ImageFormat, StackMode, VisualClass, Window, WindowClass,
};
use x11rb::rust_connection::RustConnection;

pub struct X11Presenter {
    conn: RustConnection,
    window: Window,
    gc: Gcontext,
    size: u32,
    pos: (i32, i32),
}

impl X11Presenter {
    pub fn new() -> Option<Self> {
        let (conn, screen_num) = RustConnection::connect(None).ok()?;
        let screen = &conn.setup().roots[screen_num];
        let root = screen.root;
        let visual = screen
            .allowed_depths
            .iter()
            .filter(|d| d.depth == 32)
            .flat_map(|d| d.visuals.iter())
            .find(|v| v.class == VisualClass::TRUE_COLOR)?
            .visual_id;

        let colormap = conn.generate_id().ok()?;
        conn.create_colormap(ColormapAlloc::NONE, colormap, root, visual).ok()?;
        let window = conn.generate_id().ok()?;
        let aux = CreateWindowAux::new()
            .background_pixel(0)
            .border_pixel(0)
            .override_redirect(1)
            .colormap(colormap)
            .event_mask(EventMask::EXPOSURE);
        conn.create_window(32, window, root, 0, 0, 10, 10, 0, WindowClass::INPUT_OUTPUT, visual, &aux).ok()?;
        // Empty input region: the pointer goes straight through.
        conn.shape_rectangles(shape::SO::SET, shape::SK::INPUT, ClipOrdering::UNSORTED, window, 0, 0, &[]).ok()?;
        let gc = conn.generate_id().ok()?;
        conn.create_gc(gc, window, &CreateGCAux::new()).ok()?;
        conn.flush().ok()?;
        Some(Self { conn, window, gc, size: 0, pos: (0, 0) })
    }

    fn blit(&self, frame: &Pixmap) {
        let w = frame.width();
        let bgra = orbit_render::to_bgra(frame);
        // Split into requests that fit the server's maximum request size.
        let max_bytes = self.conn.maximum_request_bytes().saturating_sub(64).max(4096);
        let rows_per_req = (max_bytes / (w as usize * 4)).max(1);
        for (i, chunk) in bgra.chunks(rows_per_req * w as usize * 4).enumerate() {
            let rows = chunk.len() / (w as usize * 4);
            let y = (i * rows_per_req) as i16;
            let _ = self.conn.put_image(ImageFormat::Z_PIXMAP, self.window, self.gc, w as u16, rows as u16, 0, y, 0, 32, chunk);
        }
        let _ = self.conn.flush();
    }
}

impl Presenter for X11Presenter {
    fn scale_at(&self, _at: Point) -> f64 {
        // X11 has no per-monitor scale; honour GDK_SCALE if the desktop sets it.
        std::env::var("GDK_SCALE").ok().and_then(|s| s.parse().ok()).unwrap_or(1.0)
    }

    fn show(&mut self, at: Point, frame: &Pixmap, _scale: f64) {
        let side = frame.width();
        let half = side as f64 / 2.0;
        self.pos = ((at.x - half).round() as i32, (at.y - half).round() as i32);
        self.size = side;
        let aux = ConfigureWindowAux::new()
            .x(self.pos.0)
            .y(self.pos.1)
            .width(side)
            .height(side)
            .stack_mode(StackMode::ABOVE);
        let _ = self.conn.configure_window(self.window, &aux);
        let _ = self.conn.map_window(self.window);
        // Make sure the window is mapped before drawing into it.
        let _ = self.conn.get_input_focus().map(|c| c.reply());
        self.blit(frame);
    }

    fn update(&mut self, frame: &Pixmap) {
        self.blit(frame);
    }

    fn hide(&mut self) {
        let _ = self.conn.unmap_window(self.window);
        let _ = self.conn.flush();
    }
}
