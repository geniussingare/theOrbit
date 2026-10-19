# Install TheOrbit

TheOrbit runs on macOS 11 or later, Windows, and Linux with an X11 session. Pick your system below. Full documentation is at <https://theorbit.vercel.app/docs/>.

- [macOS](#macos)
- [Windows](#windows)
- [Linux](#linux)
- [Build from source](#build-from-source)
- [Troubleshooting](#troubleshooting)
- [Uninstall](#uninstall)

## macOS

1. Download `TheOrbit-macos.zip` from the [latest release](https://github.com/geniussingare/theOrbit/releases/latest). It runs on Apple silicon and Intel.
2. Unzip it and move `TheOrbit.app` to **Applications**.
3. Open it. If macOS says it can't verify the developer, Control-click the app, choose **Open**, then confirm. If it still refuses, run:

   ```sh
   xattr -dr com.apple.quarantine /Applications/TheOrbit.app
   ```

4. Allow **Accessibility** access. TheOrbit opens its settings window and waits for you. Go to **System Settings → Privacy & Security → Accessibility** and switch **TheOrbit** on. The ring starts working right away.
5. Hold the right mouse button anywhere to try it.

TheOrbit is a menu-bar app, so there is no Dock icon. Your settings live in `~/Library/Application Support/TheOrbit/`.

## Windows

1. Download `TheOrbit-windows.zip` from the [latest release](https://github.com/geniussingare/theOrbit/releases/latest).
2. Unzip it into a folder you keep, such as `C:\Tools\TheOrbit`. Keep `theorbit.exe` and `theorbit-settings.exe` together.
3. Run `theorbit.exe`. If SmartScreen warns about an unrecognized app, choose **More info**, then **Run anyway**.
4. Look for the violet icon in the system tray. It may be under the **^** arrow.

No permissions are needed. Your settings live in `%APPDATA%\TheOrbit\`.

## Linux

TheOrbit needs an **X11** session. Wayland blocks the global mouse grab it relies on. Check yours with:

```sh
echo $XDG_SESSION_TYPE
```

It should print `x11`. If it prints `wayland`, choose an X11 or Xorg session on your login screen.

1. Download `TheOrbit-linux.tar.gz` from the [latest release](https://github.com/geniussingare/theOrbit/releases/latest).
2. Extract it into a folder you keep. Keep `theorbit` and `theorbit-settings` together.
3. Run `./theorbit`.

You also need a compositor, because the ring is transparent, and a tray that supports StatusNotifierItem (KDE and most desktops do). GNOME needs the AppIndicator extension. Without a tray, run `./theorbit-settings` to open the settings window.

Your settings live in `~/.config/TheOrbit/`.

## Build from source

You need [Rust](https://rustup.rs) (stable).

```sh
git clone https://github.com/geniussingare/theOrbit.git
cd theOrbit
cargo test --workspace
```

**macOS**

```sh
# One time: a local signing identity, so macOS keeps the Accessibility permission across rebuilds
scripts/create-signing-cert.sh

scripts/bundle-macos.sh              # add --universal for Apple silicon and Intel
open target/release/bundle/TheOrbit.app
```

**Windows**

```sh
cargo build --release -p orbit-app -p orbit-settings
target\release\theorbit.exe
```

**Linux** (Debian and Ubuntu shown)

```sh
sudo apt-get install -y libxkbcommon-dev libgl1-mesa-dev
cargo build --release -p orbit-app -p orbit-settings
./target/release/theorbit
```

## Troubleshooting

**I hold the right button and nothing opens.**
Check that **Orbit Enabled** is ticked in the tray or menu-bar icon. On macOS, make sure Accessibility access is on for TheOrbit, and switch it off and on again if it is already listed. On Linux, make sure you are on an X11 session.

**It stopped working on macOS after I rebuilt it.**
An ad-hoc signed build gets a new signature every time, so macOS forgets the permission. Run `scripts/create-signing-cert.sh` once, then rebuild.

**There is no tray icon on Linux.**
Your desktop may not support StatusNotifierItem. Run `theorbit-settings` to open the settings window, or install the AppIndicator extension on GNOME.

**Settings won't open.**
`theorbit` starts the settings window from the `theorbit-settings` program next to it. Keep both in the same folder.

**My config change isn't applied.**
TheOrbit ignores a `config.json` that isn't valid JSON and keeps your last good settings. Fix the file and save again. Run `theorbit` from a terminal to read the error.

More answers are in the [FAQ](https://theorbit.vercel.app/docs/faq/).

## Uninstall

Quit TheOrbit from its tray or menu-bar icon, then delete the app or folder. To remove your settings too, delete the config folder listed in your system's section above.
