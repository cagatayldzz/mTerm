#!/bin/bash
# The workflows keep the guarantees the release process relies on, and the app bundle matches Cargo's macOS target.
set -euo pipefail
cd "$(dirname "$0")/../.."
source scripts/tests/lib.sh

ci="$(cat .github/workflows/ci.yml)"
release="$(cat .github/workflows/release.yml)"
action="$(cat .github/actions/build-release/action.yml)"

assert_contains "bash scripts/tests/run.sh" "$ci" "CI runs the shell tests"
assert_contains "-D warnings" "$ci" "CI treats warnings as errors"

assert_contains "workflow_dispatch:" "$release" "release is started by hand"
assert_contains "github.ref_name == github.event.repository.default_branch" "$release" "release runs from the default branch only"
assert_contains "cancel-in-progress: false" "$release" "a running release is never cancelled"
assert_contains "needs: validate" "$release" "preparation waits for validation"
assert_contains "needs: [macos_checks, macos_arm64]" "$release" "verification waits for the checks and the build"
assert_contains "needs: verify" "$release" "publish waits for verification"
assert_contains "bash scripts/release.sh notes" "$release" "release notes come from the release script"
assert_contains "bash scripts/release.sh finalize" "$release" "assets are finalized by the release script"
assert_contains "--draft" "$release" "the release is created as a draft first"
assert_contains "cargo test --locked --workspace" "$release" "the workspace is tested while the app builds"
if [[ "$release" == *"sha256sum --check"*"gh release create"* ]]; then :; else
  echo "FAIL: checksums are verified before publishing" >&2
  failures=$((failures + 1))
fi

assert_contains "bash scripts/notarize-app.sh" "$action" "the build notarizes the app"
assert_contains "bash scripts/release.sh package" "$action" "the build packages with the release script"
notarize_line="$(grep -n 'notarize-app.sh' <<<"$action" | head -n 1 | cut -d: -f1)"
package_line="$(grep -n 'release.sh package' <<<"$action" | head -n 1 | cut -d: -f1)"
[[ "$notarize_line" -lt "$package_line" ]] || { echo "FAIL: the app is signed before it is packaged" >&2; failures=$((failures + 1)); }
[[ "$action" != *"secrets."* ]] || { echo "FAIL: the build action must not read secrets" >&2; failures=$((failures + 1)); }
if grep -Eq 'runs-on: (macos|windows)' <<<"$ci"; then
  echo "FAIL: CI runs on Ubuntu only" >&2
  failures=$((failures + 1))
fi

for variable in APPLE_CERTIFICATE_BASE64 APPLE_CERTIFICATE_PASSWORD APPLE_ID APPLE_TEAM_ID APPLE_APP_SPECIFIC_PASSWORD; do
  assert_contains "secrets.$variable" "$release" "release reads $variable"
done

# Only shell and Rust: no other scripting runtimes in automation.
for file in .github/workflows/*.yml .github/actions/*/action.yml scripts/*.sh; do
  if grep -Eq '(^|[^a-z])(bun|npm|node|python3?|ruby)( |$)' "$file"; then
    echo "FAIL: $file uses a runtime other than shell and Rust" >&2
    failures=$((failures + 1))
  fi
done

# Cargo and the app bundle use the same minimum macOS version.
minimum="$(sed -n 's#.*<key>LSMinimumSystemVersion</key><string>\([^<]*\)</string>.*#\1#p' apps/mterm/macos/Info.plist)"
target="$(sed -n 's/.*MACOSX_DEPLOYMENT_TARGET[^"]*"\([^"]*\)".*/\1/p' .cargo/config.toml)"
assert_eq "$minimum" "$target" "the minimum macOS version matches Cargo's deployment target"

finish
