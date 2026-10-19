# Configuration

All of TheOrbit's settings live in one JSON file. The settings window edits the same file.

## Where the file lives

| System | Folder |
| --- | --- |
| macOS | `~/Library/Application Support/TheOrbit/` |
| Windows | `%APPDATA%\TheOrbit\` |
| Linux | `~/.config/TheOrbit/` |

The file is called `config.json`. The folder also holds `status.json`, which the running app writes for the settings window to read, and small `.lock` files that keep a single copy running. Leave those alone.

TheOrbit creates `config.json` with defaults the first time it starts. To open the folder, choose **Show Config File** from the tray icon.

## Editing by hand

TheOrbit watches the folder and reloads `config.json` whenever it changes, so there is no need to restart.

- A field you leave out uses its default.
- If the file is not valid JSON, TheOrbit ignores the change, keeps your last good settings, and writes the error to the log. Fix the file and save again.
- The settings window saves by writing a temporary file and renaming it, so a crash never leaves a half-written config.

## Example

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

## Settings

| Field | Type | Default | What it does |
| --- | --- | --- | --- |
| `enabled` | boolean | `true` | Switches the ring on or off. When off, every mouse event passes through. |
| `holdMs` | number | `350` | Milliseconds to hold the right button before the ring opens. Values below 50 are treated as 50. |
| `radius` | number | `150` | Outer radius of the ring, in logical pixels. |
| `deadZone` | number | `42` | Releasing within this radius of the centre cancels. |
| `dragThreshold` | number | `8` | Moving further than this before the hold delay ends counts as a normal right-drag. |
| `theme` | string | `"orbit"` | The look of the ring. There is one theme today. Unknown names fall back to it. |
| `sound` | boolean | `true` | Plays a soft tick when you choose an item. |
| `items` | list | Depends on your system | The ring items, in order. The first is at the top, and the rest follow clockwise. |

The settings window offers `holdMs` from 150 to 1000, `radius` from 100 to 240, and `deadZone` from 20 to 80. The file accepts other values.

## Items

| Field | Type | Required | What it does |
| --- | --- | --- | --- |
| `title` | string | Yes | The name shown in the ring and the settings window. |
| `type` | string | Yes | One of `app`, `url`, `file`, `shell`, `shortcut`. |
| `value` | string | Yes | What to run. Its meaning depends on `type`. See [Menu items](../usage/items.md). |
| `icon` | string | No | One or two letters or digits to show instead of the automatic icon. Anything else is ignored. |
| `id` | string | No | A unique id. TheOrbit creates one if it is missing. |

## Default items

If you have no config yet, TheOrbit starts with a ring that suits your system.

=== "macOS"

    Finder, Safari, Terminal, Notes, Screenshot (`cmd+shift+4`), Downloads, Settings, Google.

=== "Windows"

    Explorer, Edge, Terminal, Notepad, Snip (`win+shift+s`), Downloads, Settings (`ms-settings:`), Google.

=== "Linux"

    Home, Browser, Terminal, Editor, Downloads, Screenshot (`print`).

## Logging

TheOrbit logs at the `info` level. To see more, set the `RUST_LOG` environment variable before you start it, for example `RUST_LOG=debug`. Run it from a terminal to read the messages.
