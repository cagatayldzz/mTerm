//! The menu bar and the Dock menu.

use gpui_kit::*;
use mterm_ui::{
  t, CheckForUpdates, ClearHistory, ClearScreen, ClosePane, Copy, DecreaseFontSize, Find, FindNext, FindPrevious, Hide, HideOthers, IncreaseFontSize,
  Minimize, NewTab, NewWindow, NextPane, NextTab, OpenSettings, Paste, PreviousPane, PreviousTab, Quit, ReportIssue, ResetFontSize, SelectAll,
  SelectWindow, ShowAll, ShowLogs, SplitHorizontally, SplitVertically, Zoom,
};

/// `windows` holds the title and front state of each open window, in opening order.
pub(crate) fn set_menus(cx: &mut App, windows: &[(String, bool)]) {
  let mut window_items = vec![MenuItem::action(t("Minimize"), Minimize), MenuItem::action(t("Zoom"), Zoom)];
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
      MenuItem::action(t("Settings…"), OpenSettings),
      MenuItem::action(t("Check for Updates…"), CheckForUpdates),
      MenuItem::separator(),
      MenuItem::action(t("Hide mTerm"), Hide),
      MenuItem::action(t("Hide Others"), HideOthers),
      MenuItem::action(t("Show All"), ShowAll),
      MenuItem::separator(),
      MenuItem::action(t("Quit mTerm"), Quit),
    ]),
    Menu::new(t("Shell")).items([
      MenuItem::action(t("New Window"), NewWindow),
      MenuItem::action(t("New Tab"), NewTab),
      MenuItem::separator(),
      MenuItem::action(t("Split Pane Vertically"), SplitVertically),
      MenuItem::action(t("Split Pane Horizontally"), SplitHorizontally),
      MenuItem::separator(),
      MenuItem::separator(),
      MenuItem::action(t("Select Next Pane"), NextPane),
      MenuItem::action(t("Select Previous Pane"), PreviousPane),
      MenuItem::separator(),
      MenuItem::action(t("Close"), ClosePane),
    ]),
    Menu::new(t("Edit")).items([
      MenuItem::action(t("Copy"), Copy),
      MenuItem::action(t("Paste"), Paste),
      MenuItem::action(t("Select All"), SelectAll),
      MenuItem::separator(),
      MenuItem::action(t("Find…"), Find),
      MenuItem::action(t("Find Next"), FindNext),
      MenuItem::action(t("Find Previous"), FindPrevious),
      MenuItem::separator(),
      MenuItem::action(t("Clear Buffer"), ClearScreen),
    ]),
    Menu::new(t("View")).items([
      MenuItem::action(t("Make Text Bigger"), IncreaseFontSize),
      MenuItem::action(t("Make Text Smaller"), DecreaseFontSize),
      MenuItem::action(t("Actual Size"), ResetFontSize),
      MenuItem::separator(),
      MenuItem::action(t("Clear Command History"), ClearHistory),
      MenuItem::separator(),
      MenuItem::action(t("Show Next Tab"), NextTab),
      MenuItem::action(t("Show Previous Tab"), PreviousTab),
    ]),
    Menu::new(t("Window")).items(window_items),
    Menu::new(t("Help")).items([MenuItem::action(t("Report an Issue…"), ReportIssue), MenuItem::action(t("Show Logs in Finder"), ShowLogs)]),
  ]);
}

pub(crate) fn set_dock_menu(cx: &mut App) {
  cx.set_dock_menu(vec![
    MenuItem::action(t("New Window"), NewWindow),
    MenuItem::action(t("New Tab"), NewTab),
    MenuItem::separator(),
    MenuItem::action(t("Settings…"), OpenSettings),
  ]);
}
