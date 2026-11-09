# Usage

mTerm opens a terminal with tabs and split panes. Right-click a pane for window,
tab, and split actions. Right-click a tab to open a new tab to its right, rename it, reload it,
duplicate it, pin it (pinned tabs stay at the front), close it, or close the other tabs. The history sidebar lists commands you have run; click an
entry to type it at the prompt.

## Shortcuts

| Shortcut | Action |
| --- | --- |
| `⌘N` / `⌘T` | New window / new tab |
| `⌘D` / `⇧⌘D` | Split the pane vertically / horizontally |
| `⌘W` | Close the pane (its tab and window with the last one) |
| `⇧⌘[` / `⇧⌘]` | Previous / next tab |
| `⌘1`–`⌘9` | Go to tab 1–8 / the last tab |
| `⌘[` / `⌘]` | Previous / next split pane |
| `⌘,` | Open Settings (`Esc` closes it) |
| `⌘C` / `⌘V` / `⌘A` | Copy / paste / select all |
| `⌘F` | Find in the screen and scrollback (`Return`/`⌘G` older match, `⇧Return`/`⇧⌘G` newer, `Esc` closes) |
| `⌘K` | Clear the buffer, including the scrollback |
| `⌘=` / `⌘-` / `⌘0` | Make the text bigger / smaller / actual size |
| `⇧PageUp` / `⇧PageDown` | Scroll the history by a screen |
| `⇧Home` / `⇧End` | Jump to the oldest / latest output |
| `⇧Enter` | Insert a newline in applications that support Shift+Enter |
| `⌥←` / `⌥→` | Move by word |
| `⌘←` / `⌘→` | Move to the start / end of the line |

## Mouse

- Drag to select; double-click selects a word or path, triple-click a line.
  Copying joins lines that only wrapped at the screen edge.
- `⌘`-click opens a link or an existing file path under the pointer.
- Middle-click pastes.
- Programs that follow the mouse (tmux, vim, htop) receive clicks, drags and the
  wheel. Hold `⇧` to select text instead.
- In pagers and editors (`less`, `man`, `vim`) the wheel moves like the arrow keys.
- Programs can copy to the clipboard with the OSC 52 escape sequence, which also
  works over SSH.

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
