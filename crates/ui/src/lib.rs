//! The mTerm interface: windows of tabs and split terminal panes, built with GPUI Kit.

mod assets;
mod components;
mod find;
mod layout;
mod model;
mod selection;
mod settings;
mod tabs;
mod terminal_view;
mod theme;
mod workspace;

use gpui_kit::*;

pub use assets::Assets;
pub use components::log_failure;
pub use model::{AppGlobal, AppModel, AppUpdate, APP_UPDATES_ENABLED, APP_VERSION};
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
    Copy,
    Paste,
    SelectAll,
    Find,
    FindNext,
    FindPrevious,
    ClearScreen,
    IncreaseFontSize,
    DecreaseFontSize,
    ResetFontSize,
    NextPane,
    PreviousPane,
    ClearHistory,
    NewWindow,
    NewTab,
    SplitVertically,
    SplitHorizontally,
    ClosePane,
    NextTab,
    PreviousTab,
    OpenSettings,
    CheckForUpdates,
    ReportIssue,
    ShowLogs
  ]
);

/// Brings the open window at this position (counting from 0) to the front.
#[derive(Clone, PartialEq, Action)]
#[action(namespace = mterm, no_json)]
pub struct SelectWindow(pub usize);

/// Creates the model every window shares and, when enabled, checks for a new release.
pub fn init(cx: &mut App) {
  if let Err(error) = cx.text_system().add_fonts(vec![
    std::borrow::Cow::Borrowed(include_bytes!("../assets/fonts/Geist.ttf")),
    std::borrow::Cow::Borrowed(include_bytes!("../assets/fonts/GeistMono.ttf")),
    std::borrow::Cow::Borrowed(include_bytes!("../assets/fonts/Inter.ttf")),
  ]) {
    eprintln!("Could not load fonts: {error}");
  }
  let model = cx.new(|_| AppModel::new());
  cx.set_global(AppGlobal(model.clone()));
  if APP_UPDATES_ENABLED && model.read(cx).preferences.check_updates {
    model.update(cx, |model, cx| model.check_update(cx));
  }
}
