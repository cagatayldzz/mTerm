# Language

English for all project-owned identifiers, comments, UI text, logs, tests, docs and scripts. Conversation may use the user's language.

# Formatting

2 spaces, LF, final newline (EditorConfig and `rustfmt.toml` match). Run `cargo fmt --all` after Rust changes. In Rust, one blank line after imports and between items.

# Layout

Workspace: `apps/*` (executables), `crates/*` (libraries). Dependencies: `apps/mterm` → `ui` → `terminal`, `storage`, `updater` (→ `version`). `terminal` and `storage` never depend on the interface. No `unsafe` (`unsafe_code = "deny"`). Update `docs/architecture.md` when architecture changes.

# Interface

Use GPUI Kit (`gpui-kit`) and its default icons. Reference: <https://gpui-kit.com/llms-full.txt> ("Coding Guides"); read the relevant part before interface work. It wins over this file for GPUI Kit usage.

- Kit components over hand-built elements; one shared component per repeated pattern.
- Size tokens (`text_sm`, `rounded_md`, `size_5`, `gap_2`, `small()`, `Size::Large`), not inline `px(...)`. Pixels only for terminal geometry, window size/placement, platform boundaries, and named constants for pixel-only component APIs.
- Colors from `cx.theme()`, not raw `rgb`/`hsla` (terminal palette and `theme.rs` excepted).
- Stable ids from identity, never list indexes.
- Keep tasks and subscriptions on their owner; do not detach recurring ones. Retained UI text is `SharedString`. Never store `&mut Window`/`App`/`Context`; store handles.
- No state changes or focus requests in `render`; use handlers, `defer`, `defer_in`.
- One command = one `Action` shared by buttons, menus and keys. Bind keys before `set_menus`.
- Show actionable failures in the UI, not only logs.

# Tests

Rust tests live in the owning package's `tests/` with descriptive names. No inline test modules in production files.

# Checks

Run `bun run check` before finishing. Commit or push only when the user asks.
