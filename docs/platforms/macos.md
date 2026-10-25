# macOS

TheOrbit runs on macOS 11 or later, on Apple silicon and Intel. It is a menu-bar app, so it has no Dock icon.

## Allow Accessibility access

macOS only lets an app see the mouse everywhere if you allow it. TheOrbit needs that to notice the right button.

1. Open TheOrbit. It asks for access and opens its settings window.
2. Open **System Settings → Privacy & Security → Accessibility**.
3. Switch **TheOrbit** on.

TheOrbit checks again every second or two, so the ring starts working as soon as you allow it. You do not need to restart.

The settings window shows a banner, **Allow Accessibility access**, until the permission is granted. Its button opens the right System Settings page.

## Keep the permission after you rebuild

!!! info "Only for people who build from source"
    If you use a downloaded release, skip this section.

macOS ties the permission to the app's code signature. A build with an ad-hoc signature has a new signature every time, so macOS forgets the permission and you must switch TheOrbit off and on again after each rebuild.

To avoid that, create a local signing identity once:

```sh
scripts/create-signing-cert.sh
```

This adds a self-signed identity named **TheOrbit Local Signing** to your login keychain. `scripts/bundle-macos.sh` uses it automatically, so the permission stays in place across rebuilds. To use a different identity, set `ORBIT_SIGN_IDENTITY` to its name before you build.

## Open an app that macOS won't verify

Releases are not yet signed with an Apple Developer ID, so macOS may warn about an unverified developer the first time you open the app.

1. Control-click `TheOrbit.app` and choose **Open**.
2. Choose **Open** again in the dialog.

If you downloaded the app and macOS still refuses, remove the download flag:

```sh
xattr -dr com.apple.quarantine /Applications/TheOrbit.app
```

Signing and installers are on the [roadmap](../project/changelog.md#roadmap).

## Defaults and shortcuts

- The default ring has Finder, Safari, Terminal, Notes, Screenshot, Downloads, Settings and Google.
- `cmd` is the Command key and `opt` is Option in [shortcut items](../usage/items.md#keyboard-shortcuts).
- Your settings live in `~/Library/Application Support/TheOrbit/`.

## Troubleshooting

??? question "I hold the right button and nothing happens."
    Check **Accessibility** in System Settings. If TheOrbit is already listed, switch it off and on again, or remove it from the list with the minus button and open the app once more. Make sure **Orbit Enabled** is checked in the menu-bar icon.

??? question "It stopped working after I rebuilt the app."
    The new build has a different signature. Switch the permission off and on, or run `scripts/create-signing-cert.sh` once so future builds keep it.

??? question "A screenshot shortcut item does nothing."
    The shortcut is sent as key presses. Confirm the keys match the shortcut you have set in System Settings, and that Accessibility access is on.
