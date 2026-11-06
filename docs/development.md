# Development

## Requirements

- Apple Silicon and macOS 15 or later
- Command Line Tools (Xcode is not required)
- Stable Rust and Cargo
- Bun

## Commands

```sh
bun run dev       # debug build, then launch dist/mTerm.app
bun run build     # release build → dist/mTerm.app
bun run check     # formatting, clippy, and tests
bun run test      # tests only
bun run update    # cargo update
bun run commit    # git add -A && oco --yes
```

All build commands call `scripts/app.sh`:

```sh
scripts/app.sh --dev                     # debug build, then launch
scripts/app.sh [debug|release] --build   # build dist/mTerm.app (default: debug)
scripts/app.sh --check                   # cargo fmt --check, clippy -D warnings, tests
```

## Workspace packages

| Package | Kind | Purpose |
| --- | --- | --- |
| `mterm` | app | the macOS application (binary `mTerm`) |
| `mterm-ui` | library | views built with GPUI Kit |
| `mterm-terminal` | library | PTY, emulation, key encoding |
| `mterm-storage` | library | window size, command history and preferences |
| `mterm-version` | library | version comparison for update checks |
| `mterm-updater` | library | self-update from GitHub releases |

## Formatting

2 spaces, LF line endings, a final newline, and `max_width = 150`. Run
`cargo fmt --all` after Rust changes; `rustfmt.toml` and `.editorconfig` hold the rules.

## Tests

Test code lives in `crates/<crate>/tests/`. `mterm-terminal`'s session tests run
real programs (`/bin/sh -c …`) through a PTY. Tests that touch the data directory
use temporary directories; `MTERM_DATA_DIR` overrides the location.

## Local data

`~/Library/Application Support/mTerm/` holds `window-size.txt`, `history.txt`, `preferences.txt` and `layout.txt`.
