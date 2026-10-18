//! TheOrbit settings: a minimal native window (egui) that edits `config.json`.
//!
//! One calm column: the live ring is the hero, items are a grid of tiles,
//! everything else hides behind "Preferences". Editing happens in a sheet
//! that slides up from the bottom. The window is a separate process started
//! from the tray, so it costs nothing while closed.

#![cfg_attr(windows, windows_subsystem = "windows")]

use eframe::egui::{self, pos2, Color32, Id, Pos2, Rect, RichText, Sense, Stroke, TextureHandle, Vec2};
use orbit_core::{ActionType, OrbitConfig, OrbitItem, RingTheme};
use orbit_render::anim::{self, HoverTween, Pose};
use orbit_render::{logo, Font, Ring, RingCache, Segment, Theme};
use orbit_sys::{actions, icons, paths, status::HookStatus};
use std::collections::HashMap;
use std::time::{Duration, Instant};

// Neutral, storm-dark palette; the accent follows the ring theme.
const BG: Color32 = Color32::from_rgb(11, 14, 20);
const SURFACE: Color32 = Color32::from_rgb(20, 25, 35);
const SURFACE_2: Color32 = Color32::from_rgb(28, 34, 47);
const BORDER: Color32 = Color32::from_rgb(38, 45, 60);
const TEXT: Color32 = Color32::from_rgb(236, 242, 250);
const MUTED: Color32 = Color32::from_rgb(147, 160, 180);
const OK: Color32 = Color32::from_rgb(80, 210, 150);
const WARN: Color32 = Color32::from_rgb(240, 180, 70);
const BAD: Color32 = Color32::from_rgb(242, 104, 114);

const CONTENT_WIDTH: f32 = 520.0;
const PREVIEW_RADIUS: f32 = 150.0;
const TILE: Vec2 = Vec2::new(92.0, 96.0);
const SHEET_HEIGHT: f32 = 360.0;

const TYPES: [ActionType; 5] = [ActionType::App, ActionType::Url, ActionType::File, ActionType::Shortcut, ActionType::Shell];

fn accent(theme: RingTheme) -> Color32 {
    match theme {
        RingTheme::Orbit => Color32::from_rgb(139, 114, 255),
    }
}

/// Text colour on top of the accent.
fn on_accent(theme: RingTheme) -> Color32 {
    match theme {
        RingTheme::Orbit => Color32::WHITE,
    }
}

fn ring_theme(theme: RingTheme) -> Theme {
    match theme {
        RingTheme::Orbit => Theme::orbit(),
    }
}

fn type_label(t: ActionType) -> &'static str {
    match t {
        ActionType::App => "App",
        ActionType::Url => "Website",
        ActionType::File => "File",
        ActionType::Shortcut => "Shortcut",
        ActionType::Shell => "Command",
    }
}

fn placeholder(t: ActionType) -> &'static str {
    match t {
        ActionType::App if cfg!(target_os = "macos") => "Safari, or /Applications/Safari.app",
        ActionType::App if cfg!(windows) => "notepad.exe or C:\\Path\\App.exe",
        ActionType::App => "firefox.desktop",
        ActionType::Url => "https://example.com",
        ActionType::File => "~/Downloads",
        ActionType::Shortcut if cfg!(target_os = "macos") => "cmd+shift+4",
        ActionType::Shortcut => "ctrl+alt+t",
        ActionType::Shell if cfg!(windows) => "start ms-settings:",
        ActionType::Shell => "open -a Calculator",
    }
}

/// Is the resident app running? It holds an exclusive lock on its lock file.
fn resident_running() -> bool {
    match std::fs::OpenOptions::new().write(true).create(true).truncate(false).open(paths::lock_file("theorbit")) {
        Ok(f) => match f.try_lock() {
            Ok(()) => {
                let _ = f.unlock();
                false
            }
            Err(_) => true,
        },
        Err(_) => false,
    }
}

fn autostart() -> Option<auto_launch::AutoLaunch> {
    let exe = paths::sibling_exe("theorbit")?;
    auto_launch::AutoLaunchBuilder::new()
        .set_app_name("TheOrbit")
        .set_app_path(exe.to_str()?)
        .set_macos_launch_mode(auto_launch::MacOSLaunchMode::LaunchAgent)
        .build()
        .ok()
}

fn icon_key(item: &OrbitItem) -> String {
    format!("{:?}|{}|{:?}", item.kind, item.value, item.icon)
}

/// The live ring at the top of the window.
struct Preview {
    key: String,
    cache: Option<RingCache>,
    texture: Option<TextureHandle>,
    tween: HoverTween,
    hover: Option<usize>,
    pointer: Option<f32>,
    opened: Instant,
    /// A click on a segment plays the "chosen" close animation: since when.
    select: Option<Instant>,
    drawn: Option<Instant>,
}

