//! The mTerm interface: a history sidebar next to a terminal, built with GPUI Kit.

mod history;
mod terminal_view;
mod workspace;

use gpui_kit::*;

pub use gpui_kit_assets::Assets;
pub use workspace::{Workspace, TOOLBAR_HEIGHT};

actions!(mterm, [Quit, Hide, HideOthers, ShowAll, Minimize, Zoom, ToggleSidebar, Paste, ClearScreen, ClearHistory]);
