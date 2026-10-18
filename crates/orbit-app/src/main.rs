//! TheOrbit resident app: tray icon + global right-button hook + native ring
//! overlay. Idle cost is one parked event loop; work happens only while the
//! right button is held.

// No console window on Windows.
#![cfg_attr(windows, windows_subsystem = "windows")]

mod overlay;
mod tray;

use orbit_core::{segment_at, OrbitConfig, OrbitItem};
use orbit_input::{Engine, Output, Point, StartError};
use orbit_render::anim::{self, HoverTween, Pose};
use orbit_render::{Font, Pixmap, Ring, RingCache, Segment, Theme};
use orbit_sys::{actions, icons, paths, status::HookStatus};
use overlay::Presenter;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::event::{StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::window::WindowId;

/// Everything that wakes the main thread.
pub enum UserEvent {
    Hook(Output),
    HookStarted(Arc<Engine>),
    Status(HookStatus),
    ConfigChanged,
    RunAction(Box<OrbitItem>),
    Tray(tray::Command),
}

/// The orbit currently on screen.
struct Session {
    center: Point,
    /// Device pixels per logical pixel on the monitor under the cursor.
    scale: f64,
    hover: Option<usize>,
    cache: RingCache,
    tween: HoverTween,
    opened: Instant,
    /// Set once the button is released: when, and whether an item was chosen.
    closing: Option<(Instant, bool)>,
    /// Pointer direction from the centre, once it has left the middle.
    pointer: Option<f32>,
    /// A frame must be drawn on the next tick.
    dirty: bool,
}

impl Session {
    fn pose(&self, now: Instant) -> Pose {
        let open = Pose::opening(anim::progress(self.opened, anim::OPEN, now));
        match self.closing {
            Some((at, chosen)) => {
                let close = Pose::closing(anim::progress(at, anim::CLOSE, now), chosen);
                Pose { scale: open.scale * close.scale, opacity: open.opacity * close.opacity }
            }
            None => open,
        }
    }

    fn animating(&self, now: Instant) -> bool {
        anim::progress(self.opened, anim::OPEN, now) < 1.0 || !self.tween.done(now) || self.closing.is_some()
    }

    fn finished(&self, now: Instant) -> bool {
        self.closing.is_some_and(|(at, _)| anim::progress(at, anim::CLOSE, now) >= 1.0)
    }
}

struct App {
    proxy: EventLoopProxy<UserEvent>,
    config: OrbitConfig,
    engine: Option<Arc<Engine>>,
    presenter: Option<Box<dyn Presenter>>,
    tray: Option<tray::Tray>,
    session: Option<Session>,
    segments: Vec<Segment>,
    icon_cache: HashMap<String, Option<Pixmap>>,
    font: Option<Font>,
    theme: Theme,
    watcher: Option<notify::RecommendedWatcher>,
    last_frame: Instant,
}

impl App {
    fn ring(&self, scale: f64, hover: Option<usize>) -> Ring<'_> {
        Ring {
            segments: &self.segments,
            radius: self.config.radius as f32,
            dead_zone: self.config.dead_zone as f32,
            hover,
            scale: scale as f32,
            font: self.font.as_ref(),
            theme: &self.theme,
            pointer: None,
            idle_hint: Some("Cancel"),
        }
    }

    /// Hook coordinates per logical pixel: the macOS tap reports points,
    /// Windows and X11 report physical pixels.
    fn hook_units(scale: f64) -> f64 {
        if cfg!(target_os = "macos") {
            1.0
        } else {
            scale
        }
    }

    fn segment_under(&self, s: &Session, at: Point) -> Option<usize> {
        let k = Self::hook_units(s.scale);
        segment_at((at.x - s.center.x) / k, (at.y - s.center.y) / k, self.segments.len(), self.config.dead_zone)
    }

    fn on_hook(&mut self, out: Output) {
        if self.presenter.is_none() {
            return;
        }
        let now = Instant::now();
        match out {
            Output::Open(at) => {
                if self.segments.is_empty() {
                    return;
                }
                let scale = self.presenter.as_ref().unwrap().scale_at(at);
                let cache = RingCache::new(&self.ring(scale, None));
                let session = Session {
                    center: at,
                    scale,
                    hover: None,
                    cache,
                    tween: HoverTween::new(now),
                    opened: now,
                    closing: None,
                    pointer: None,
                    dirty: true,
                };
                // First frame of the pop-in: fully transparent, so showing is invisible.
                let frame = self.render(&session, now);
                self.presenter.as_mut().unwrap().show(at, &frame, scale);
                self.last_frame = now;
                self.session = Some(session);
            }
            Output::Move(at) => {
                let Some(s) = self.session.as_ref() else { return };
                if s.closing.is_some() {
                    return;
                }
                let hover = self.segment_under(s, at);
                let pointer = self.pointer_angle(s, at);
                let n = self.segments.len();
                let s = self.session.as_mut().unwrap();
                if hover != s.hover {
                    s.hover = hover;
                    s.tween.set(hover, n, now);
                    s.dirty = true;
                }
                // Redraw for the hub notch only when it moved visibly.
                let moved = match (pointer, s.pointer) {
                    (Some(a), Some(b)) => anim::shortest_delta(a, b).abs() > 0.02,
                    (a, b) => a.is_some() != b.is_some(),
                };
                if moved {
                    s.pointer = pointer;
                    s.dirty = true;
                }
            }
            Output::Close(at) => {
                let Some(s) = self.session.as_ref() else { return };
                if s.closing.is_some() {
                    return;
                }
                let index = self.segment_under(s, at);
                let chosen = index.and_then(|i| self.config.items.get(i).cloned());
                let n = self.segments.len();
                let s = self.session.as_mut().unwrap();
                s.hover = index;
                s.tween.set(index, n, now);
                s.closing = Some((now, chosen.is_some()));
                s.dirty = true;
                if chosen.is_some() && self.config.sound {
                    orbit_sys::sound::play_select();
                }
                if let Some(item) = chosen {
                    // Launch while the ring fades out; run on the main thread.
                    let proxy = self.proxy.clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(Duration::from_millis(40));
                        let _ = proxy.send_event(UserEvent::RunAction(Box::new(item)));
                    });
                }
            }
        }
    }

    /// Pointer direction (clockwise from the top) once it is a few pixels
    /// away from the centre.
    fn pointer_angle(&self, s: &Session, at: Point) -> Option<f32> {
        let k = Self::hook_units(s.scale);
        let (dx, dy) = ((at.x - s.center.x) / k, (at.y - s.center.y) / k);
        (dx.hypot(dy) > 8.0).then(|| orbit_core::geometry::angle_from_top(dx, dy) as f32)
    }

    fn render(&self, s: &Session, now: Instant) -> Pixmap {
        let ring = Ring { pointer: s.pointer, ..self.ring(s.scale, s.hover) };
        let frame = s.cache.frame_animated(&ring, s.tween.at(now));
        orbit_render::compose(frame, s.pose(now))
    }

    /// Draws the next animation frame if one is due and returns when the
    /// following one is needed (`None` = nothing is moving, sleep).
    fn tick(&mut self) -> Option<Instant> {
        let now = Instant::now();
        let s = self.session.as_ref()?;
        if !s.dirty && !s.animating(now) {
            return None;
        }
        // Coalesce bursts of mouse events to ~60 fps.
        let due = self.last_frame + anim::FRAME;
        if now < due {
            return Some(due);
        }
        if s.finished(now) {
            self.presenter.as_mut()?.hide();
            self.session = None;
            return None;
        }
        let frame = self.render(s, now);
        self.presenter.as_mut()?.update(&frame);
        self.last_frame = now;
        let s = self.session.as_mut()?;
        s.dirty = false;
        // Keep ticking until the last frame (at progress 1) has been drawn.
        (s.animating(now) || s.closing.is_some()).then_some(now + anim::FRAME)
    }

    /// (Re)loads config.json and rebuilds the segment list. Icons are cached
    /// by item, so unchanged items are not decoded again.
    fn reload_config(&mut self) {
        let path = paths::config_file();
        let config = match OrbitConfig::load_or_create(&path) {
            Ok(c) => c,
            Err(e) => {
                log::error!("ignoring invalid {}: {e}", path.display());
                return;
            }
        };
        let mut used = HashMap::new();
        self.segments = config
            .items
            .iter()
            .map(|item| {
                let key = format!("{:?}|{}|{:?}", item.kind, item.value, item.icon);
                let icon = self.icon_cache.remove(&key).unwrap_or_else(|| load_icon(item));
                used.insert(key, icon.clone());
                let monogram = item
                    .monogram()
                    .filter(|m| self.font.as_ref().is_none_or(|f| f.covers(m)))
                    .unwrap_or_else(|| orbit_render::monogram(&item.title));
                Segment { title: item.title.clone(), icon, monogram }
            })
            .collect();
        self.icon_cache = used;
        self.theme = match config.theme {
            orbit_core::RingTheme::Orbit => Theme::orbit(),
        };
        anim::set_reduced_motion(orbit_sys::motion::prefers_reduced_motion());
        if let Some(engine) = &self.engine {
            engine.configure(config.enabled, config.hold_ms, config.drag_threshold);
        }
        if let Some(tray) = &mut self.tray {
            tray.set_enabled(config.enabled);
        }
        self.config = config;
    }

    fn toggle_enabled(&mut self) {
        let mut cfg = self.config.clone();
        cfg.enabled = !cfg.enabled;
        if let Err(e) = cfg.save(&paths::config_file()) {
            log::error!("could not save config: {e}");
        }
        self.reload_config();
    }
}

