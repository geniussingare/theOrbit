//! Menu-bar / system-tray icon. macOS and Windows use `tray-icon`; Linux uses
//! a StatusNotifierItem over D-Bus (`ksni`), which needs no GTK.

use crate::UserEvent;
use orbit_render::logo;
use winit::event_loop::EventLoopProxy;

pub enum Command {
    Toggle,
    Settings,
    RevealConfig,
    Quit,
}

const TOOLTIP: &str = "TheOrbit: hold right-click for the orbit";

fn icon_rgba(size: u32) -> Vec<u8> {
    // macOS tints template images itself; elsewhere use the brand colour.
    let color = if cfg!(target_os = "macos") {
        orbit_render::Color::BLACK
    } else {
        orbit_render::Color::from_rgba8(139, 114, 255, 255)
    };
    logo::to_straight_rgba(&logo::mark(size, color))
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
pub struct Tray {
    _icon: tray_icon::TrayIcon,
    toggle: tray_icon::menu::CheckMenuItem,
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
impl Tray {
    pub fn create(proxy: EventLoopProxy<UserEvent>, enabled: bool) -> Option<Self> {
        use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
        use tray_icon::{Icon, TrayIconBuilder};

        let toggle = CheckMenuItem::new("Orbit Enabled", true, enabled, None);
        let settings = MenuItem::new("Settings…", true, None);
        let reveal = MenuItem::new("Show Config File", true, None);
        let quit = MenuItem::new("Quit TheOrbit", true, None);
        let menu = Menu::new();
        menu.append_items(&[
            &toggle,
            &PredefinedMenuItem::separator(),
            &settings,
            &reveal,
            &PredefinedMenuItem::separator(),
            &quit,
        ])
        .ok()?;

        let ids = [
            (toggle.id().clone(), Command::Toggle as u8),
            (settings.id().clone(), Command::Settings as u8),
            (reveal.id().clone(), Command::RevealConfig as u8),
            (quit.id().clone(), Command::Quit as u8),
        ];
        MenuEvent::set_event_handler(Some(move |ev: MenuEvent| {
            let Some((_, code)) = ids.iter().find(|(id, _)| *id == ev.id) else { return };
            let cmd = match code {
                0 => Command::Toggle,
                1 => Command::Settings,
                2 => Command::RevealConfig,
                _ => Command::Quit,
            };
            let _ = proxy.send_event(UserEvent::Tray(cmd));
        }));

        const SIZE: u32 = 44;
        let icon = Icon::from_rgba(icon_rgba(SIZE), SIZE, SIZE).ok()?;
        let builder = TrayIconBuilder::new().with_menu(Box::new(menu)).with_tooltip(TOOLTIP);
        #[cfg(target_os = "macos")]
        let builder = builder.with_icon_templated(icon);
        #[cfg(not(target_os = "macos"))]
        let builder = builder.with_icon(icon);
        let tray = builder
            .build()
            .map_err(|e| log::error!("tray icon failed: {e}"))
            .ok()?;
        Some(Self { _icon: tray, toggle })
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.toggle.set_checked(enabled);
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub struct Tray {
    handle: ksni::blocking::Handle<SniTray>,
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub struct SniTray {
    proxy: EventLoopProxy<UserEvent>,
    enabled: bool,
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
impl ksni::Tray for SniTray {
    fn id(&self) -> String {
        "theorbit".into()
    }
    fn title(&self) -> String {
        "TheOrbit".into()
    }
    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip { title: TOOLTIP.into(), ..Default::default() }
    }
    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        const SIZE: u32 = 32;
        // SNI wants ARGB32 in network byte order.
        let data = icon_rgba(SIZE).as_chunks::<4>().0.iter().flat_map(|p| [p[3], p[0], p[1], p[2]]).collect();
        vec![ksni::Icon { width: SIZE as i32, height: SIZE as i32, data }]
    }
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::{CheckmarkItem, StandardItem};
        let send = |cmd: fn() -> Command| {
            Box::new(move |t: &mut Self| {
                let _ = t.proxy.send_event(UserEvent::Tray(cmd()));
            }) as Box<dyn Fn(&mut Self) + Send>
        };
        vec![
            CheckmarkItem { label: "Orbit Enabled".into(), checked: self.enabled, activate: send(|| Command::Toggle), ..Default::default() }.into(),
            ksni::MenuItem::Separator,
            StandardItem { label: "Settings…".into(), activate: send(|| Command::Settings), ..Default::default() }.into(),
            StandardItem { label: "Show Config File".into(), activate: send(|| Command::RevealConfig), ..Default::default() }.into(),
            ksni::MenuItem::Separator,
            StandardItem { label: "Quit TheOrbit".into(), activate: send(|| Command::Quit), ..Default::default() }.into(),
        ]
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
impl Tray {
    pub fn create(proxy: EventLoopProxy<UserEvent>, enabled: bool) -> Option<Self> {
        use ksni::blocking::TrayMethods;
        match (SniTray { proxy, enabled }).spawn() {
            Ok(handle) => Some(Self { handle }),
            Err(e) => {
                log::warn!("no system tray available ({e}); use the settings app instead");
                None
            }
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.handle.update(|t| t.enabled = enabled);
    }
}
