//! The menu bar and the Dock menu.

use gpui_kit::*;
use mterm_ui::{
  CheckForUpdates, ClearHistory, ClearScreen, ClosePane, Copy, DecreaseFontSize, Find, FindNext, FindPrevious, Hide, HideOthers, IncreaseFontSize,
  Minimize, NewTab, NewWindow, NextPane, NextTab, OpenSettings, Paste, PreviousPane, PreviousTab, Quit, ReportIssue, ResetFontSize, SelectAll,
  SelectWindow, ShowAll, ShowLogs, SplitHorizontally, SplitVertically, Zoom,
};

/// `windows` holds the title and front state of each open window, in opening order.
pub(crate) fn set_menus(cx: &mut App, windows: &[(String, bool)]) {
  let mut window_items = vec![MenuItem::action("Minimize", Minimize), MenuItem::action("Zoom", Zoom)];
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
      MenuItem::action("Settings…", OpenSettings),
      MenuItem::action("Check for Updates…", CheckForUpdates),
      MenuItem::separator(),
      MenuItem::action("Hide mTerm", Hide),
      MenuItem::action("Hide Others", HideOthers),
      MenuItem::action("Show All", ShowAll),
      MenuItem::separator(),
      MenuItem::action("Quit mTerm", Quit),
    ]),
    Menu::new("Shell").items([
      MenuItem::action("New Window", NewWindow),
      MenuItem::action("New Tab", NewTab),
      MenuItem::separator(),
      MenuItem::action("Split Pane Vertically", SplitVertically),
      MenuItem::action("Split Pane Horizontally", SplitHorizontally),
      MenuItem::separator(),
      MenuItem::separator(),
      MenuItem::action("Select Next Pane", NextPane),
      MenuItem::action("Select Previous Pane", PreviousPane),
      MenuItem::separator(),
      MenuItem::action("Close", ClosePane),
    ]),
    Menu::new("Edit").items([
      MenuItem::action("Copy", Copy),
      MenuItem::action("Paste", Paste),
      MenuItem::action("Select All", SelectAll),
      MenuItem::separator(),
      MenuItem::action("Find…", Find),
      MenuItem::action("Find Next", FindNext),
      MenuItem::action("Find Previous", FindPrevious),
      MenuItem::separator(),
      MenuItem::action("Clear Buffer", ClearScreen),
    ]),
    Menu::new("View").items([
      MenuItem::action("Make Text Bigger", IncreaseFontSize),
      MenuItem::action("Make Text Smaller", DecreaseFontSize),
      MenuItem::action("Actual Size", ResetFontSize),
      MenuItem::separator(),
      MenuItem::action("Clear Command History", ClearHistory),
      MenuItem::separator(),
      MenuItem::action("Show Next Tab", NextTab),
      MenuItem::action("Show Previous Tab", PreviousTab),
    ]),
    Menu::new("Window").items(window_items),
    Menu::new("Help").items([MenuItem::action("Report an Issue…", ReportIssue), MenuItem::action("Show Logs in Finder", ShowLogs)]),
  ]);
}

pub(crate) fn set_dock_menu(cx: &mut App) {
  cx.set_dock_menu(vec![
    MenuItem::action("New Window", NewWindow),
    MenuItem::action("New Tab", NewTab),
    MenuItem::separator(),
    MenuItem::action("Settings…", OpenSettings),
  ]);
}
