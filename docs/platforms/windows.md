# Windows

TheOrbit needs no special permissions on Windows. It installs a low-level mouse hook, so the ring works over any window.

## Run it

1. Put `theorbit.exe` and `theorbit-settings.exe` in the same folder. Keep them together.
2. Run `theorbit.exe`.
3. Look for the violet icon in the system tray. It may be hidden under the **^** arrow.

If SmartScreen says Windows protected your PC, choose **More info**, then **Run anyway**. Releases are not yet code signed. Signing is on the [roadmap](../project/changelog.md#roadmap).

To start TheOrbit when you sign in, turn on **Launch at login** in the [settings window](../usage/settings.md#preferences).

## Defaults and shortcuts

- The default ring has Explorer, Edge, Terminal, Notepad, Snip, Downloads, Settings and Google.
- `win` is the Windows key in [shortcut items](../usage/items.md#keyboard-shortcuts). The default Snip item sends `win+shift+s`.
- `ms-settings:` is a `url` item that opens the Windows Settings app.
- Shell items run through `cmd /C` with no console window.
- Your settings live in `%APPDATA%\TheOrbit\`.

## Troubleshooting

??? question "I can't find the tray icon."
    Click the **^** arrow beside the clock. Drag the TheOrbit icon onto the taskbar to keep it visible.

??? question "Settings won't open."
    `theorbit-settings.exe` must sit in the same folder as `theorbit.exe`. You can also start TheOrbit a second time, which opens the settings window.

??? question "The ring doesn't open in one particular app."
    Some apps read the mouse in their own way. Check that **Orbit Enabled** is on, then see whether the ring opens on the desktop. If it only fails in one app, please [open an issue](https://github.com/geniussingare/theOrbit/issues) and name the app.

??? question "My drag in a game or design tool behaves differently."
    Moving the pointer more than `dragThreshold` pixels before the hold delay ends passes the drag through to the app. Increase `holdMs` or reduce `dragThreshold` if you want to tune it. See [Gestures](../usage/gestures.md#tune-the-feel).
