#!/usr/bin/env bash
# Builds target/release/bundle/TheOrbit.app (menu-bar app, both binaries inside).
# Usage: scripts/bundle-macos.sh [--universal]
set -euo pipefail
cd "$(dirname "$0")/.."

VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
APP=target/release/bundle/TheOrbit.app

if [[ "${1:-}" == "--universal" ]]; then
  rustup target add aarch64-apple-darwin x86_64-apple-darwin >/dev/null
  for t in aarch64-apple-darwin x86_64-apple-darwin; do
    cargo build --release --target "$t" -p orbit-app -p orbit-settings
  done
  BIN_DIR=target/universal
  mkdir -p "$BIN_DIR"
  for b in theorbit theorbit-settings; do
    lipo -create -output "$BIN_DIR/$b" "target/aarch64-apple-darwin/release/$b" "target/x86_64-apple-darwin/release/$b"
  done
else
  cargo build --release -p orbit-app -p orbit-settings
  BIN_DIR=target/release
fi

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN_DIR/theorbit" "$BIN_DIR/theorbit-settings" "$APP/Contents/MacOS/"
cp assets/icons/icon.icns "$APP/Contents/Resources/TheOrbit.icns"

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>TheOrbit</string>
  <key>CFBundleDisplayName</key><string>TheOrbit</string>
  <key>CFBundleIdentifier</key><string>app.theorbit.desktop</string>
  <key>CFBundleExecutable</key><string>theorbit</string>
  <key>CFBundleIconFile</key><string>TheOrbit</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>${VERSION}</string>
  <key>CFBundleVersion</key><string>${VERSION}</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>LSUIElement</key><true/>
  <key>NSHighResolutionCapable</key><true/>
  <key>LSApplicationCategoryType</key><string>public.app-category.productivity</string>
</dict>
</plist>
PLIST

# Sign with the stable local identity when it exists (see
# scripts/create-signing-cert.sh) so macOS keeps the Accessibility permission
# across rebuilds. Otherwise fall back to an ad-hoc signature, which changes
# every build and needs TheOrbit re-enabled in Accessibility each time.
IDENTITY="${ORBIT_SIGN_IDENTITY:-TheOrbit Local Signing}"
if security find-identity -p codesigning | grep -q "$IDENTITY"; then
  codesign --force --deep --sign "$IDENTITY" "$APP" >/dev/null
  SIGNED_WITH="$IDENTITY"
else
  codesign --force --deep --sign - "$APP" >/dev/null
  SIGNED_WITH="ad-hoc (run scripts/create-signing-cert.sh once to keep permissions across builds)"
fi
echo "Built $APP ($(du -sh "$APP" | cut -f1)), signed: $SIGNED_WITH"
