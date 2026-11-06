#!/bin/bash
# Usage:
#   scripts/app.sh --dev                     debug build, then launch dist/mTerm.app
#   scripts/app.sh [debug|release] --build   build dist/mTerm.app (default: debug)
#   scripts/app.sh --check                   formatting, clippy and tests
#   scripts/app.sh --icon                    regenerate AppIcon.icns from icon.svg
set -euo pipefail

# Project paths and helpers.
project_dir="$(cd "$(dirname "$0")/.." && pwd)"
cd "$project_dir"

usage="Usage: scripts/app.sh --dev | [debug|release] --build | --check | --icon"
resources="$project_dir/apps/mterm/macos"
app_dir="$project_dir/dist/mTerm.app"

fail() {
  echo "$1" >&2
  exit 1
}

# Parse arguments and verify the toolchain.
configuration="debug"
action=""

for argument in "$@"; do
  case "$argument" in
    debug | release) configuration="$argument" ;;
    --dev) action="dev"; configuration="debug" ;;
    --build) action="build" ;;
    --check) action="check" ;;
    --icon) action="icon" ;;
    *) fail "$usage" ;;
  esac
done

[[ -n "$action" ]] || fail "$usage"
command -v cargo >/dev/null 2>&1 || fail "Rust was not found. Install the Rust toolchain and try again."

# Formatting, clippy and tests.
run_check() {
  cargo fmt --all -- --check
  cargo clippy --locked --workspace --all-targets -- -D warnings
  cargo test --locked --workspace
}

# Render icon.svg at every size macOS expects and pack it into AppIcon.icns.
make_icon() {
  local work iconset
  work="$(mktemp -d)"
  iconset="$work/AppIcon.iconset"
  mkdir "$iconset"
  # Headless Chrome keeps the transparent corners (Quick Look flattens them to white).
  local chrome="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
  [[ -x "$chrome" ]] || fail "Google Chrome is required to render icon.svg."
  "$chrome" --headless=new --disable-gpu --hide-scrollbars --default-background-color=00000000 \
    --window-size=1024,1024 --screenshot="$work/icon.svg.png" "file://$resources/icon.svg" >/dev/null 2>&1
  for size in 16 32 128 256 512; do
    sips -z "$size" "$size" "$work/icon.svg.png" --out "$iconset/icon_${size}x${size}.png" >/dev/null
    sips -z "$((size * 2))" "$((size * 2))" "$work/icon.svg.png" --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
  done
  iconutil -c icns "$iconset" -o "$resources/AppIcon.icns"
  rm -rf "$work"
  echo "Ready: $resources/AppIcon.icns"
}

# Build the binary, wrap it in dist/mTerm.app and sign it.
compile() {
  local profile_flags=()
  [[ "$configuration" == release ]] && profile_flags=(--release)

  # Link for the oldest supported macOS rather than the build machine.
  export MACOSX_DEPLOYMENT_TARGET="$(/usr/libexec/PlistBuddy -c 'Print :LSMinimumSystemVersion' "$resources/Info.plist")"
  cargo build --locked ${profile_flags[@]+"${profile_flags[@]}"} -p mterm
}

bundle() {
  rm -rf "$app_dir"
  mkdir -p "$app_dir/Contents/MacOS" "$app_dir/Contents/Resources"
  cp "$project_dir/target/$configuration/mTerm" "$app_dir/Contents/MacOS/mTerm"
  cp "$resources/Info.plist" "$app_dir/Contents/Info.plist"
  cp "$resources/AppIcon.icns" "$app_dir/Contents/Resources/AppIcon.icns"
}

# Local development signature. Distribution requires Developer ID + notarization.
sign() {
  local output
  output="$(codesign --force --sign - "$app_dir" 2>&1)" || fail "$output"
}

build_app() {
  compile
  bundle
  sign
  echo "Ready: $app_dir"
}

# Run the requested action.
case "$action" in
  check) run_check ;;
  icon) make_icon ;;
  build) build_app ;;
  dev)
    build_app
    open -n "$app_dir"
    ;;
esac
