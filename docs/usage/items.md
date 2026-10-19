# Menu items

Every ring item has a title, a type, and a value. The type decides what the value means.

```json
{ "title": "Docs", "type": "url", "value": "https://docs.rs" }
```

## Item types

| Type | Value | What it does |
| --- | --- | --- |
| `app` | A path, bundle id, app name, `.exe`, or `.desktop` file | Launches the application. |
| `url` | A web address | Opens it in your default handler. |
| `file` | A file or folder path | Opens it with the default app. A leading `~` means your home folder. |
| `shortcut` | Keys such as `ctrl+alt+t` | Presses those keys. |
| `shell` | A command line | Runs it in the background. |

An item with an empty value does nothing, and the error is written to the log.

## Apps

=== "macOS"

    TheOrbit looks at the value to decide how to launch the app:

    | You write | Example | Launched as |
    | --- | --- | --- |
    | A path starting with `/` or `~` | `/Applications/Safari.app` | The app at that path |
    | A bundle id | `com.apple.Safari` | The app with that identifier |
    | An app name | `Visual Studio Code` | The app with that name |

=== "Windows"

    Anything that the Windows `start` command understands works: an executable name such as `notepad.exe`, a registered app name such as `msedge`, a full path, or a shortcut (`.lnk`) file.

=== "Linux"

    - A `.desktop` file, by absolute path or by name, launches that application.
    - Anything else runs as a shell command.

## Websites and system links

`url` items open in your default handler, so they are not limited to websites. On Windows, `ms-settings:` opens the Settings app, and the default Windows ring uses it.

## Files and folders

`file` items open with the default app for that type. For a folder, that is your file manager.

```json
{ "title": "Downloads", "type": "file", "value": "~/Downloads" }
```

## Keyboard shortcuts

A `shortcut` item presses a key combination in the app you are using. Write it as modifiers and one key, joined with `+`. Case does not matter.

```json
{ "title": "Screenshot", "type": "shortcut", "value": "cmd+shift+4" }
```

| Kind | Names you can use |
| --- | --- |
| Command, Windows or Super key | `cmd`, `command`, `meta`, `super`, `win`, `windows` |
| Control | `ctrl`, `control` |
| Alt or Option | `alt`, `opt`, `option` |
| Shift | `shift` |
| Letters, digits, symbols | A single character such as `t`, `4`, or `/`. Use `plus` for the `+` key. |
| Function keys | `f1` to `f12` |
| Navigation | `up`, `down`, `left`, `right`, `home`, `end`, `pageup`, `pagedown` |
| Editing | `enter` or `return`, `tab`, `space`, `esc` or `escape`, `backspace`, `delete` or `del` |
| Print Screen (Windows and Linux) | `print`, `printscreen`, `prtsc` |

A shortcut needs exactly one key that is not a modifier. `ctrl+shift` has no key, and `a+b` has two, so both are rejected and the reason is written to the log.

## Shell commands

A `shell` item runs a command and does not wait for it.

- On macOS and Linux it runs through your login shell (`$SHELL -lc`), so your usual `PATH` is available even when TheOrbit was started from the Dock or a menu.
- On Windows it runs through `cmd /C`, with no console window.
- Output is discarded and the command gets no input. To see a result, have the command open something, such as a terminal or a file.

```json
{ "title": "Build", "type": "shell", "value": "cd ~/code/app && make", "icon": "B" }
```

!!! warning "Only run commands you trust"
    A shell item runs with your permissions. Treat the config file like any other script, and be careful with config files you did not write.

## Icons

TheOrbit shows each app's own icon when it can find one. Otherwise it draws a monogram, the first letters of the title.

To choose your own, set `icon` to one or two letters or digits:

```json
{ "title": "Build", "type": "shell", "value": "make", "icon": "B" }
```

Emoji and longer text are not supported, because the ring's font cannot draw them. TheOrbit ignores such an `icon` when it loads the file.

## How many items?

Use up to 12. The settings window warns when you add more, because the segments get hard to hit. If you need more, make the ring larger with `radius`.
