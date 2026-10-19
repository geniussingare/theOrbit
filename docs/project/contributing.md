# Contributing

Fixes, ideas and new platform support are welcome.

## Set up

Install [Rust](https://rustup.rs) (stable), then:

```sh
git clone https://github.com/geniussingare/theOrbit.git
cd theOrbit
cargo test --workspace
```

On Debian and Ubuntu you also need `libxkbcommon-dev` and `libgl1-mesa-dev`.

## Before you open a pull request

The same checks run on every push for macOS, Windows and Linux.

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Both must pass. Add a test when you change the gesture engine, the geometry, the config model, or shortcut parsing. Those have tests today.

## Project layout

```text
crates/
  orbit-core      config model and ring geometry
  orbit-input     global right-button hook and gesture engine
  orbit-render    CPU renderer and animations
  orbit-sys       actions, icons, sound, file locations
  orbit-app       theorbit: tray app and overlay window
  orbit-settings  theorbit-settings: the settings window
assets/icons      app icons
scripts/          macOS packaging
website/          the landing page
docs/             these docs
```

Read [Architecture](../reference/architecture.md) for how the crates fit together.

## Try the renderer on its own

The render crate has examples that don't need the full app.

```sh
# Draw a sample ring to a PNG
cargo run -p orbit-render --example preview -- ring.png

# Other examples
cargo run -p orbit-render --example motion
cargo run -p orbit-sys --example tick
```

To regenerate the app icons:

```sh
cargo run -p orbit-render --example icons -- assets/icons
```

On macOS, run `iconutil` afterwards to produce the `.icns` file.

## Common changes

??? example "Add a new item type"
    1. Add a variant to `ActionType` in `orbit-core/src/config.rs`.
    2. Handle it in `run` in `orbit-sys/src/actions.rs`.
    3. List it in the settings window's type menu in `orbit-settings/src/main.rs`.
    4. Document it in [Menu items](../usage/items.md).

??? example "Add a new setting"
    1. Add a field to `OrbitConfig` and its default in `orbit-core/src/config.rs`.
    2. Read it where it applies, usually in `orbit-app/src/main.rs`.
    3. Add a control in the settings window.
    4. Add a row to the [configuration reference](../reference/configuration.md).

## Work on the docs

The docs use [MkDocs Material](https://squidfunnel.github.io/mkdocs-material/). Each page has an edit icon that opens it on GitHub.

```sh
pip install -r requirements-docs.txt
mkdocs serve
```

Open <http://127.0.0.1:8000>. The landing page is a single file at `website/index.html`.

Vercel deploys the website and docs whenever `main` changes, using the settings in `vercel.json`. Pull requests that touch the docs also run a strict build, so a broken link fails the check.

## Release

Releases are built by GitHub Actions when you push a version tag.

1. Update `version` in the root `Cargo.toml` and add an entry to the [changelog](changelog.md).
2. Commit, then tag and push:

    ```sh
    git tag v0.1.0
    git push origin v0.1.0
    ```

3. The **release** workflow builds macOS, Windows and Linux, then attaches `TheOrbit-macos.zip`, `TheOrbit-windows.zip` and `TheOrbit-linux.tar.gz` to a new GitHub release with generated notes.

## License

By contributing, you agree that your work is licensed under the [Apache License 2.0](https://github.com/geniussingare/theOrbit/blob/main/LICENSE).