impl Preview {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            key: String::new(),
            cache: None,
            texture: None,
            tween: HoverTween::new(now),
            hover: None,
            pointer: None,
            opened: now,
            select: None,
            drawn: None,
        }
    }

    fn replay(&mut self) {
        self.opened = Instant::now();
        self.select = None;
        self.drawn = None;
    }
}

/// The edit sheet: which item, and whether it is opening or closing.
#[derive(Clone, Copy)]
struct Sheet {
    index: usize,
    open: bool,
}

/// A destructive action waiting for confirmation.
#[derive(Clone, Copy)]
enum Confirm {
    Remove(usize),
    Reset,
}

struct SettingsApp {
    config: OrbitConfig,
    saved: OrbitConfig,
    dirty_since: Option<Instant>,
    save_error: Option<String>,
    status: HookStatus,
    running: bool,
    last_poll: Instant,
    autostart: Option<auto_launch::AutoLaunch>,
    autostart_on: bool,
    toast: Option<(String, Instant)>,
    font: Option<Font>,
    icons: HashMap<String, Option<(orbit_render::Pixmap, TextureHandle)>>,
    preview: Preview,
    sheet: Option<Sheet>,
    confirm: Option<Confirm>,
    logo: TextureHandle,
    styled_for: Option<RingTheme>,
    reduced: bool,
}

impl SettingsApp {
    fn new(cc: &eframe::CreationContext) -> Self {
        let reduced = orbit_sys::motion::prefers_reduced_motion();
        anim::set_reduced_motion(reduced);
        let font = Font::system();
        if let Some(f) = &font {
            // Use the OS UI font (crisp, and it has the symbols egui's default lacks).
            cc.egui_ctx.add_font(egui::epaint::text::FontInsert::new(
                "system-ui",
                egui::FontData::from_static(f.bytes()),
                vec![egui::epaint::text::InsertFontFamily {
                    family: egui::FontFamily::Proportional,
                    priority: egui::epaint::text::FontPriority::Highest,
                }],
            ));
        }
        cc.egui_ctx.set_theme(egui::Theme::Dark);

        let config = OrbitConfig::load_or_create(&paths::config_file()).unwrap_or_default();
        let autostart = autostart();
        let autostart_on = autostart.as_ref().and_then(|a| a.is_enabled().ok()).unwrap_or(false);
        let mark = logo::mark(96, orbit_render::Color::from_rgba8(236, 242, 250, 255));
        let logo = cc.egui_ctx.load_texture(
            "logo",
            egui::ColorImage::from_rgba_premultiplied([96, 96], mark.data()),
            egui::TextureOptions::LINEAR,
        );
        Self {
            saved: config.clone(),
            config,
            dirty_since: None,
            save_error: None,
            status: HookStatus::read().unwrap_or_default(),
            running: resident_running(),
            last_poll: Instant::now(),
            autostart,
            autostart_on,
            toast: None,
            font,
            icons: HashMap::new(),
            preview: Preview::new(),
            sheet: None,
            confirm: None,
            logo,
            styled_for: None,
            reduced,
        }
    }

    fn accent(&self) -> Color32 {
        accent(self.config.theme)
    }

    fn flash(&mut self, msg: impl Into<String>) {
        self.toast = Some((msg.into(), Instant::now()));
    }

    fn icon(&mut self, ctx: &egui::Context, item: &OrbitItem) -> Option<(orbit_render::Pixmap, TextureHandle)> {
        let key = icon_key(item);
        if let Some(hit) = self.icons.get(&key) {
            return hit.clone();
        }
        let loaded = icons::for_item(item).and_then(|png| orbit_render::decode_png(&png)).map(|pm| {
            let img = egui::ColorImage::from_rgba_premultiplied([pm.width() as usize, pm.height() as usize], pm.data());
            let tex = ctx.load_texture(format!("icon-{key}"), img, egui::TextureOptions::LINEAR);
            (pm, tex)
        });
        self.icons.insert(key, loaded.clone());
        loaded
    }

    fn monogram(&self, item: &OrbitItem) -> String {
        item.monogram()
            .filter(|m| self.font.as_ref().is_none_or(|f| f.covers(m)))
            .unwrap_or_else(|| orbit_render::monogram(&item.title))
    }

    fn segments(&mut self, ctx: &egui::Context) -> Vec<Segment> {
        let items = self.config.items.clone();
        items
            .iter()
            .map(|it| Segment { title: it.title.clone(), icon: self.icon(ctx, it).map(|(pm, _)| pm), monogram: self.monogram(it) })
            .collect()
    }

    // -- persistence ---------------------------------------------------------

    fn mark_dirty(&mut self) {
        if self.config != self.saved && self.dirty_since.is_none() {
            self.dirty_since = Some(Instant::now());
        }
    }

    fn autosave(&mut self, ctx: &egui::Context) {
        let Some(since) = self.dirty_since else { return };
        let wait = Duration::from_millis(400);
        if since.elapsed() < wait {
            ctx.request_repaint_after(wait - since.elapsed());
            return;
        }
        self.dirty_since = None;
        match self.config.save(&paths::config_file()) {
            Ok(()) => {
                self.saved = self.config.clone();
                self.save_error = None;
            }
            Err(e) => self.save_error = Some(e.to_string()),
        }
    }

