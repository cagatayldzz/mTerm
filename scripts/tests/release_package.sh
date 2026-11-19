#!/bin/bash
# Version stamping and asset verification, on a temporary copy of the real build inputs.
set -euo pipefail
cd "$(dirname "$0")/../.."
source scripts/tests/lib.sh
source scripts/release.sh

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

mkdir -p "$work/apps/mterm/macos"
cp Cargo.toml Cargo.lock package.json "$work/"
cp apps/mterm/macos/Info.plist "$work/apps/mterm/macos/"
stamp_version "$work" 7.8.9 42

assert_contains 'version = "7.8.9"' "$(awk '/^\[workspace\.package\]/{f=1} f' "$work/Cargo.toml" | head -n 3)" "workspace version"
assert_contains '"version": "7.8.9"' "$(cat "$work/package.json")" "package.json version"
assert_contains '<key>CFBundleShortVersionString</key><string>7.8.9</string>' "$(cat "$work/apps/mterm/macos/Info.plist")" "plist version"
assert_contains '<key>CFBundleVersion</key><string>42</string>' "$(cat "$work/apps/mterm/macos/Info.plist")" "plist build number"
names="$(grep -c '^name = "mterm' "$work/Cargo.lock")"
stamped="$(grep -A1 '^name = "mterm' "$work/Cargo.lock" | grep -c '^version = "7.8.9"')"
assert_eq "$names" "$stamped" "every mTerm package in the lockfile is restamped"
assert_eq "0" "$(diff Cargo.lock "$work/Cargo.lock" | grep '^[<>]' | grep -vc '^[<>] version = ' || true)" "only version lines changed in the lockfile"

assert_fails "rejects a bad version" stamp_version "$work" 1.0 1
assert_fails "rejects a bad build number" stamp_version "$work" 1.0.0 x

assert_eq "mTerm-v1.2.3-macos-arm64.zip" "$(artifact_names v1.2.3)" "asset names"
assert_fails "rejects tags without v" artifact_names 1.2.3
assert_fails "rejects an empty tag" artifact_names ""

dist="$work/dist"
mkdir "$dist"
assert_fails "missing assets" finalize_assets "$dist" v1.2.3
echo payload >"$dist/mTerm-v1.2.3-macos-arm64.zip"
finalize_assets "$dist" v1.2.3
expected="$(sha256_of "$dist/mTerm-v1.2.3-macos-arm64.zip")  mTerm-v1.2.3-macos-arm64.zip"
assert_eq "$expected" "$(cat "$dist/SHA256SUMS.txt")" "checksum file"
echo extra >"$dist/mTerm-v1.2.3-linux.tar.gz"
assert_fails "unexpected assets" finalize_assets "$dist" v1.2.3
rm "$dist/mTerm-v1.2.3-linux.tar.gz"
: >"$dist/mTerm-v1.2.3-macos-arm64.zip"
assert_fails "empty assets" finalize_assets "$dist" v1.2.3

finish
