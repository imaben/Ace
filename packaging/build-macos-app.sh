#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Prefer rustup toolchain over Homebrew rustc if both are installed.
if [[ -d "$HOME/.cargo/bin" ]]; then
  export PATH="$HOME/.cargo/bin:$PATH"
fi

INSTALL_TO_APPLICATIONS=false
BUILD_DMG=false
SKIP_BUILD=false

for arg in "$@"; do
  case "$arg" in
    --install) INSTALL_TO_APPLICATIONS=true ;;
    --dmg) BUILD_DMG=true ;;
    --skip-build) SKIP_BUILD=true ;;
    -h|--help)
      cat <<'EOF'
Usage: packaging/build-macos-app.sh [options]

Builds a release macOS application bundle at dist/Ace.app.

Options:
  --install      Copy dist/Ace.app to /Applications/Ace.app
  --dmg          Also package a distributable DMG in dist/
  --skip-build   Reuse existing target/release/ace binary
  -h, --help     Show this help message
EOF
      exit 0
      ;;
    *)
      echo "Unknown option: $arg" >&2
      exit 1
      ;;
  esac
done

VERSION="$(awk -F'"' '/^version[[:space:]]*=/ { print $2; exit }' "$ROOT_DIR/Cargo.toml")"
VERSION="${VERSION:-0.1.0}"
ARCH="$(uname -m)"

APP_NAME="Ace"
BUNDLE_ID="com.ace.app"
DIST_DIR="$ROOT_DIR/dist"
APP_DIR="$DIST_DIR/${APP_NAME}.app"
CONTENTS_DIR="$APP_DIR/Contents"
MACOS_DIR="$CONTENTS_DIR/MacOS"
RESOURCES_DIR="$CONTENTS_DIR/Resources"
BINARY_SRC="$ROOT_DIR/target/release/ace"
ICON_SRC="$ROOT_DIR/assets/icons/ace.png"

echo "==> Building ${APP_NAME} v${VERSION} (${ARCH}) for macOS"
echo "    rustc: $(rustc --version)"

if [[ "$SKIP_BUILD" != "true" ]]; then
  (cd "$ROOT_DIR" && cargo build --release -p ace-app)
fi

if [[ ! -f "$BINARY_SRC" ]]; then
  echo "Error: release binary not found at $BINARY_SRC" >&2
  exit 1
fi

echo "==> Assembling $APP_DIR"
rm -rf "$APP_DIR"
mkdir -p "$MACOS_DIR" "$RESOURCES_DIR"

cp "$BINARY_SRC" "$MACOS_DIR/ace"
chmod +x "$MACOS_DIR/ace"
printf 'APPL????' > "$CONTENTS_DIR/PkgInfo"

# Generate multi-resolution macOS AppIcon.icns with proper HIG Dock padding.
TMP_ICON_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_ICON_DIR"' EXIT
ICONSET_DIR="$TMP_ICON_DIR/AppIcon.iconset"
mkdir -p "$ICONSET_DIR"

MASTER_PNG="$TMP_ICON_DIR/master_1024.png"
if [[ -f "$ICON_SRC" ]]; then
  if command -v python3 >/dev/null 2>&1 && python3 -c "import PIL" >/dev/null 2>&1; then
    python3 - "$ICON_SRC" "$MASTER_PNG" <<'PY'
import sys
from PIL import Image, ImageFilter

src_path, dst_path = sys.argv[1], sys.argv[2]
src = Image.open(src_path).convert("RGBA")

# Apple macOS HIG: ~824x824 squircle centered on a 1024x1024 transparent canvas
# with a subtle drop shadow so Dock visual weight matches native macOS apps.
canvas_size = 1024
inner_size = 832
offset = (canvas_size - inner_size) // 2

resized = src.resize((inner_size, inner_size), Image.Resampling.LANCZOS)

shadow = Image.new("RGBA", (canvas_size, canvas_size), (0, 0, 0, 0))
alpha = resized.getchannel("A")
shadow_layer = Image.new("RGBA", (inner_size, inner_size), (0, 0, 0, 72))
shadow.paste(shadow_layer, (offset, offset + 12), alpha)
shadow = shadow.filter(ImageFilter.GaussianBlur(radius=16))

canvas = Image.new("RGBA", (canvas_size, canvas_size), (0, 0, 0, 0))
canvas.alpha_composite(shadow)
canvas.paste(resized, (offset, offset), resized)
canvas.save(dst_path, "PNG")
PY
  else
    sips -z 1024 1024 "$ICON_SRC" --out "$MASTER_PNG" >/dev/null
  fi

  for size in 16 32 128 256 512; do
    double=$((size * 2))
    sips -z "$size" "$size" "$MASTER_PNG" --out "$ICONSET_DIR/icon_${size}x${size}.png" >/dev/null
    sips -z "$double" "$double" "$MASTER_PNG" --out "$ICONSET_DIR/icon_${size}x${size}@2x.png" >/dev/null
  done

  iconutil -c icns "$ICONSET_DIR" -o "$RESOURCES_DIR/AppIcon.icns"
else
  echo "Warning: icon source not found at $ICON_SRC; continuing without AppIcon.icns" >&2
fi

cat > "$CONTENTS_DIR/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleDisplayName</key>
    <string>${APP_NAME}</string>
    <key>CFBundleExecutable</key>
    <string>ace</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>CFBundleIdentifier</key>
    <string>${BUNDLE_ID}</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>${APP_NAME}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>${VERSION}</string>
    <key>CFBundleVersion</key>
    <string>${VERSION}</string>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.developer-tools</string>
    <key>LSMinimumSystemVersion</key>
    <string>12.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSHumanReadableCopyright</key>
    <string>Copyright © Ace Contributors</string>
    <key>NSPrincipalClass</key>
    <string>NSApplication</string>
    <key>NSSupportsAutomaticGraphicsSwitching</key>
    <true/>
</dict>
</plist>
EOF

plutil -lint "$CONTENTS_DIR/Info.plist" >/dev/null

SIGN_IDENTITY="${CODESIGN_IDENTITY:--}"
echo "==> Signing $APP_DIR (identity: $SIGN_IDENTITY)"
codesign --force --deep --sign "$SIGN_IDENTITY" "$APP_DIR"
codesign --verify --deep --strict "$APP_DIR"

if [[ "$BUILD_DMG" == "true" ]]; then
  DMG_PATH="$DIST_DIR/${APP_NAME}-${VERSION}-${ARCH}.dmg"
  DMG_STAGE="$TMP_ICON_DIR/dmg-stage"
  mkdir -p "$DMG_STAGE"
  cp -R "$APP_DIR" "$DMG_STAGE/"
  ln -s /Applications "$DMG_STAGE/Applications"
  rm -f "$DMG_PATH"
  echo "==> Creating $DMG_PATH"
  hdiutil create -volname "$APP_NAME" -srcfolder "$DMG_STAGE" -ov -format UDZO "$DMG_PATH" >/dev/null
  echo "    DMG: $DMG_PATH"
fi

if [[ "$INSTALL_TO_APPLICATIONS" == "true" ]]; then
  echo "==> Installing to /Applications/${APP_NAME}.app"
  rm -rf "/Applications/${APP_NAME}.app"
  cp -R "$APP_DIR" "/Applications/${APP_NAME}.app"
fi

APP_SIZE="$(du -sh "$APP_DIR" | awk '{print $1}')"
echo "==> Done! Built $APP_DIR ($APP_SIZE)"