    fn poll_status(&mut self, ctx: &egui::Context) {
        if self.last_poll.elapsed() >= Duration::from_secs(1) {
            self.last_poll = Instant::now();
            self.status = HookStatus::read().unwrap_or_default();
            self.running = resident_running();
        }
        ctx.request_repaint_after(Duration::from_secs(1));
    }

    // -- sections ------------------------------------------------------------

    fn header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add(egui::Image::new(&self.logo).fit_to_exact_size(Vec2::splat(22.0)));
            ui.label(RichText::new("TheOrbit").size(17.0).strong().color(TEXT));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let (text, color) = if !self.running {
                    ("Not running", BAD)
                } else if self.status.error.is_some() {
                    ("Error", BAD)
                } else if self.status.needs_permission {
                    ("Needs access", WARN)
                } else if !self.config.enabled {
                    ("Paused", MUTED)
                } else {
                    ("Active", OK)
                };
                ui.label(RichText::new(text).color(MUTED));
                let (dot, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                ui.painter().circle_filled(dot.center(), 4.0, color);
                if self.save_error.is_some() {
                    ui.add_space(8.0);
                    ui.label(RichText::new("Not saved").color(BAD));
                }
            });
        });

        let notice = if !self.running {
            Some(("TheOrbit isn't running", "Start it to use the orbit.", "Start", 0))
        } else if self.status.needs_permission {
            Some(("Allow Accessibility access", "Needed to see the right mouse button anywhere.", "Open Settings", 1))
        } else {
            self.status.error.as_deref().map(|_| ("The orbit couldn't start", "See the error below.", "", 2))
        };
        if let Some((title, body, action, kind)) = notice {
            ui.add_space(10.0);
            let clicked = notice_card(ui, title, body, action, self.config.theme);
            if kind == 2 {
                if let Some(e) = &self.status.error {
                    ui.label(RichText::new(e).small().color(MUTED));
                }
            }
            if clicked {
                match kind {
                    0 => match paths::sibling_exe("theorbit") {
                        Some(exe) => {
                            let _ = std::process::Command::new(exe).spawn();
                        }
                        None => self.flash("Couldn't find TheOrbit"),
                    },
                    _ => {
                        orbit_input::has_permission(true);
                        let _ = open::that_detached("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility");
                    }
                }
            }
        }
    }

    fn hero(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let scale = ctx.pixels_per_point();
        let k = PREVIEW_RADIUS / self.config.radius as f32;
        let dead_zone = self.config.dead_zone as f32 * k;
        let n = self.config.items.len();
        let side = orbit_render::side(PREVIEW_RADIUS);
        let (rect, resp) = ui.allocate_exact_size(Vec2::splat(side), Sense::click());
        let now = Instant::now();
        let p = &mut self.preview;

        let selecting = p.select.is_some();
        let (hover, pointer) = match resp.hover_pos() {
            Some(pos) if !selecting => {
                let d = pos - rect.center();
                (
                    orbit_core::segment_at(d.x as f64, d.y as f64, n, dead_zone as f64),
                    (d.length() > 8.0).then(|| orbit_core::geometry::angle_from_top(d.x as f64, d.y as f64) as f32),
                )
            }
            _ if selecting => (p.hover, p.pointer),
            _ => (None, None),
        };
        if hover.is_some() {
            ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        // Clicking an item plays the "chosen" animation (nothing is launched here).
        if resp.clicked() && hover.is_some() && !selecting {
            p.select = Some(now);
            if self.config.sound {
                orbit_sys::sound::play_select();
            }
        }

        let key = format!(
            "{:?}|{}|{}|{}",
            self.config.items.iter().map(|i| (&i.title, i.kind, &i.value, &i.icon)).collect::<Vec<_>>(),
            self.config.dead_zone,
            self.config.radius,
            scale,
        );
        let select_t = p.select.map(|at| anim::progress(at, anim::CLOSE, now));
        if select_t.is_some_and(|t| t >= 1.0) {
            // Pause on the empty frame briefly, then open again.
            if p.select.is_some_and(|at| at.elapsed() > anim::CLOSE + Duration::from_millis(250)) {
                p.replay();
            }
        }
        let open_t = anim::progress(p.opened, anim::OPEN, now);
        let moving = open_t < 1.0 || !p.tween.done(now) || p.select.is_some();
        let pointer_moved = match (p.pointer, pointer) {
            (Some(a), Some(b)) => anim::shortest_delta(a, b).abs() > 0.02,
            (a, b) => a.is_some() != b.is_some(),
        };
        let rebuild = p.key != key || p.cache.is_none();

        if rebuild || moving || pointer_moved || hover != p.hover {
            let theme = ring_theme(self.config.theme);
            let segments = self.segments(&ctx);
            let p = &mut self.preview;
            let ring = Ring {
                segments: &segments,
                radius: PREVIEW_RADIUS,
                dead_zone,
                hover,
                scale,
                font: self.font.as_ref(),
                theme: &theme,
                pointer,
                idle_hint: Some("Try me"),
            };
            if rebuild {
                p.cache = Some(RingCache::new(&Ring { hover: None, pointer: None, ..ring }));
                p.key = key;
            }
            p.tween.set(hover, n, now);
            p.hover = hover;
            p.pointer = pointer;
            let pose = match select_t {
                Some(t) => Pose::closing(t, true),
                None => Pose::opening(open_t),
            };
            let frame = orbit_render::compose(p.cache.as_ref().unwrap().frame_animated(&ring, p.tween.at(now)), pose);
            let img = egui::ColorImage::from_rgba_premultiplied([frame.width() as usize, frame.height() as usize], frame.data());
            match &mut p.texture {
                Some(t) => t.set(img, egui::TextureOptions::LINEAR),
                None => p.texture = Some(ctx.load_texture("preview", img, egui::TextureOptions::LINEAR)),
            }
            p.drawn = Some(now);
        }
        let p = &self.preview;
        if moving || !p.tween.done(now) {
            ctx.request_repaint();
        }
        if let Some(t) = &p.texture {
            ui.painter().image(t.id(), rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        }
    }

    fn tiles(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        ui.horizontal(|ui| {
            ui.label(RichText::new("Items").size(15.0).strong().color(TEXT));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new("Drag to reorder").small().color(MUTED));
            });
        });
        ui.add_space(4.0);

        let mut open: Option<usize> = None;
        let mut moved: Option<(usize, usize)> = None;
        let mut add: Option<ActionType> = None;
        let accent = self.accent();
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(10.0, 10.0);
            for i in 0..self.config.items.len() {
                let item = self.config.items[i].clone();
                let icon = self.icon(&ctx, &item);
                let mono = self.monogram(&item);
                let (rect, resp) = ui.allocate_exact_size(TILE, Sense::click_and_drag());
                if resp.drag_started() {
                    resp.dnd_set_drag_payload(i);
                }
                let dragging = resp.dragged();
                let drop_here = resp.dnd_hover_payload::<usize>().is_some_and(|from| *from != i);
                if let Some(from) = resp.dnd_release_payload::<usize>() {
                    moved = Some((*from, i));
                }
                if resp.clicked() {
                    open = Some(i);
                }
                if resp.hovered() && !dragging {
                    ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                let lift = ctx.animate_bool_with_time(Id::unique(("lift", &item.id)), resp.hovered() || dragging, 0.12);
                let r = rect.translate(Vec2::new(0.0, -2.0 * lift));
                let painter = ui.painter();
                let border = if drop_here { accent } else { lerp(BORDER, accent.gamma_multiply(0.6), lift) };
                painter.rect(r, 14.0, if dragging { SURFACE_2 } else { SURFACE }, Stroke::new(1.0, border), egui::StrokeKind::Inside);
                let icon_rect = Rect::from_center_size(pos2(r.center().x, r.top() + 36.0), Vec2::splat(36.0));
                match icon {
                    Some((_, tex)) => {
                        painter.image(tex.id(), icon_rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                    }
                    None => {
                        painter.circle_filled(icon_rect.center(), 17.0, SURFACE_2);
                        painter.text(icon_rect.center(), egui::Align2::CENTER_CENTER, mono, egui::FontId::proportional(14.0), TEXT);
                    }
                }
                let title = truncate(if item.title.is_empty() { "Untitled" } else { &item.title }, 11);
                painter.text(pos2(r.center().x, r.bottom() - 20.0), egui::Align2::CENTER_CENTER, title, egui::FontId::proportional(12.5), TEXT);
            }

            // "+" tile with a small menu of types.
            let (rect, resp) = ui.allocate_exact_size(TILE, Sense::click());
            let hot = ctx.animate_bool_with_time(Id::unique("add-tile"), resp.hovered(), 0.12);
            let painter = ui.painter();
            painter.rect_stroke(rect, 14.0, Stroke::new(1.0, lerp(BORDER, accent, hot)), egui::StrokeKind::Inside);
            let c = rect.center();
            let color = lerp(MUTED, accent, hot);
            painter.line_segment([c - Vec2::new(9.0, 0.0), c + Vec2::new(9.0, 0.0)], Stroke::new(2.0, color));
            painter.line_segment([c - Vec2::new(0.0, 9.0), c + Vec2::new(0.0, 9.0)], Stroke::new(2.0, color));
            if resp.hovered() {
                ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            egui::Popup::menu(&resp).show(|ui| {
                ui.set_min_width(160.0);
                for t in TYPES {
                    if ui.add(egui::Button::new(type_label(t)).frame(false).min_size(Vec2::new(150.0, 28.0))).clicked() {
                        add = Some(t);
                        ui.close();
                    }
                }
            });
        });

        if let Some((from, to)) = moved {
            if from < self.config.items.len() && to < self.config.items.len() {
                let item = self.config.items.remove(from);
                self.config.items.insert(to, item);
                self.mark_dirty();
            }
        }
        if let Some(i) = open {
            self.sheet = Some(Sheet { index: i, open: true });
        }
        if let Some(t) = add {
            self.add_item(t);
        }
        if self.config.items.len() > 12 {
            ui.add_space(4.0);
            ui.label(RichText::new("More than 12 items makes segments hard to hit.").small().color(WARN));
        }
    }

    fn preferences(&mut self, ui: &mut egui::Ui) {
        let mut changed = false;
        egui::CollapsingHeader::new(RichText::new("Preferences").size(15.0).strong().color(TEXT))
            .id_salt("prefs")
            .default_open(false)
            .show_unindented(ui, |ui| {
                ui.add_space(6.0);
                let theme = self.config.theme;
                changed |= switch_row(ui, "Orbit enabled", &mut self.config.enabled, theme, self.reduced);
                let mut on = self.autostart_on;
                if ui.add_enabled_ui(self.autostart.is_some(), |ui| switch_row(ui, "Launch at login", &mut on, theme, self.reduced)).inner {
                    if let Some(a) = &self.autostart {
                        match if on { a.enable() } else { a.disable() } {
                            Ok(()) => self.autostart_on = on,
                            Err(e) => self.flash(format!("Couldn't change launch at login: {e}")),
                        }
                    }
                }
                changed |= switch_row(ui, "Sound on select", &mut self.config.sound, theme, self.reduced);
                changed |= slider_row(ui, "Hold delay", &mut self.config.hold_ms, 150..=1000, 25.0, "ms");
                ui.add_space(4.0);
                egui::CollapsingHeader::new(RichText::new("Advanced").color(MUTED)).id_salt("advanced").show_unindented(ui, |ui| {
                    changed |= slider_row(ui, "Ring size", &mut self.config.radius, 100.0..=240.0, 5.0, "px");
                    changed |= slider_row(ui, "Cancel zone", &mut self.config.dead_zone, 20.0..=80.0, 2.0, "px");
                    ui.add_space(6.0);
                    if ui.add(quiet_button("Reset to defaults")).clicked() {
                        self.confirm = Some(Confirm::Reset);
                    }
                });
            });
        if changed {
            self.mark_dirty();
        }
    }

    // -- edit sheet ----------------------------------------------------------

    fn sheet(&mut self, ctx: &egui::Context) {
        let Some(sheet) = self.sheet else { return };
        let t = ctx.animate_bool_with_time(Id::unique("sheet"), sheet.open, if self.reduced { 0.0 } else { 0.22 });
        if !sheet.open && t <= 0.0 {
            self.sheet = None;
            return;
        }
        if sheet.index >= self.config.items.len() {
            self.sheet = None;
            return;
        }
        let screen = ctx.content_rect();
        let e = anim::ease_out_cubic(t);

        // Scrim: dims the window; clicking it closes the sheet.
        egui::Area::new(Id::unique("scrim")).order(egui::Order::Middle).fixed_pos(screen.min).show(ctx, |ui| {
            let resp = ui.allocate_rect(screen, Sense::click());
            ui.painter().rect_filled(screen, 0.0, Color32::from_black_alpha((120.0 * e) as u8));
            if resp.clicked() && sheet.open {
                self.sheet = Some(Sheet { open: false, ..sheet });
            }
        });

        let width = screen.width().min(CONTENT_WIDTH + 40.0);
        let left = screen.center().x - width / 2.0;
        let top = screen.bottom() - SHEET_HEIGHT * e;
        let mut close = false;
        egui::Area::new(Id::unique("sheet-panel")).order(egui::Order::Foreground).fixed_pos(Pos2::new(left, top)).show(ctx, |ui| {
            egui::Frame::new()
                .fill(SURFACE)
                .stroke(Stroke::new(1.0, BORDER))
                .corner_radius(egui::CornerRadius { nw: 20, ne: 20, sw: 0, se: 0 })
                .inner_margin(egui::Margin { left: 22, right: 22, top: 14, bottom: 22 })
                .show(ui, |ui| {
                    ui.set_width(width - 44.0);
                    ui.set_min_height(SHEET_HEIGHT);
                    // Grabber.
                    let (g, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 8.0), Sense::hover());
                    ui.painter().rect_filled(Rect::from_center_size(g.center(), Vec2::new(36.0, 4.0)), 2.0, BORDER);
                    ui.add_space(6.0);
                    close = self.editor(ui, sheet.index);
                });
        });
        if close || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.sheet = Some(Sheet { open: false, ..sheet });
        }
        if t < 1.0 && t > 0.0 {
            ctx.request_repaint();
        }
    }

    /// Returns true when the sheet should close.
    fn editor(&mut self, ui: &mut egui::Ui, i: usize) -> bool {
        let ctx = ui.ctx().clone();
        let theme = self.config.theme;
        let accent = self.accent();
        let mut changed = false;
        let mut close = false;
        let mut run = false;
        let mut remove = false;
        let item_snapshot = self.config.items[i].clone();
        let icon = self.icon(&ctx, &item_snapshot);
        let mono = self.monogram(&item_snapshot);

        ui.horizontal(|ui| {
            match &icon {
                Some((_, tex)) => {
                    ui.add(egui::Image::new(tex).fit_to_exact_size(Vec2::splat(34.0)));
                }
                None => {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(34.0), Sense::hover());
                    ui.painter().circle_filled(r.center(), 17.0, SURFACE_2);
                    ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, mono, egui::FontId::proportional(14.0), TEXT);
                }
            }
            let item = &mut self.config.items[i];
            let edit = egui::TextEdit::singleline(&mut item.title)
                .hint_text("Name")
                .font(egui::FontId::proportional(18.0))
                .frame(egui::Frame::NONE)
                .desired_width(ui.available_width() - 80.0);
            changed |= ui.add(edit).changed();
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(accent_button("Done", theme)).clicked() {
                    close = true;
                }
            });
        });
        ui.add_space(12.0);

        let item = &mut self.config.items[i];
        // Type: a quiet segmented row.
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            for t in TYPES {
                let on = item.kind == t;
                let text = RichText::new(type_label(t)).color(if on { on_accent(theme) } else { MUTED });
                let b = egui::Button::new(text).fill(if on { accent } else { SURFACE_2 }).corner_radius(15).min_size(Vec2::new(0.0, 30.0));
                if ui.add(b).clicked() && !on {
                    item.kind = t;
                    changed = true;
                }
            }
        });
        ui.add_space(12.0);

        let err = (item.kind == ActionType::Shortcut && !item.value.trim().is_empty())
            .then(|| actions::parse_shortcut(&item.value).err())
            .flatten();
        ui.horizontal(|ui| {
            let browse = matches!(item.kind, ActionType::App | ActionType::File);
            let w = ui.available_width() - if browse { 92.0 } else { 0.0 };
            let mut edit = egui::TextEdit::singleline(&mut item.value)
                .hint_text(placeholder(item.kind))
                .desired_width(w)
                .margin(Vec2::new(10.0, 8.0));
            if err.is_some() {
                edit = edit.text_color(BAD);
            }
            changed |= ui.add(edit).changed();
            if browse && ui.add(quiet_button("Browse…")).clicked() {
                if let Some(path) = pick_path(item.kind) {
                    if item.title.trim().is_empty() {
                        item.title = title_from_path(&path);
                    }
                    item.value = path;
                    changed = true;
                }
            }
        });
        if let Some(e) = &err {
            ui.label(RichText::new(e).small().color(BAD));
        }

        ui.add_space(18.0);
        ui.horizontal(|ui| {
            if ui.add_enabled(!item.value.trim().is_empty() && err.is_none(), quiet_button("Try it")).clicked() {
                run = true;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(egui::Button::new(RichText::new("Remove").color(BAD)).frame(false)).clicked() {
                    remove = true;
                }
            });
        });

        if run {
            let item = self.config.items[i].clone();
            match actions::run(&item) {
                Ok(()) => self.flash(format!("Opened {}", item.title)),
                Err(e) => self.flash(e),
            }
        }
        if remove {
            self.confirm = Some(Confirm::Remove(i));
        }
        if changed {
            self.mark_dirty();
        }
        close
    }

    fn add_item(&mut self, kind: ActionType) {
        let mut item = OrbitItem::new("", kind, "");
        match kind {
            ActionType::App | ActionType::File => {
                let Some(path) = pick_path(kind) else { return };
                item.title = title_from_path(&path);
                item.value = path;
            }
            ActionType::Url => item.title = "Website".into(),
            ActionType::Shortcut => item.title = "Shortcut".into(),
            ActionType::Shell => item.title = "Command".into(),
        }
        self.config.items.push(item);
        self.mark_dirty();
        self.sheet = Some(Sheet { index: self.config.items.len() - 1, open: true });
    }

    fn confirm_dialog(&mut self, ctx: &egui::Context) {
        let Some(what) = self.confirm else { return };
        let (title, action) = match what {
            Confirm::Remove(i) => {
                let name = self.config.items.get(i).map(|it| it.title.clone()).unwrap_or_default();
                (format!("Remove {name}?"), "Remove")
            }
            Confirm::Reset => ("Reset everything to defaults?".to_string(), "Reset"),
        };
        let mut decision = None;
        let resp = egui::Modal::new(Id::unique("confirm")).show(ctx, |ui| {
            ui.set_width(300.0);
            ui.label(RichText::new(title).size(16.0).strong().color(TEXT));
            ui.add_space(14.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let danger = egui::Button::new(RichText::new(action).strong().color(Color32::WHITE)).fill(BAD).corner_radius(10).min_size(Vec2::new(0.0, 32.0));
                if ui.add(danger).clicked() {
                    decision = Some(true);
                }
                if ui.add(quiet_button("Cancel")).clicked() {
                    decision = Some(false);
                }
            });
        });
        if resp.should_close() && decision.is_none() {
            decision = Some(false);
        }
        match decision {
            Some(true) => {
                match what {
                    Confirm::Remove(i) if i < self.config.items.len() => {
                        self.config.items.remove(i);
                        if let Some(s) = self.sheet {
                            self.sheet = Some(Sheet { open: false, ..s });
                        }
                    }
                    Confirm::Remove(_) => {}
                    Confirm::Reset => self.config = OrbitConfig::default(),
                }
                self.confirm = None;
                self.mark_dirty();
            }
            Some(false) => self.confirm = None,
            None => {}
        }
    }

    fn toast(&mut self, ctx: &egui::Context) {
        let Some((msg, at)) = &self.toast else { return };
        let age = at.elapsed().as_secs_f32();
        if age >= 2.6 {
            self.toast = None;
            return;
        }
        let alpha = (age / 0.15).min(1.0).min((2.6 - age) / 0.4).clamp(0.0, 1.0);
        egui::Area::new(Id::unique("toast")).order(egui::Order::Tooltip).anchor(egui::Align2::CENTER_BOTTOM, [0.0, -24.0]).show(ctx, |ui| {
            ui.set_opacity(alpha);
            egui::Frame::new()
                .fill(SURFACE_2)
                .corner_radius(18)
                .inner_margin(egui::Margin::symmetric(16, 9))
                .show(ui, |ui| ui.label(RichText::new(msg.as_str()).color(TEXT)));
        });
        ctx.request_repaint();
    }

    fn apply_style(&mut self, ctx: &egui::Context) {
        if self.styled_for == Some(self.config.theme) {
            return;
        }
        self.styled_for = Some(self.config.theme);
        let accent = self.accent();
        let reduced = self.reduced;
        ctx.style_mut_of(egui::Theme::Dark, |s| {
            use egui::{FontId, TextStyle};
            s.text_styles = [
                (TextStyle::Heading, FontId::proportional(20.0)),
                (TextStyle::Body, FontId::proportional(14.0)),
                (TextStyle::Button, FontId::proportional(14.0)),
                (TextStyle::Monospace, FontId::monospace(13.0)),
                (TextStyle::Small, FontId::proportional(12.0)),
            ]
            .into();
            s.spacing.item_spacing = Vec2::new(8.0, 8.0);
            s.spacing.button_padding = Vec2::new(14.0, 7.0);
            s.spacing.interact_size.y = 30.0;
            s.animation_time = if reduced { 0.0 } else { 0.15 };
            let v = &mut s.visuals;
            v.override_text_color = Some(TEXT);
            v.panel_fill = BG;
            v.window_fill = SURFACE;
            v.window_stroke = Stroke::new(1.0, BORDER);
            v.window_corner_radius = 16.into();
            v.extreme_bg_color = SURFACE_2;
            v.selection.bg_fill = accent.gamma_multiply(0.4);
            v.selection.stroke = Stroke::new(1.5, accent);
            v.slider_trailing_fill = true;
            for (w, fill, stroke) in [
                (&mut v.widgets.noninteractive, SURFACE, Color32::TRANSPARENT),
                (&mut v.widgets.inactive, SURFACE_2, Color32::TRANSPARENT),
                (&mut v.widgets.hovered, SURFACE_2, accent.gamma_multiply(0.5)),
                (&mut v.widgets.active, SURFACE_2, accent),
                (&mut v.widgets.open, SURFACE_2, accent.gamma_multiply(0.5)),
            ] {
                w.bg_fill = fill;
                w.weak_bg_fill = fill;
                w.bg_stroke = Stroke::new(1.0, stroke);
                w.corner_radius = 10.into();
                w.expansion = 0.0;
            }
            v.widgets.inactive.fg_stroke = Stroke::new(1.5, accent);
            v.widgets.hovered.fg_stroke = Stroke::new(1.5, TEXT);
            v.widgets.active.fg_stroke = Stroke::new(1.5, TEXT);
            v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, MUTED);
        });
    }
}

