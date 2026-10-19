# Changelog and roadmap

Versions follow [Semantic Versioning](https://semver.org/).

## 0.1.0

The first version of TheOrbit.

- A radial menu that opens when you hold the right mouse button, and runs the item you release on.
- A normal right-click still opens the context menu. Right-drags made before the hold delay pass through.
- macOS, Windows and Linux (X11) support.
- Five item types: apps, websites, files and folders, keyboard shortcuts, and shell commands.
- A settings window with drag-to-reorder, and a JSON config file that reloads when you save it.
- A menu-bar and system-tray icon with an on/off switch.
- Smooth, spring-based motion that follows the system Reduce motion setting, and an optional select sound.
- A single resident process that uses about 16 MB and no CPU while idle.

## Roadmap

Planned, in no particular order:

- Native app icons on Windows and Linux. They currently show monograms.
- Installers: `.dmg`, `.msi`, and `.deb` or AppImage.
- Code signing, so macOS and Windows stop warning about unverified developers.
- Android, as a floating bubble.
- iOS and iPadOS, as an in-app orbit.

Android and iOS would reuse `orbit-core` and `orbit-render`.

Have an idea? [Open an issue](https://github.com/geniussingare/theOrbit/issues).
