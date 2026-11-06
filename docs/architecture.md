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
| `terminal` | `Session` (PTY + emulator), `Snapshot`, `Palette`, `encode_key`, `paste_bytes` | `alacritty_terminal` |
| `storage` | data directory, `WindowSize`, `History`, `Preferences` | none |
| `version` | numeric version comparison | none |
| `updater` | release lookup, download, verification, bundle swap | version |
| `ui` | `AppModel` (shared preferences and update state), `Workspace` (top band, history sidebar, tabs and panes), the Settings page, actions, assets | terminal, storage, updater, gpui-kit |
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
   The cursor is drawn as an inverted cell inside the snapshot.
4. Key presses go through `encode_key` and are written to the PTY. The same keys
   feed a small line tracker that adds finished commands to `History`.

## Windows, tabs and panes

Each window owns one `Workspace`. A workspace holds tabs; a tab is a binary tree
of panes (`layout::Node`) plus the focused pane, and every pane owns a `Session`.
Panes report events over one channel tagged with their id; a shell that exits closes
its pane, the last pane closes the tab, and the last tab closes the window. Each pane
fits its PTY to the space layout gives it (a `canvas` measures it during paint).
New windows are opened by the app (`⌘N` / the context menu's `NewWindow` action).

## Settings and updates

`AppModel` is one entity shared by every window. It owns the saved `Preferences`
(sidebar state, theme, automatic update checks) and mTerm's own update state. Windows
observe it, so a theme change or a new release reaches all of them. Settings (`⌘,`, or
the gear in the sidebar) replace the terminal inside the window and edit it; `Esc` closes them.

Updates follow TendKit: the updater asks the GitHub releases API for the latest release,
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

History is built from typed keys, not from the shell, so it works with any shell
without configuration. A line is recorded on Enter unless it was edited in a way
that cannot be followed (history recall, cursor movement, Tab completion). The
last 500 commands are kept in `~/Library/Application Support/mTerm/history.txt`.