/// Decodes an item's native icon, downscaled so a 1024 px app icon does not
/// sit in memory.
fn load_icon(item: &OrbitItem) -> Option<Pixmap> {
    const MAX: u32 = 96;
    let pm = orbit_render::decode_png(&icons::for_item(item)?)?;
    if pm.width() <= MAX && pm.height() <= MAX {
        return Some(pm);
    }
    let k = MAX as f32 / pm.width().max(pm.height()) as f32;
    let (w, h) = ((pm.width() as f32 * k).round() as u32, (pm.height() as f32 * k).round() as u32);
    let mut small = Pixmap::new(w.max(1), h.max(1))?;
    let paint = orbit_render::PixmapPaint { quality: orbit_render::FilterQuality::Bicubic, ..Default::default() };
    small.draw_pixmap(0, 0, pm.as_ref(), &paint, orbit_render::Transform::from_scale(k, k), None);
    Some(small)
}

pub fn open_settings() {
    match paths::sibling_exe("theorbit-settings") {
        Some(exe) => {
            if let Err(e) = std::process::Command::new(exe).spawn() {
                log::error!("could not start settings: {e}");
            }
        }
        None => {
            // No settings binary next to us: fall back to the raw config file.
            let _ = open::that_detached(paths::config_file());
        }
    }
}

