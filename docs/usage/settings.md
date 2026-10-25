# Settings and tray

TheOrbit lives in your menu bar or system tray. The settings window starts on demand and closes when you are done, so it adds nothing while you work.

## The tray menu

Click the TheOrbit icon.

| Item | What it does |
| --- | --- |
| **Orbit Enabled** | Switches the ring on or off. A check mark means on. |
| **Settings…** | Opens the settings window. |
| **Show Config File** | Opens the config folder in your file manager. |
| **Quit TheOrbit** | Stops TheOrbit. |

Hover over the icon to see a reminder: hold right-click for the orbit.

!!! tip "No tray icon on Linux?"
    Some desktops do not show tray icons. Run `theorbit-settings` from a terminal to open the settings window instead. See [Linux setup](../platforms/linux.md).

## Opening settings another way

Start TheOrbit a second time. Only one copy runs at once, so the second launch opens the settings window and exits.

## The settings window

The window shows your ring items as tiles.

- **Add** an item with the add button, then choose its type.
- **Drag** a tile to reorder. The first tile is the top of the ring and the rest follow clockwise.
- **Click** a tile to edit its title, value and icon. Press `Esc` to close the editor.
- **Remove** an item from its editor.

A banner at the top tells you when TheOrbit is not running, when macOS still needs the Accessibility permission, or when the ring could not start. It includes a button to fix the first two.

### Preferences

Open **Preferences** at the bottom of the window.

| Option | What it does |
| --- | --- |
| Orbit enabled | Same as the tray menu switch. |
| Launch at login | Starts TheOrbit when you sign in. |
| Sound on select | Plays a soft tick when you choose an item. |
| Hold delay | How long to hold the right button before the ring opens. 150 to 1000 ms. |

Under **Advanced**:

| Option | What it does |
| --- | --- |
| Ring size | Outer radius of the ring. 100 to 240 px. |
| Cancel zone | Radius of the centre where releasing cancels. 20 to 80 px. |
| Reset to defaults | Restores every setting and item. Asks you to confirm first. |

The sliders cover the useful range. You can set values outside it by editing the [config file](../reference/configuration.md).

## Settings and the config file

The settings window and the config file are two views of the same data. Edit either one. TheOrbit watches the file and applies changes as soon as it is saved.