// -- small widgets -------------------------------------------------------------

fn lerp(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let mix = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_premultiplied(mix(a.r(), b.r()), mix(a.g(), b.g()), mix(a.b(), b.b()), mix(a.a(), b.a()))
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() > max {
        let mut out: String = s.chars().take(max - 1).collect();
        out.push('…');
        out
    } else {
        s.to_string()
    }
}

fn quiet_button(text: &str) -> egui::Button<'static> {
    egui::Button::new(RichText::new(text.to_string()).color(TEXT)).fill(SURFACE_2).corner_radius(10).min_size(Vec2::new(0.0, 32.0))
}

fn accent_button(text: &str, theme: RingTheme) -> egui::Button<'static> {
    egui::Button::new(RichText::new(text.to_string()).strong().color(on_accent(theme)))
        .fill(accent(theme))
        .corner_radius(10)
        .min_size(Vec2::new(0.0, 32.0))
}

fn notice_card(ui: &mut egui::Ui, title: &str, body: &str, action: &str, theme: RingTheme) -> bool {
    let mut clicked = false;
    egui::Frame::new().fill(SURFACE).corner_radius(14).inner_margin(14.0).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.label(RichText::new(title).strong().color(TEXT));
                ui.label(RichText::new(body).small().color(MUTED));
            });
            if !action.is_empty() {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    clicked = ui.add(accent_button(action, theme)).clicked();
                });
            }
        });
    });
    clicked
}

