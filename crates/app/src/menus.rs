//! The menu bar and the Dock menu.

use gpui_kit::*;
use mterm_ui::{
  t, CheckForUpdates, ClearHistory, ClearScreen, ClosePane, Copy, DecreaseFontSize, Find, FindNext, FindPrevious, Hide, HideOthers, IncreaseFontSize,
  Minimize, NewTab, NewWindow, NextPane, NextTab, OpenSettings, Paste, PreviousPane, PreviousTab, Quit, ReportIssue, ResetFontSize, SelectAll,
  SelectWindow, ShowAll, ShowLogs, SplitHorizontally, SplitVertically, Zoom,
};

/// `windows` holds the title and front state of each open window, in opening order.
pub(crate) fn set_menus(cx: &mut App, windows: &[(String, bool)]) {
  let mut window_items = vec![MenuItem::action(t("menu.minimize"), Minimize), MenuItem::action(t("menu.zoom"), Zoom)];
  if !windows.is_empty() {
    window_items.push(MenuItem::separator());
  }
  for (index, (title, front)) in windows.iter().enumerate().take(9) {
    let name = if title.is_empty() { "mTerm" } else { title.as_str() };
    window_items.push(MenuItem::Action {
      name: name.to_string().into(),
      action: Box::new(SelectWindow(index)),
      os_action: None,
      checked: *front,
      disabled: false,
    });
  }
  cx.set_menus([
    Menu::new("mTerm").items([
      MenuItem::action(t("menu.settings"), OpenSettings),
      MenuItem::action(t("menu.check_for_updates"), CheckForUpdates),
      MenuItem::separator(),
      MenuItem::action(t("menu.hide"), Hide),
      MenuItem::action(t("menu.hide_others"), HideOthers),
      MenuItem::action(t("menu.show_all"), ShowAll),
      MenuItem::separator(),
      MenuItem::action(t("menu.quit"), Quit),
    ]),
    Menu::new(t("menu.shell")).items([
      MenuItem::action(t("menu.new_window"), NewWindow),
      MenuItem::action(t("menu.new_tab"), NewTab),
      MenuItem::separator(),
      MenuItem::action(t("menu.split_vertically"), SplitVertically),
      MenuItem::action(t("menu.split_horizontally"), SplitHorizontally),
      MenuItem::separator(),
      MenuItem::separator(),
      MenuItem::action(t("menu.next_pane"), NextPane),
      MenuItem::action(t("menu.previous_pane"), PreviousPane),
      MenuItem::separator(),
      MenuItem::action(t("menu.close"), ClosePane),
    ]),
    Menu::new(t("menu.edit")).items([
      MenuItem::action(t("menu.copy"), Copy),
      MenuItem::action(t("menu.paste"), Paste),
      MenuItem::action(t("menu.select_all"), SelectAll),
      MenuItem::separator(),
      MenuItem::action(t("menu.find"), Find),
      MenuItem::action(t("menu.find_next"), FindNext),
      MenuItem::action(t("menu.find_previous"), FindPrevious),
      MenuItem::separator(),
      MenuItem::action(t("menu.clear_buffer"), ClearScreen),
    ]),
    Menu::new(t("menu.view")).items([
      MenuItem::action(t("menu.text_bigger"), IncreaseFontSize),
      MenuItem::action(t("menu.text_smaller"), DecreaseFontSize),
      MenuItem::action(t("menu.actual_size"), ResetFontSize),
      MenuItem::separator(),
      MenuItem::action(t("menu.clear_history"), ClearHistory),
      MenuItem::separator(),
      MenuItem::action(t("menu.next_tab"), NextTab),
      MenuItem::action(t("menu.previous_tab"), PreviousTab),
    ]),
    Menu::new(t("menu.window")).items(window_items),
    Menu::new(t("menu.help")).items([MenuItem::action(t("menu.report_issue"), ReportIssue), MenuItem::action(t("menu.show_logs"), ShowLogs)]),
  ]);
}

pub(crate) fn set_dock_menu(cx: &mut App) {
  cx.set_dock_menu(vec![
    MenuItem::action(t("menu.new_window"), NewWindow),
    MenuItem::action(t("menu.new_tab"), NewTab),
    MenuItem::separator(),
    MenuItem::action(t("menu.settings"), OpenSettings),
  ]);
}