fn reveal_config() {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg("-R").arg(paths::config_file()).spawn();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("explorer").arg(format!("/select,{}", paths::config_file().display())).spawn();
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let _ = open::that_detached(paths::config_dir());
}

/// Installs the hook on a worker thread, waiting for macOS Accessibility
/// access if needed, and reports progress through the event loop.
fn start_hook(proxy: EventLoopProxy<UserEvent>) {
    std::thread::spawn(move || {
        let mut prompted = false;
        loop {
            let sink = Mutex::new(proxy.clone());
            let result = orbit_input::start(move |replay| {
                Engine::new(replay, move |out| {
                    let _ = sink.lock().unwrap().send_event(UserEvent::Hook(out));
                })
            });
            let pid = std::process::id();
            match result {
                Ok(engine) => {
                    let _ = proxy.send_event(UserEvent::HookStarted(engine));
                    let _ = proxy.send_event(UserEvent::Status(HookStatus { running: true, pid, ..Default::default() }));
                    return;
                }
                Err(StartError::PermissionDenied) => {
                    let _ = proxy
                        .send_event(UserEvent::Status(HookStatus { needs_permission: true, pid, ..Default::default() }));
                    if !prompted {
                        prompted = true;
                        orbit_input::has_permission(true);
                        open_settings();
                    }
                    std::thread::sleep(Duration::from_millis(1500));
                }
                Err(e) => {
                    log::error!("mouse hook failed: {e}");
                    let _ = proxy.send_event(UserEvent::Status(HookStatus {
                        error: Some(e.to_string()),
                        pid,
                        ..Default::default()
                    }));
                    return;
                }
            }
        }
    });
}

