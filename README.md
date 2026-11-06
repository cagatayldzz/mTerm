# mTerm

A minimal native macOS terminal written in Rust with a GPU-rendered
[GPUI](https://gpui.rs) interface, built with GPUI Kit.

mTerm opens a window with tabs and split panes: a collapsible sidebar on the left lists the commands
you have run, and the terminal fills the rest. Right-click a pane for New Window, New Tab, and Split Pane. Click a history entry to type it at
the prompt. Terminal emulation is provided by `alacritty_terminal`.

<p>
  <img src="docs/images/terminal.png" alt="mTerm with split terminal panes" width="49%">
  <img src="docs/images/settings.png" alt="mTerm settings dialog" width="49%">
</p>

## Requirements

- Apple Silicon, macOS 15+
- Command Line Tools (Xcode is not required)
- Stable Rust and Cargo
- Bun

## Development

```sh
bun run dev       # build and launch the app
bun run build     # local release build → dist/mTerm.app
bun run check     # formatting, clippy, and tests
```

## Opening from other apps

mTerm opens a window in any folder it is handed, so other apps can use it as their terminal:

```sh
open -a mTerm ~/Projects/app      # or: mTerm.app/Contents/MacOS/mTerm ~/Projects/app
```

Copy `dist/mTerm.app` to `/Applications` first. In GitHub Desktop, choose
Settings → Integrations → Shell → **Configure Custom Shell…** and set:

- Path: `/usr/bin/open`
- Arguments: `-a mTerm %TARGET_PATH%`

Apps with their own terminal list (such as the built-in Terminal / iTerm2 entries)
only show terminals they know about; those need a custom command like the one above.
Finder's **Open With** also lists mTerm for folders.

## Shortcuts

| Shortcut | Action |
| --- | --- |
| `⌘N` / `⌘T` | New window / new tab |
| `⌘D` / `⇧⌘D` | Split the pane vertically / horizontally |
| `⌘W` | Close the pane (its tab and window with the last one) |
| `⇧⌘[` / `⇧⌘]` | Previous / next tab |
| `⌘B` | Show or hide the history sidebar |
| `⌘,` | Settings (in the window; `Esc` closes) |
| `⌘V` | Paste |
| `⌘K` | Clear the screen |
| `⌥←` / `⌥→` | Move by word |
| `⌘←` / `⌘→` | Move to the start / end of the line |

## Layout

- `apps/mterm` — the application: window, menus, and actions
- `crates/ui` — the sidebar and terminal views
- `crates/terminal` — PTY, terminal emulation, and key encoding
- `crates/storage` — window size and command history

## Updates

Settings → Updates checks the latest GitHub release (on launch, unless turned off) and
installs it when you choose **Download & Install**. The download is verified against its
checksum and code signature before **Restart to Update** swaps the app. Release builds
only; development builds never update.

## Documentation

- [docs/development.md](docs/development.md) — commands, workspace packages, tests
- [docs/architecture.md](docs/architecture.md) — crate layers and data flow
- [docs/platform-support.md](docs/platform-support.md) — platform support
- [docs/releasing.md](docs/releasing.md) — building a release
- [AGENTS.md](AGENTS.md) — project conventions
