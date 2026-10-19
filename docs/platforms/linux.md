# Linux

TheOrbit supports Linux sessions that run **X11**.

## What you need

| Need | Why |
| --- | --- |
| An **X11** session | TheOrbit grabs the right mouse button through X11. Wayland blocks global mouse grabs, so it can't work there. |
| A **compositor** | The ring is transparent, and that needs compositing. Most desktops have one on by default. |
| A tray that supports **StatusNotifierItem** | KDE and most desktops do. GNOME needs the AppIndicator extension. |

To check your session, run:

```sh
echo $XDG_SESSION_TYPE
```

It should print `x11`. On many distributions you can pick **Xorg** or **X11** on the login screen, from the gear or session menu.

## Run it

```sh
./theorbit
```

Keep `theorbit` and `theorbit-settings` in the same folder. To build them yourself, see [Install](../getting-started/install.md#build-from-source).

## No tray icon?

If your desktop has no StatusNotifierItem tray, TheOrbit still works. It logs that no system tray is available, and you open the settings window yourself:

```sh
./theorbit-settings
```

Starting `theorbit` a second time also opens the settings window.

On GNOME, install the **AppIndicator and KStatusNotifierItem Support** extension to get the tray icon.

## Defaults and shortcuts

- The default ring has Home, Browser, Terminal, Editor, Downloads and Screenshot.
- The default Terminal and Editor items try a list of common programs in order, for example `x-terminal-emulator || gnome-terminal || konsole || xterm`. Edit them to match your setup.
- `print` is the Print Screen key in [shortcut items](../usage/items.md#keyboard-shortcuts).
- `.desktop` files launch with `gio launch` or `gtk-launch`.
- Your settings live in `~/.config/TheOrbit/`.

## Troubleshooting

??? question "Nothing happens when I hold the right button."
    Run `echo $XDG_SESSION_TYPE`. If it says `wayland`, log out and pick an X11 session at the login screen.

??? question "The ring's background isn't transparent."
    The ring needs a compositor for transparency. Turn on compositing in your desktop's settings, or run a standalone compositor.

??? question "How do I see what went wrong?"
    Run `theorbit` from a terminal. It prints errors there. For more detail, run `RUST_LOG=debug ./theorbit`.

??? question "An item that launches an app does nothing."
    Run the same command from a terminal. Items of type `app` that are not `.desktop` files run as a shell command, so the command must work in your login shell.