/// Reloads the config whenever settings (or a text editor) saves it.
fn watch_config(proxy: EventLoopProxy<UserEvent>) -> Option<notify::RecommendedWatcher> {
    use notify::{RecursiveMode, Watcher};
    let target = paths::config_file();
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if let Ok(ev) = res {
            if ev.paths.iter().any(|p| p.file_name() == target.file_name()) {
                let _ = proxy.send_event(UserEvent::ConfigChanged);
            }
        }
    })
    .ok()?;
    // Watch the directory: saves replace the file via rename.
    watcher.watch(&paths::config_dir(), RecursiveMode::NonRecursive).ok()?;
    Some(watcher)
}

impl ApplicationHandler<UserEvent> for App {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        if cause != StartCause::Init {
            return;
        }
        event_loop.set_control_flow(ControlFlow::Wait);
        self.presenter = overlay::create();
        if self.presenter.is_none() {
            log::error!("could not create the overlay window");
        }
        self.tray = tray::Tray::create(self.proxy.clone(), self.config.enabled);
        self.watcher = watch_config(self.proxy.clone());
        start_hook(self.proxy.clone());
    }

    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        match self.tick() {
            Some(next) => event_loop.set_control_flow(ControlFlow::WaitUntil(next)),
            None => event_loop.set_control_flow(ControlFlow::Wait),
        }
    }

    fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: WindowEvent) {}

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::Hook(out) => self.on_hook(out),
            UserEvent::HookStarted(engine) => {
                engine.configure(self.config.enabled, self.config.hold_ms, self.config.drag_threshold);
                self.engine = Some(engine);
            }
            UserEvent::Status(status) => status.write(),
            UserEvent::ConfigChanged => self.reload_config(),
            UserEvent::RunAction(item) => {
                if let Err(e) = actions::run(&item) {
                    log::error!("{}: {e}", item.title);
                }
            }
            UserEvent::Tray(cmd) => match cmd {
                tray::Command::Toggle => self.toggle_enabled(),
                tray::Command::Settings => open_settings(),
                tray::Command::RevealConfig => reveal_config(),
                tray::Command::Quit => {
                    HookStatus::default().write();
                    event_loop.exit();
                }
            },
        }
    }
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // One resident instance only: a second launch just opens settings.
    let _ = std::fs::create_dir_all(paths::config_dir());
    let lock = std::fs::File::create(paths::lock_file("theorbit")).expect("cannot create lock file");
    if lock.try_lock().is_err() {
        log::info!("TheOrbit is already running; opening settings");
        open_settings();
        return;
    }
    let first_run = !paths::config_file().exists();

    #[allow(unused_mut)]
    let mut builder = EventLoop::<UserEvent>::with_user_event();
    #[cfg(target_os = "macos")]
    {
        use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};
        // Menu-bar only: no Dock icon.
        builder.with_activation_policy(ActivationPolicy::Accessory);
    }
    let event_loop = builder.build().expect("cannot create event loop");

    let mut app = App {
        proxy: event_loop.create_proxy(),
        config: OrbitConfig::default(),
        engine: None,
        presenter: None,
        tray: None,
        session: None,
        segments: Vec::new(),
        icon_cache: HashMap::new(),
        font: Font::system(),
        theme: Theme::default(),
        watcher: None,
        last_frame: Instant::now(),
    };
    app.reload_config();
    if first_run {
        open_settings();
    }
    if let Err(e) = event_loop.run_app(&mut app) {
        log::error!("event loop failed: {e}");
    }
    drop(lock);
}
