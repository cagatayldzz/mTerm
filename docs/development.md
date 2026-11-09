# Development

## Platform

Supported and tested: macOS 15+ on Apple Silicon. Intel, Linux and Windows are unsupported. Minimum system version: `LSMinimumSystemVersion` in `apps/mterm/macos/Info.plist` (`scripts/app.sh` links against it). The terminal font is Menlo.

Requirements: Command Line Tools (not Xcode), Rust 1.92+ (stable), Bun. After cloning run `bun install --frozen-lockfile`.

## Commands

| Command | Does |
| --- | --- |
| `bun run dev` | debug build, launch `dist/mTerm.app` |
| `bun run build` | release build → `dist/mTerm.app` |
| `bun run check` | TypeScript, `cargo fmt`, clippy, tests (run before finishing a change) |
| `bun run test` | `cargo test --locked --workspace` |
| `bun run update` | `cargo update` |
| `bun run commit` | `git add -A && oco --yes` |

## Scripts

| Path | Language | Purpose |
| --- | --- | --- |
| `scripts/app.sh` | shell | build, bundle, sign, launch, check (`--dev`, `[debug|release] --build`, `--check`) |
| `scripts/notarize-app.sh` | shell | sign and notarize the app bundle (macOS tools) |
| `scripts/release.ts` | TypeScript | the only script with logic beyond shell: version bump, release notes, packaging, checksums (`prepare`, `notes`, `package`, `finalize`) |
| `scripts/tests/*.test.ts` | TypeScript | `bun test scripts`; type-checked with `bun run typecheck` |

Use shell for macOS tool orchestration and TypeScript for anything that parses data or calls APIs; keep all TypeScript in `release.ts`, not in new scripts.

## Packages

| Package | Kind | Purpose |
| --- | --- | --- |
| `mterm` | app | executable entry point (binary `mTerm`) |
| `mterm-app` | lib | startup, windows, menus, key bindings, saved state |
| `mterm-ui` | lib | views built with GPUI Kit |
| `mterm-terminal` | lib | PTY, emulation, key and mouse encoding |
| `mterm-storage` | lib | window size, history, preferences, window state |
| `mterm-version` | lib | version comparison |
| `mterm-updater` | lib | self-update from GitHub releases |

Code map and data flow: [architecture](architecture.md).

## Tests

Tests live in `crates/<crate>/tests/`, never inline. Terminal session tests run real programs (`/bin/sh -c …`) through a PTY. Tests touching the data directory use temporary directories; `MTERM_DATA_DIR` overrides the location.

## Running the app in isolation

Use a scratch data directory and an empty shell config so your own state and prompt stay out:

```sh
MTERM_DATA_DIR=/tmp/mterm-data ZDOTDIR=/tmp/mterm-zsh ./target/debug/mterm
```

Seed `MTERM_DATA_DIR/state.json` to start with chosen windows (format: `SavedState` in `crates/storage/src/state.rs`).

## Formatting

2 spaces, LF, final newline, `max_width = 150`. `rustfmt.toml` and `.editorconfig` hold the rules; run `cargo fmt --all`.
