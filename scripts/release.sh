#!/bin/bash
# Usage:
#   scripts/release.sh            request a release: dispatches the Release workflow for the default branch
#   scripts/release.sh prepare    (CI) choose the next version from commits and stamp the build inputs
#   scripts/release.sh notes      (CI) write dist/RELEASE_NOTES.md from commit titles
#   scripts/release.sh package    (CI, macOS) zip dist/mTerm.app as dist/mTerm-<tag>-macos-arm64.zip
#   scripts/release.sh finalize   (CI) check the assets and write dist/SHA256SUMS.txt
#
# The version follows Conventional Commits: a breaking change is major, `feat` is minor, anything else is patch.
# The script can be sourced (the tests do) without running anything.

set -euo pipefail

usage="Usage: scripts/release.sh [prepare|notes|package|finalize]"
release_targets=("macos-arm64.zip")
sections=("New Features" "Bug Fixes" "Documentation" "Chores")

fail() {
  echo "$1" >&2
  exit 1
}

# Pure helpers.

# Read full commit messages separated by NUL bytes on stdin and print major, minor or patch.
release_bump() {
  local message bump="patch"
  while IFS= read -r -d '' message; do
    if printf '%s\n' "$message" | grep -Eiq '^[a-z]+(\([^)]+\))?!:' || printf '%s\n' "$message" | grep -Eq '^BREAKING[ -]CHANGE:[[:space:]]'; then
      echo major
      return
    fi
    if printf '%s\n' "$message" | grep -Eiq '^feat(\([^)]+\))?:'; then
      bump="minor"
    fi
  done
  echo "$bump"
}

# next_version <current> <major|minor|patch>
next_version() {
  local current="$1" bump="$2" major minor patch
  [[ "$current" =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ ]] || fail "Invalid stable version: $current"
  IFS=. read -r major minor patch <<<"$current"
  case "$bump" in
    major) echo "$((major + 1)).0.0" ;;
    minor) echo "$major.$((minor + 1)).0" ;;
    patch) echo "$major.$minor.$((patch + 1))" ;;
    *) fail "Release bump must be patch, minor or major." ;;
  esac
}

# artifact_names <tag>: one asset name per line.
artifact_names() {
  local tag="${1:-}" suffix
  [[ "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]] || fail "Invalid release tag."
  for suffix in "${release_targets[@]}"; do
    echo "mTerm-$tag-$suffix"
  done
}

# note_section <subject>: the release notes section a commit title belongs to.
note_section() {
  case "$1" in
    feat:* | feat\(*) echo "New Features" ;;
    fix:* | fix\(*) echo "Bug Fixes" ;;
    docs:* | docs\(*) echo "Documentation" ;;
    *) echo "Chores" ;;
  esac
}

# render_notes <repository> <tag> <previous-tag or empty>: commits as "<sha>TAB<subject>" lines on stdin.
render_notes() {
  local repository="$1" tag="$2" previous="$3" base commits section sha subject printed
  base="https://github.com/$repository"
  commits="$(cat)"
  [[ -n "$commits" ]] || fail "There are no changes to release."
  for section in "${sections[@]}"; do
    printed=""
    while IFS=$'\t' read -r sha subject; do
      [[ "$(note_section "$subject")" == "$section" ]] || continue
      if [[ -z "$printed" ]]; then
        printf '## %s\n\n' "$section"
        printed=1
      fi
      printf -- '- %s ([%s](%s/commit/%s))\n' "$subject" "${sha:0:7}" "$base" "$sha"
    done <<<"$commits"
    [[ -z "$printed" ]] || echo
  done
  printf '## Changelog\n\n'
  if [[ -n "$previous" ]]; then
    printf '[Full Changelog: %s...%s](%s/compare/%s...%s)\n\n' "$previous" "$tag" "$base" "$previous" "$tag"
  else
    printf '[Full Changelog](%s/commits/%s)\n\n' "$base" "$tag"
  fi
}

sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | cut -d ' ' -f 1
  else
    shasum -a 256 "$1" | cut -d ' ' -f 1
  fi
}

# Replace the first `version = "…"` after a matching header line, or every package whose name matches, in place.
rewrite() {
  local file="$1" tmp
  shift
  tmp="$(mktemp)"
  awk "$@" "$file" >"$tmp"
  cat "$tmp" >"$file"
  rm -f "$tmp"
}

