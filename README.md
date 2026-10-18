# TheOrbit

A radial quick menu around your cursor. **Hold the right mouse button** anywhere and a ring of apps and actions opens around the pointer. Move onto an item and **release** to run it. A normal short right-click still opens the usual context menu.

- **Lightweight:** native Rust, no web view. The resident process uses about 16 MB and 0% CPU while idle, and it only does work while the right button is held.
- **Cross-platform:** macOS, Windows and Linux (X11). The core crates are platform-free, so Android and iOS/iPadOS can reuse them later.
- **Configurable:** apps, websites, files/folders, keyboard shortcuts and shell commands.
- **Calm motion:** the ring springs open, the highlight glides between items, and the ring swells away when you choose one. A soft tick confirms the choice (optional). All motion follows the OS "Reduce motion" setting.

## How it works

| Gesture | Result |
| --- | --- |
| Right-click (released before the hold delay) | Normal context menu; the click is replayed to the app |
| Hold right-click (350 ms by default) | The orbit opens, centred on the cursor |
| Move onto a segment, then release | Runs that item |
| Release in the centre | Cancels |
| Right-drag before the hold delay | Passed through untouched (games, CAD, gestures) |

## Project layout

```
crates/
  orbit-core      config model + segment geometry (platform-free, shared with future mobile apps)
  orbit-input     global right-button hook + gesture state machine
                  (macOS CGEventTap, Windows WH_MOUSE_LL, X11 passive grab + XTEST)
  orbit-render    CPU renderer + animations for the ring (tiny-skia); also draws the orbit logo
  orbit-sys       actions (launch/open/shortcut/shell), native icons, select sound, file locations
  orbit-app       `theorbit`: resident tray app + native overlay window per OS
  orbit-settings  `theorbit-settings`: egui settings window, started on demand from the tray
assets/icons      app icons (regenerate: cargo run -p orbit-render --example icons -- assets/icons,
                  then iconutil for the .icns)
scripts/          packaging scripts
```

## Build and run

You need Rust (stable). Install it with `curl https://sh.rustup.rs -sSf | sh`.

```sh
cargo test --workspace

# macOS: one-time local signing identity (keeps the Accessibility permission across rebuilds)
scripts/create-signing-cert.sh

# macOS: build TheOrbit.app (menu-bar app) and launch it
scripts/bundle-macos.sh
open target/release/bundle/TheOrbit.app

# Windows / Linux: build both binaries (keep them in the same folder)
cargo build --release -p orbit-app -p orbit-settings
./target/release/theorbit
```

Settings live in `config.json`:

- macOS: `~/Library/Application Support/TheOrbit/`
- Windows: `%APPDATA%\TheOrbit\`
- Linux: `~/.config/TheOrbit/`

The settings window edits this file. You can also edit it by hand: the app reloads it automatically.

```json
{
  "enabled": true,
  "holdMs": 350,
  "radius": 150,
  "deadZone": 42,
  "dragThreshold": 8,
  "theme": "orbit",
  "sound": true,
  "items": [
    { "title": "Safari", "type": "app", "value": "/Applications/Safari.app" },
    { "title": "Screenshot", "type": "shortcut", "value": "cmd+shift+4" },
    { "title": "Docs", "type": "url", "value": "https://docs.rs" },
    { "title": "Downloads", "type": "file", "value": "~/Downloads" },
    { "title": "Build", "type": "shell", "value": "cd ~/code/app && make", "icon": "B" }
  ]
}
```

Item types:

- `app`: a path, bundle id (`com.apple.Safari`), app name, `.exe`, or `.desktop` file.
- `url`: a URL.
- `file`: any file or folder.
- `shortcut`: keys such as `ctrl+alt+t`.
- `shell`: a shell command.

The optional `icon` field takes 1–2 letters and replaces the auto-detected icon.

## Platform notes

- **macOS:** TheOrbit needs **Accessibility** access (System Settings → Privacy & Security → Accessibility). The hook starts as soon as access is granted. Builds signed with the local identity from `scripts/create-signing-cert.sh` keep that permission across rebuilds. Ad-hoc builds need it re-granted each time.
- **Windows:** no permissions needed.
- **Linux:** needs an **X11** session. Wayland blocks global mouse grabs. A compositor is needed for the ring's transparency. The tray uses StatusNotifierItem (KDE and most desktops; GNOME needs the AppIndicator extension).

## Roadmap

- Native app icons on Windows and Linux (they currently show monograms).
- Installers (`.dmg`, `.msi`, `.deb`/AppImage) and code signing.
- Android (floating bubble) and iOS/iPadOS (in-app orbit), reusing `orbit-core` and `orbit-render`.
