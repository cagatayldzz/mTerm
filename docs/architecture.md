# Architecture

Cargo workspace, one-way dependencies:

```text
apps/mterm ─► app ─► ui ─► terminal
                │      ├──► storage
                │      └──► updater ─► version
                └─────────► storage
```

`terminal` and `storage` never depend on the interface and are tested without a window.

## Code map

| Path | Owns |
| --- | --- |
| `apps/mterm/src/main.rs` | executable entry point only; calls `mterm_app::run` |
| `crates/app/src/lib.rs` | startup: actions, quit, opening folders from the system, restoring saved windows |
| `crates/app/src/windows.rs` | window registry (`OpenWindows`), opening and placing windows, Window menu, window size |
| `crates/app/src/persistence.rs` | `save_state`: writes `SavedState` when the open windows change |
| `crates/app/src/{keys,menus}.rs` | key bindings; menu bar and Dock menu |
| `crates/ui/src/workspace.rs` | `Workspace` (one per window): tabs, panes, input, history, Settings dialog |
| `crates/ui/src/tabs.rs` | top band, `TabBar`, tab context menu |
| `crates/ui/src/terminal_view.rs` | pane rendering, mouse, selection, scrollbar, split dividers |
| `crates/ui/src/components.rs` | shared pieces: `icon_button`, `close_button`, `truncated`, `pane_menu`, `log_failure` |
| `crates/ui/src/{find,selection,layout,settings,theme,model,assets}.rs` | find bar, selection math, split tree (`Node`), Settings pages, themes, `AppModel`, icons |
| `crates/terminal/src/session.rs` | `Session`: PTY, `alacritty_terminal`, shell directory lookup |
| `crates/terminal/src/{snapshot,palette,keys,mouse,search}.rs` | styled screen snapshot, colors, key and mouse encoding, scrollback search |
| `crates/storage/src/` | `state.rs` (`SavedState`), `history.rs`, `preferences.rs`, `window_size.rs`, `paths.rs`, `dropped_files.rs` |
| `crates/updater/src/`, `crates/version/src/` | release lookup, download, verification, bundle swap; version comparison |

## Terminal data flow

1. `Session::spawn` opens a PTY and the login shell; a thread feeds output to `alacritty_terminal`.
2. The thread reports `SessionEvent` (`Wakeup`, `Title`, `Directory`, `ClipboardStore`, `Exit`) by callback. `Workspace` receives them on one channel tagged with `PaneId`, handles bursts in one update, and calls `cx.notify()`. The receiving task is stored on the workspace (`_events_task`).
3. Each render fits the PTY to the pane (`Session::resize`) and takes a `Snapshot` (styled spans, resolved RGB, cursor). `Session::modes` tells the UI whether a program wants mouse, alternate-screen or focus reports.
4. Keys go through `encode_key` to the PTY; the same keys feed a line tracker that records finished commands in `History`.
5. A shell exit closes its pane; the last pane closes the tab; the last tab closes the window.

## Windows, tabs, panes

- `Workspace.tabs: Vec<Tab>`. `Tab { root: Node, focused: PaneId, pinned, title: Option<SharedString>, id: u64 }`. `Node` is a binary split tree whose leaves are `PaneId`s.
- Identities: `PaneId` and `Tab.id` only grow and never repeat in a window; use them for `ElementId`s, never list indexes.
- Tab title = `Tab.title` if set (Rename), else the focused pane's title.
- Tab actions (new to the right, rename, reload, duplicate, pin, close, close others) are `Workspace` methods called from the tab context menu.
- New tabs and splits start in the focused pane's directory.
- All tabs of a window share one `History` (`history.txt`, up to 500 commands). It is recorded from typed keys, not from the shell, and skipped when a line was edited in ways that cannot be followed (history recall, cursor movement, Tab completion).
- Pane directory comes from `SessionEvent::Directory` (checked after Enter via `lsof` on macOS or `/proc`).

## Persistence

Data directory `~/Library/Application Support/mTerm` (override with `MTERM_DATA_DIR`).

| File | Type | Written |
| --- | --- | --- |
| `state.json` | `SavedState` → `SavedWindow` → `SavedTab` | whenever the snapshot changes |
| `history.txt` | `History` | on each recorded command or clear |
| `preferences.txt` | `Preferences` | on each settings change |
| `window-size.txt` | `WindowSize` | on resize and close; default for new windows |

Restore flow:

- `state.json` holds, per window: tabs (layout string from `Node::encode`, one directory per pane, focused pane index, pinned, title), the selected tab, and window bounds.
- `mterm-app` observes every `Workspace`, builds `SavedState` from `Workspace::saved_window()` (bounds are filled in by `persistence.rs`, because only the window knows them), and writes only when it differs from `OpenWindows.saved`. Bounds changes trigger a deferred save.
- Closing the last window or quitting keeps the previous state (`OpenWindows.quitting`, empty-window guard).
- On launch each saved window reopens with fresh shells in the saved directories; processes and output are not restored. A missing directory falls back to home. Bounds beyond the total display area are ignored and the window is placed like a new one (display origins are unreliable, see `on_a_screen` in `windows.rs`).

## Settings and updates

- `AppModel` is one entity shared by all windows: `Preferences` (theme, font, terminal font size and line height, update checks) and update state. Windows observe it.
- Settings (`⌘,` or the gear) open a GPUI Kit `Dialog` holding the `Settings` component. `Workspace.settings` mirrors it; `render` clears it when the dialog is gone (Esc, close button).
- Updater: GitHub releases API → download `mTerm-<tag>-macos-arm64.zip` only on request → verify `SHA256SUMS.txt`, code signature, Gatekeeper and team → helper script swaps the `.app` and reopens it; a failed swap restores the old app. Development builds never check or install.

## Rules

- Language, formatting, tests, GPUI Kit conventions and checks: `AGENTS.md`.
- A change to windows, state files or crate boundaries must update this file.
