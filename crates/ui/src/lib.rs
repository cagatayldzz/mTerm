//! The mTerm interface: a history sidebar next to tabs of split terminal panes, built with GPUI Kit.

mod history;
mod layout;
mod tabs;
mod terminal_view;
mod workspace;

use gpui_kit::*;

pub use gpui_kit_assets::Assets;
pub use workspace::{Workspace, CONTEXT, TOOLBAR_HEIGHT};

actions!(
  mterm,
  [
    Quit,
    Hide,
    HideOthers,
    ShowAll,
    Minimize,
    Zoom,
    ToggleSidebar,
    Paste,
    ClearScreen,
    ClearHistory,
    NewWindow,
    NewTab,
    SplitVertically,
    SplitHorizontally,
    ClosePane,
    NextTab,
    PreviousTab
  ]
);
