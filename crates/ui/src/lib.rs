//! The mTerm interface: a history sidebar next to tabs of split terminal panes, built with GPUI Kit.

mod assets;
mod history;
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
    ClearScreen,
    ClearHistory,
    NewWindow,
    NewTab,
    SplitVertically,
    SplitHorizontally,
    ClosePane,
    NextTab,
    PreviousTab,
    OpenSettings,
    CheckForUpdates
  ]
);

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
