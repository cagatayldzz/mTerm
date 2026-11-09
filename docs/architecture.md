# Architecture

mTerm is a Cargo workspace of small crates with a one-way dependency direction.

## Layers

```text
apps/mterm ──► ui ──► terminal
     │          ├────► storage
     │          └────► updater ──► version
     └───────────────► storage
```

| Crate | Owns | Depends on |
| --- | --- | --- |
| `terminal` | `Session` (PTY + emulator and shell directory lookup), `Snapshot`, `Palette`, `encode_key`, `encode_mouse`, `paste_bytes` | `alacritty_terminal` |
| `storage` | data directory, `WindowSize`, `SavedState`, `History`, `Preferences` | `serde_json` |
| `version` | numeric version comparison | none |
| `updater` | release lookup, download, verification, bundle swap | version |
| `ui` | `AppModel` (shared preferences and update state), `Workspace` (top band, tabs and panes; shared pieces such as the pane menu and close button live in `components.rs`), the Settings panel, actions, assets | terminal, storage, updater, gpui-kit |
| `apps/mterm` | window, menus, key bindings, quit handling | ui, storage, gpui-kit |

`terminal` and `storage` know nothing about the interface, so they are tested
without a window.

## Data flow

1. `Session::spawn` opens a PTY and starts the user's login shell. A background
   thread reads the PTY and feeds `alacritty_terminal`, which keeps the screen state.
2. The thread reports `SessionEvent`s (`Wakeup`, `Title`, `Exit`) through a callback.
   The `ui` crate forwards them over a channel to a GPUI task, which updates the
   `Workspace` and calls `cx.notify()`. Bursts of output are handled in one update.
3. On every render, `Workspace` fits the PTY to the window (`Session::resize`) and
   takes a `Snapshot`: the visible lines as styled spans with resolved RGB colors.
   A block cursor is drawn as an inverted cell inside the snapshot; beam and underline
   cursors are reported in `Snapshot::cursor` and drawn by the interface. `Session::modes`
   exposes the mouse, alternate-screen and focus modes so the interface can report mouse
   events to programs that ask for them.
4. Key presses go through `encode_key` and are written to the PTY. The same keys
   feed a small line tracker that adds finished commands to `History`.

## Windows, tabs and panes

Each window owns one `Workspace`: a list of tabs, each a binary tree of panes plus
the focused pane. All tabs of a window share one command history (`history.txt`).

mTerm continues where it left off. The app keeps `state.json` in the data directory
up to date with every window's tabs, split layouts, the directory of each pane, the
focused pane, the selected tab and each window's position and size (`SavedState` in `mterm-storage`, built from
`Workspace::saved_window`). It is rewritten whenever that snapshot changes, and is
kept when the last window closes. On launch each saved window is reopened with fresh
shells in the saved directories; processes and terminal output cannot be restored. A
missing directory falls back to the user's home directory, and a window whose saved position is no longer on a connected display is placed like a new window. New tabs and splits start
in the focused pane's directory.

Panes report events over one channel tagged with their id; a shell that exits
closes its pane, the last pane closes the tab, and the last tab closes the window.
Split dividers can be dragged to resize panes. Split layouts and ratios are part of
the saved window state. The shell's directory is checked asynchronously after Enter
using the shell process working directory (`lsof` on macOS, `/proc` on Linux).

## Settings and updates

`AppModel` is one entity shared by every window. It owns the saved `Preferences`
(theme, automatic update checks) and mTerm's own update state. Windows
observe it, so a theme change or a new release reaches all of them. Settings (`⌘,`, or
the gear in the top bar) open a panel over the terminal to edit them; `Esc` closes it.

For updates, the updater asks the GitHub releases API for the latest release,
downloads `mTerm-<tag>-macos-arm64.zip` only when asked, and verifies it against
`SHA256SUMS.txt`, the code signature, Gatekeeper and the signing team before a helper
script swaps the `.app` bundle and reopens it. A failed swap restores the old app.
Development builds never check for or install releases.

## Conventions

- English for every identifier, comment, string, and document.
- `unsafe_code` is denied workspace-wide.
- Tests live in each package's `tests/` directory, never inline in `src/`.
- The interface is built with GPUI Kit (`gpui-kit`) and its default icons.

## Command history

The command history is built from typed keys, not from the shell,
so it works without changing shell configuration. A line is recorded on Enter
unless it was edited in a way that cannot be followed (history recall, cursor
movement, Tab completion). Up to 500 commands are kept. This
history is stored without being displayed in the interface. It does not replace or isolate the shell's own history file or shell history
navigation.
