#!/bin/bash
# Usage:
#   scripts/app.sh --dev                     debug build, then launch dist/mTerm.app
#   scripts/app.sh [debug|release] --build   build dist/mTerm.app (default: debug)
#   scripts/app.sh --check                   scripts, formatting, clippy and tests
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
  bun run typecheck
  bun test scripts
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
  # Draw the development ribbon over the same rendered icon without a Swift toolchain.
  {
    printf '%s' '<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="1024" viewBox="0 0 1024 1024"><image width="1024" height="1024" href="data:image/png;base64,'
    base64 < "$work/icon.svg.png" | tr -d '\n'
    cat <<'SVG'
"/><defs><clipPath id="icon"><rect x="100" y="100" width="824" height="824" rx="185"/></clipPath></defs>
<g clip-path="url(#icon)"><path d="M100 500V100H500Z" fill="#f26133"/>
<text x="238" y="317" transform="rotate(-45 238 317)" fill="white" font-family="-apple-system, sans-serif" font-size="90" font-weight="bold">DEV</text></g></svg>
SVG
  } > "$work/dev.svg"
  "$chrome" --headless=new --disable-gpu --hide-scrollbars --default-background-color=00000000 \
    --window-size=1024,1024 --screenshot="$work/dev.png" "file://$work/dev.svg" >/dev/null 2>&1
  for size in 16 32 128 256 512; do
    sips -z "$size" "$size" "$work/dev.png" --out "$iconset/icon_${size}x${size}.png" >/dev/null
    sips -z "$((size * 2))" "$((size * 2))" "$work/dev.png" --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
  done
  iconutil -c icns "$iconset" -o "$resources/DevIcon.icns"
  rm -rf "$work"
  echo "Ready: $resources/AppIcon.icns and $resources/DevIcon.icns"
}

# Build the binary, wrap it in dist/mTerm.app and sign it.
compile() {
  local profile_flags=()
  [[ "$configuration" == release ]] && profile_flags=(--release)

  cargo build --locked ${profile_flags[@]+"${profile_flags[@]}"} -p mterm
}

bundle() {
  rm -rf "$app_dir"
  mkdir -p "$app_dir/Contents/MacOS" "$app_dir/Contents/Resources"
  cp "$project_dir/target/$configuration/mTerm" "$app_dir/Contents/MacOS/mTerm"
  cp "$resources/Info.plist" "$app_dir/Contents/Info.plist"
  if [[ "$configuration" == debug ]]; then
    cp "$resources/DevIcon.icns" "$app_dir/Contents/Resources/DevIcon.icns"
    /usr/libexec/PlistBuddy -c 'Set :CFBundleIconFile DevIcon' "$app_dir/Contents/Info.plist"
    /usr/libexec/PlistBuddy -c 'Set :CFBundleIdentifier com.cagatayldzz.mterm.dev' "$app_dir/Contents/Info.plist"
  else
    cp "$resources/AppIcon.icns" "$app_dir/Contents/Resources/AppIcon.icns"
  fi
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
