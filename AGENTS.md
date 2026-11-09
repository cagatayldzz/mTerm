# Project language

Use English for all project-owned identifiers, comments, UI text, logs,
diagnostics, test descriptions and fixtures, documentation, and build scripts.
The conversation can follow the user's preferred language.

# Formatting

Use 2 spaces for indentation in all project-owned files, with LF line endings
and a final newline. Keep EditorConfig and `rustfmt.toml` aligned with this
rule. Run `cargo fmt --all` after Rust changes.

In Rust files, leave one blank line after import groups and between functions,
methods, structs, enums, traits, and implementations.

# Workspace layout

The Cargo workspace has `apps/*` (executables) and `crates/*` (libraries). The
dependency direction is `apps/mterm` → `ui` → `terminal`, `storage` and `updater` (→ `version`).
`terminal` and `storage` never depend on the interface. Update
`docs/architecture.md` when the architecture changes. Keep `unsafe` out of the
workspace (`unsafe_code = "deny"`).

# Interface

Build the interface with GPUI Kit (`gpui-kit`) and its default icons. Use its components
(`Button`, `TabBar`, `Dialog`, `Input`, `Settings`, menus) instead of hand-built
elements, and its theme colors (`link`, `warning`, `selection`, `border`) instead of
fixed color values. Size interface
elements with its tokens (`text_xs`/`text_sm`, `rounded_md`, `size_5`, `gap_2`,
`Sizable`'s `small()`, `Size::Large`) instead of inline pixel values. Pixels stay only
for terminal geometry (cells, padding, scrollbar and divider hit areas), window sizes,
and named constants for sizes a component only accepts in pixels.

# Tests

Keep all Rust test implementations in the owning package's `tests/` directory
with descriptive file names. Do not write inline test modules in production files.

# Checks

Run `bun run check` before finishing a change. Commit or push only when the user asks.
