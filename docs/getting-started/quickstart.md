# Quickstart

Five minutes from first launch to your own menu.

## 1. Start TheOrbit

Open the app. A small icon appears in your menu bar (macOS) or system tray (Windows, Linux). There is no main window and no Dock icon.

On macOS, allow **Accessibility** access the first time. TheOrbit opens its settings window and waits for the permission, so you only have to switch it on.

## 2. Open the ring

Hold the **right mouse button** anywhere on screen. After about a third of a second, a ring of items opens around your pointer.

Move onto an item and **release** to run it. Release in the centre to cancel.

A quick right-click still opens the normal context menu, so nothing you already do changes.

## 3. Make it yours

Click the TheOrbit icon and choose **Settings…**. In the settings window you can:

- Add an app, website, file or folder, keyboard shortcut, or shell command.
- Drag items to reorder them. The first item sits at the top of the ring, and the rest follow clockwise.
- Change the hold delay under **Preferences**.

Changes apply as soon as you make them.

## 4. Or edit the file

Everything is stored in one JSON file. Choose **Show Config File** from the tray icon to find it, or see [where the file lives](../reference/configuration.md#where-the-file-lives). A minimal example:

```json
{
  "items": [
    { "title": "Docs", "type": "url", "value": "https://docs.rs" },
    { "title": "Downloads", "type": "file", "value": "~/Downloads" },
    { "title": "Build", "type": "shell", "value": "cd ~/code/app && make", "icon": "B" }
  ]
}
```

TheOrbit reloads the file when you save it. Fields you leave out use their defaults.

## Where next

- Learn every gesture in [Gestures](../usage/gestures.md).
- See what each item type can do in [Menu items](../usage/items.md).
- Look up every setting in the [configuration reference](../reference/configuration.md).
