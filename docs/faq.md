# FAQ and troubleshooting

## About TheOrbit

??? question "Does TheOrbit replace my right-click menu?"
    No. A quick right-click opens the normal context menu. TheOrbit only opens its ring when you hold the button for about a third of a second. The context menu shows up when you release, not when you press.

??? question "Can I still right-drag in games, CAD tools and design apps?"
    Yes. If you move the pointer more than a few pixels before the hold delay ends, TheOrbit hands the drag to the app untouched. If a hold in your app opens the ring by accident, raise `holdMs`. See [Gestures](usage/gestures.md#tune-the-feel).

??? question "How much memory and CPU does it use?"
    It is a native Rust program with no web view. The resident process uses about 16 MB and 0% CPU while idle, and it only works while the right button is held.

??? question "Is it free?"
    Yes. TheOrbit is open source under the [Apache License 2.0](https://github.com/geniussingare/theOrbit/blob/main/LICENSE).

??? question "Does it collect any data?"
    TheOrbit has no account, no analytics and no network code of its own. Your settings stay in a JSON file on your computer. Items of type `url` open your browser, and `shell` and `app` items run programs you chose.

??? question "Why does it need Accessibility access on macOS?"
    macOS only lets an app see the mouse outside its own windows if you allow it. TheOrbit needs that to notice the right button wherever you are. See [macOS setup](platforms/macos.md).

??? question "Why is Linux limited to X11?"
    Wayland blocks global mouse grabs for security, so an app can't watch the right button across other windows. TheOrbit needs that, so it requires an X11 session. See [Linux setup](platforms/linux.md).

??? question "Will it come to Android and iOS?"
    It is on the [roadmap](project/changelog.md#roadmap). The core crates have no platform code so they can be reused.

## Using it

??? question "I hold the right button and nothing opens."
    Check, in this order:

    1. **Orbit Enabled** is checked in the tray icon menu.
    2. On macOS, **Accessibility** access is on for TheOrbit.
    3. On Linux, you are on an X11 session.
    4. You hold for at least the hold delay (350 ms by default) and stay within a few pixels while you wait.

??? question "The ring opens by accident."
    Raise `holdMs`, for example to 500. You can also change **Hold delay** in the settings window.

??? question "The ring feels slow to open."
    Lower `holdMs`. The settings window goes down to 150 ms. Values below 50 ms in the config file are treated as 50.

??? question "An item doesn't do anything."
    TheOrbit writes the reason to its log. Run it from a terminal to read the message. Common causes are an empty value, a path that doesn't exist, or a shortcut with a mistake in its key names. See [Menu items](usage/items.md).

??? question "My emoji icon disappeared."
    Icons are limited to one or two letters or digits, because the ring's font can't draw emoji. TheOrbit ignores other icon values when it loads the file.

??? question "My config change isn't applied."
    TheOrbit ignores a `config.json` that isn't valid JSON and keeps your last working settings. Check for a missing comma or quote, then save again. The error is in the log.

??? question "How do I reset everything?"
    Open **Settings…**, then **Preferences → Advanced → Reset to defaults**. Or delete `config.json` and restart TheOrbit, which writes a fresh one.

## Still stuck?

[Open an issue](https://github.com/geniussingare/theOrbit/issues) and tell us your system, your TheOrbit version, and what you see in the log. To read the log, run `theorbit` from a terminal.
