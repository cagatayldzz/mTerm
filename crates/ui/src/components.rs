//! Interface pieces used in more than one place.

use gpui_kit::{
  component::{
    button::{Button, ButtonVariants as _},
    menu::PopupMenu,
    IconName, Sizable as _,
  },
  *,
};
use std::io;

use crate::{i18n::t, ClearScreen, ClosePane, Copy, NewTab, NewWindow, Paste, SplitHorizontally, SplitVertically};

/// A ghost icon button with a tooltip, for the toolbar and the find bar.
pub(crate) fn icon_button(
  id: impl Into<ElementId>,
  icon: IconName,
  tooltip: impl Into<SharedString>,
  on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Button {
  Button::new(id).icon(icon).ghost().tooltip(tooltip).on_click(on_click)
}

/// The small × that closes a tab or a pane.
pub(crate) fn close_button(id: impl Into<ElementId>, on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Button {
  Button::new(id).icon(IconName::Close).ghost().xsmall().on_click(move |event, window, cx| {
    cx.stop_propagation();
    on_click(event, window, cx);
  })
}

/// A row of text that shrinks to the space it is given and ends with an ellipsis.
pub(crate) fn truncated() -> Div {
  div().flex_1().min_w_0().overflow_hidden().whitespace_nowrap().text_ellipsis()
}

/// The window, tab, split and editing actions of a terminal pane, for its context and dropdown menus.
/// `context` is the workspace's focus handle, which the actions are dispatched to.
pub(crate) fn pane_menu(menu: PopupMenu, context: &FocusHandle) -> PopupMenu {
  menu
    .action_context(context.clone())
    .menu(t("menu.new_window"), Box::new(NewWindow))
    .menu(t("menu.new_tab"), Box::new(NewTab))
    .separator()
    .menu(t("menu.split_vertically"), Box::new(SplitVertically))
    .menu(t("menu.split_horizontally"), Box::new(SplitHorizontally))
    .separator()
    .menu(t("menu.copy"), Box::new(Copy))
    .menu(t("menu.paste"), Box::new(Paste))
    .menu(t("menu.clear_buffer"), Box::new(ClearScreen))
    .separator()
    .menu(t("menu.close"), Box::new(ClosePane))
}

/// Reports a failed save on stderr; the app keeps running without what could not be saved.
pub fn log_failure(action: &str, result: io::Result<()>) {
  if let Err(error) = result {
    let message = format!("Could not {action}: {error}");
    eprintln!("{message}");
    let _ = mterm_storage::Diagnostics::default().log(&message);
  }
}
