# Usage

mTerm opens a terminal with tabs and split panes. Right-click a pane for window,
tab, and split actions. The history sidebar lists commands you have run; click an
entry to type it at the prompt.

## Shortcuts

| Shortcut | Action |
| --- | --- |
| `⌘N` / `⌘T` | New window / new tab |
| `⌘D` / `⇧⌘D` | Split the pane vertically / horizontally |
| `⌘W` | Close the pane (its tab and window with the last one) |
| `⇧⌘[` / `⇧⌘]` | Previous / next tab |
| `⌘B` | Show or hide the history sidebar |
| `⌘,` | Open Settings (`Esc` closes it) |
| `⌘V` | Paste |
| `⌘K` | Clear the screen |
| `⌥←` / `⌥→` | Move by word |
| `⌘←` / `⌘→` | Move to the start / end of the line |

## Opening from other apps

mTerm opens a window in any folder it is handed:

```sh
open -a mTerm ~/Projects/app
# Or: mTerm.app/Contents/MacOS/mTerm ~/Projects/app
```

Copy `dist/mTerm.app` to `/Applications` first. In GitHub Desktop, choose
Settings → Integrations → Shell → **Configure Custom Shell…** and set:

- Path: `/usr/bin/open`
- Arguments: `-a mTerm %TARGET_PATH%`

Apps with their own terminal list may need a custom command like the one above.
Finder's **Open With** also lists mTerm for folders.

## Updates

Settings → Updates checks the latest GitHub release on launch unless automatic
checks are turned off. Choose **Download & Install** to download an update; mTerm
verifies its checksum and code signature before **Restart to Update** installs it.
Updates are available in release builds, not development builds.