/// Label on the left, animated switch on the right. Returns true if toggled.
fn switch_row(ui: &mut egui::Ui, label: &str, on: &mut bool, theme: RingTheme, reduced: bool) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).color(TEXT));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(40.0, 24.0), Sense::click());
            if resp.clicked() {
                *on = !*on;
                changed = true;
            }
            let t = ui.ctx().animate_bool_with_time(resp.id, *on, if reduced { 0.0 } else { 0.15 });
            let enabled = ui.is_enabled();
            let mut bg = lerp(SURFACE_2, accent(theme), t);
            if !enabled {
                bg = bg.gamma_multiply(0.5);
            }
            let p = ui.painter();
            p.rect_filled(rect, 12.0, bg);
            let x = egui::lerp((rect.left() + 12.0)..=(rect.right() - 12.0), t);
            p.circle_filled(pos2(x, rect.center().y), 9.0, if enabled { Color32::WHITE } else { MUTED });
            if resp.has_focus() {
                p.rect_stroke(rect.expand(2.0), 14.0, Stroke::new(1.5, accent(theme)), egui::StrokeKind::Outside);
            }
            if resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
        });
    });
    changed
}

fn slider_row<N: egui::emath::Numeric + std::fmt::Display>(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut N,
    range: std::ops::RangeInclusive<N>,
    step: f64,
    unit: &str,
) -> bool {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).color(TEXT));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(format!("{value} {unit}")).color(MUTED));
        });
    });
    ui.spacing_mut().slider_width = ui.available_width();
    let changed = ui.add(egui::Slider::new(value, range).step_by(step).show_value(false)).changed();
    ui.add_space(4.0);
    changed
}