# stamp_version <root> <version> <build-number>: write the version into every build input below <root>.
stamp_version() {
  local root="$1" version="$2" build="$3"
  [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || fail "Invalid version: $version"
  [[ "$build" =~ ^[0-9]+$ ]] || fail "Invalid build number: $build"

  grep -q '^\[workspace\.package\]' "$root/Cargo.toml" || fail "Could not stamp [workspace.package] in Cargo.toml."
  rewrite "$root/Cargo.toml" -v v="$version" '
    /^\[/ { inside = ($0 == "[workspace.package]") }
    inside && !done && /^version = / { print "version = \"" v "\""; done = 1; next }
    { print }'

  grep -Eq '^name = "mterm(-[a-z]+)*"$' "$root/Cargo.lock" || fail "Could not stamp the mTerm packages in Cargo.lock."
  rewrite "$root/Cargo.lock" -v v="$version" '
    /^name = "mterm(-[a-z]+)*"$/ { ours = 1; print; next }
    ours && /^version = / { print "version = \"" v "\""; ours = 0; next }
    { ours = 0; print }'

  grep -q '^  "version": ' "$root/package.json" || fail "Could not stamp package.json."
  sed -E -i.bak "s/^(  \"version\": \")[^\"]+/\1$version/" "$root/package.json"

  local plist="$root/apps/mterm/macos/Info.plist"
  sed -E -i.bak \
    -e "s#(<key>CFBundleShortVersionString</key><string>)[^<]+#\1$version#" \
    -e "s#(<key>CFBundleVersion</key><string>)[^<]+#\1$build#" "$plist"
  rm -f "$root/package.json.bak" "$plist.bak"
}

# finalize_assets <directory> <tag>: check the asset set, then write SHA256SUMS.txt.
finalize_assets() {
  local directory="$1" tag="$2" names name expected found
  names="$(artifact_names "$tag")"
  found="$(cd "$directory" && ls -1 | grep -E '\.(zip|tar\.gz|dmg)$' | sort || true)"
  expected="$(printf '%s\n' "$names" | sort)"
  [[ "$found" == "$expected" ]] || fail "Release assets do not match the required platform matrix."
  : >"$directory/SHA256SUMS.txt"
  while IFS= read -r name; do
    [[ -s "$directory/$name" ]] || fail "Empty release asset: $name"
    printf '%s  %s\n' "$(sha256_of "$directory/$name")" "$name" >>"$directory/SHA256SUMS.txt"
  done <<<"$names"
}

# Commands.

stable_tags() {
  git tag --list 'v*' --sort=-version:refname | grep -E '^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$' || true
}

require_ci() {
  [[ "${GITHUB_ACTIONS:-}" == true ]] || fail "This command runs only in GitHub Actions. Use scripts/release.sh without arguments."
}

request_release() {
  command -v gh >/dev/null 2>&1 || fail "Install GitHub CLI (brew install gh), then run gh auth login."
  [[ -z "$(git status --porcelain)" ]] || fail "Commit and push all changes before releasing."
  local repository branch sha remote_sha
  repository="$(gh repo view --json nameWithOwner --jq .nameWithOwner)"
  branch="$(gh repo view --json defaultBranchRef --jq .defaultBranchRef.name)"
  [[ "$(git branch --show-current)" == "$branch" ]] || fail "Release from the default branch: $branch."
  sha="$(git rev-parse HEAD)"
  remote_sha="$(gh api "repos/$repository/commits/$branch" --jq .sha)"
  [[ "$sha" == "$remote_sha" ]] || fail "Local HEAD must match the pushed default branch. Push or pull before releasing."
  gh workflow run release.yml --repo "$repository" --ref "$branch"
  echo "Release requested from $branch. The version increment is automatic."
  echo "Follow progress: https://github.com/$repository/actions/workflows/release.yml"
}

prepare() {
  require_ci
  local sha previous range current bump version tag count
  sha="$(git rev-parse HEAD)"
  [[ -z "${EXPECTED_SHA:-}" || "$EXPECTED_SHA" == "$sha" ]] || fail "The checkout does not match the workflow source commit. Run the release again."
  previous="$(stable_tags | head -n 1)"
  range="HEAD"
  if [[ -n "$previous" ]]; then
    git merge-base --is-ancestor "$previous" HEAD || fail "The previous release tag is not an ancestor of HEAD."
    range="$previous..HEAD"
  fi
  count="$(git rev-list --count "$range")"
  [[ "$count" -gt 0 ]] || fail "There are no changes since the previous release."
  current="${previous#v}"
  [[ -n "$current" ]] || current="$(sed -n 's/^  "version": "\(.*\)",\{0,1\}$/\1/p' package.json | head -n 1)"
  bump="$(git log --format=%B%x00 "$range" | release_bump)"
  version="$(next_version "$current" "$bump")"
  tag="v$version"
  mkdir -p dist
  printf 'PREVIOUS_TAG=%s\nTAG=%s\n' "$previous" "$tag" >dist/release-context.env
  # Stamp build inputs in the disposable CI checkout. Tags remain the version source of truth.
  stamp_version "$PWD" "$version" "${GITHUB_RUN_NUMBER:?GITHUB_RUN_NUMBER is required to prepare a release.}"
  : "${GITHUB_OUTPUT:?GITHUB_OUTPUT is required to prepare a release.}"
  printf 'version=%s\ntag=%s\nsha=%s\n' "$version" "$tag" "$sha" >>"$GITHUB_OUTPUT"
  echo "Preparing $tag ($bump) from $count commits."
}

notes() {
  require_ci
  [[ -f dist/release-context.env ]] || fail "Run prepare before notes."
  local PREVIOUS_TAG TAG range
  # shellcheck disable=SC1091
  source dist/release-context.env
  range="HEAD"
  [[ -z "$PREVIOUS_TAG" ]] || range="$PREVIOUS_TAG..HEAD"
  git log --reverse --format='%H%x09%s' "$range" | render_notes "${GITHUB_REPOSITORY:?GITHUB_REPOSITORY is required}" "$TAG" "$PREVIOUS_TAG" >dist/RELEASE_NOTES.md
}

package() {
  local tag="${RELEASE_TAG:-}"
  artifact_names "$tag" >/dev/null
  [[ "$(uname -s)/$(uname -m)" == Darwin/arm64 ]] || fail "Unsupported release host: $(uname -s)/$(uname -m)"
  [[ -x dist/mTerm.app/Contents/MacOS/mTerm ]] || fail "Build and notarize the macOS application first."
  ditto -c -k --sequesterRsrc --keepParent dist/mTerm.app "dist/mTerm-$tag-macos-arm64.zip"
}

finalize() {
  finalize_assets dist "${RELEASE_TAG:-}"
}

main() {
  cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
  [[ $# -le 1 ]] || fail "$usage"
  case "${1:-}" in
    "") request_release ;;
    prepare) prepare ;;
    notes) notes ;;
    package) package ;;
    finalize) finalize ;;
    *) fail "$usage" ;;
  esac
}

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
  main "$@"
fi
