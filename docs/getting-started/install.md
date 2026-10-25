# Install

TheOrbit runs on macOS 11 or later, Windows, and Linux with an X11 session. Pick a prebuilt download or build it yourself in a few minutes.

## Download

Get the file for your system from the [latest release](https://github.com/geniussingare/theOrbit/releases/latest).

| System | File | What's inside |
| --- | --- | --- |
| macOS (Apple silicon and Intel) | `TheOrbit-0.1.0.dmg` or `TheOrbit-macos.zip` | `TheOrbit.app`, a menu-bar app |
| Windows | `TheOrbit-windows.zip` | `theorbit.exe` and `theorbit-settings.exe` |
| Linux (X11) | `TheOrbit-linux.tar.gz` | `theorbit` and `theorbit-settings` |

!!! note "No release yet?"
    Until a release is published, [build from source](#build-from-source). It takes one command per system.

Keep both programs in the same folder. TheOrbit starts the settings window from the program next to it.

### First launch

=== "macOS"

    1. Open the `.dmg` and drag `TheOrbit.app` to **Applications**. With the zip, unzip it and move the app instead.
    2. Open it. If macOS says it can't verify the developer, Control-click the app, choose **Open**, then confirm.
    3. Allow **Accessibility** access when asked. See [macOS setup](../platforms/macos.md).

=== "Windows"

    1. Unzip into a folder you keep, such as `C:\Tools\TheOrbit`.
    2. Run `theorbit.exe`. If SmartScreen warns about an unrecognized app, choose **More info**, then **Run anyway**.
    3. No permissions are needed. See [Windows setup](../platforms/windows.md).

=== "Linux"

    1. Extract the archive into a folder you keep.
    2. Make sure you are on an **X11** session. See [Linux setup](../platforms/linux.md).
    3. Run `./theorbit`.

The ring is ready when the TheOrbit icon appears in your menu bar or system tray. Hold the right mouse button anywhere to try it.

## Build from source

You need [Rust](https://rustup.rs) (stable).

```sh
curl https://sh.rustup.rs -sSf | sh
git clone https://github.com/geniussingare/theOrbit.git
cd theOrbit
cargo test --workspace
```

=== "macOS"

    ```sh
    # One time: a local signing identity so macOS keeps the Accessibility permission across rebuilds
    scripts/create-signing-cert.sh

    # Build TheOrbit.app and open it
    scripts/bundle-macos.sh
    open target/release/bundle/TheOrbit.app
    ```

    Add `--universal` to `scripts/bundle-macos.sh` for a single app that runs on both Apple silicon and Intel.

=== "Windows"

    ```sh
    cargo build --release -p orbit-app -p orbit-settings
    target\release\theorbit.exe
    ```

=== "Linux"

    Install the build dependencies first. On Debian and Ubuntu:

    ```sh
    sudo apt-get install -y libxkbcommon-dev libgl1-mesa-dev
    ```

    Then build and run:

    ```sh
    cargo build --release -p orbit-app -p orbit-settings
    ./target/release/theorbit
    ```

On Windows and Linux the two binaries land in `target/release/`. Keep them in the same folder if you move them.

## Update

Download the newer release and replace the old files, or pull and rebuild. Your settings stay in place because they live in a separate [config folder](../reference/configuration.md#where-the-file-lives).

## Uninstall

1. Quit TheOrbit from its menu-bar or tray icon.
2. Delete the app or folder.
3. Optional: delete the config folder to remove your settings.

## Next step

Follow the [quickstart](quickstart.md) to open your first ring.