fn title_from_path(p: &str) -> String {
    let base = std::path::Path::new(p).file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| p.into());
    for ext in [".app", ".exe", ".lnk", ".desktop", ".AppImage"] {
        if let Some(stripped) = base.strip_suffix(ext) {
            return stripped.into();
        }
    }
    base
}

fn pick_path(kind: ActionType) -> Option<String> {
    let mut dialog = rfd::FileDialog::new();
    if kind == ActionType::App {
        dialog = if cfg!(target_os = "macos") {
            dialog.add_filter("Applications", &["app"]).set_directory("/Applications")
        } else if cfg!(windows) {
            dialog.add_filter("Programs", &["exe", "lnk", "bat", "cmd"])
        } else {
            dialog.add_filter("Applications", &["desktop", "AppImage", "sh"]).set_directory("/usr/share/applications")
        };
    }
    dialog.pick_file().map(|p| p.to_string_lossy().into_owned())
}

impl eframe::App for SettingsApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.apply_style(&ctx);
        self.poll_status(&ctx);

        egui::CentralPanel::default().frame(egui::Frame::new().fill(BG)).show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                // One centred column.
                let full = ui.available_width();
                let width = (full - 32.0).min(CONTENT_WIDTH);
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    ui.add_space((full - width) / 2.0);
                    ui.vertical(|ui| {
                        ui.set_width(width);
                        self.header(ui);
                        ui.add_space(6.0);
                        ui.vertical_centered(|ui| {
                            self.hero(ui);
                            ui.label(RichText::new("Hold right-click anywhere, release on an item.").small().color(MUTED));
                        });
                        ui.add_space(26.0);
                        self.tiles(ui);
                        ui.add_space(22.0);
                        self.preferences(ui);
                        ui.add_space(30.0);
                    });
                });
            });
        });

        self.sheet(&ctx);
        self.confirm_dialog(&ctx);
        self.toast(&ctx);
        self.autosave(&ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if self.config != self.saved {
            let _ = self.config.save(&paths::config_file());
        }
    }
}

fn main() -> eframe::Result {
    // One settings window at a time.
    let _ = std::fs::create_dir_all(paths::config_dir());
    let lock = std::fs::File::create(paths::lock_file("settings")).ok();
    if lock.as_ref().is_some_and(|f| f.try_lock().is_err()) {
        return Ok(());
    }

    let mark = logo::app_icon(256);
    let icon = egui::IconData { rgba: logo::to_straight_rgba(&mark), width: 256, height: 256 };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("TheOrbit")
            .with_inner_size([600.0, 860.0])
            .with_min_inner_size([480.0, 640.0])
            .with_icon(icon),
        ..Default::default()
    };
    eframe::run_native("TheOrbit", options, Box::new(|cc| Ok(Box::new(SettingsApp::new(cc)))))
}
