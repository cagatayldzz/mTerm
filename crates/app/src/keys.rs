//! Key bindings and the application-level window actions.

use crate::menus::{set_dock_menu, set_menus};
use gpui_kit::*;
use mterm_ui::{
  ClearScreen, ClosePane, Copy, DecreaseFontSize, Find, FindNext, FindPrevious, Hide, HideOthers, IncreaseFontSize, Minimize, NewTab, NewWindow,
  NextPane, NextTab, OpenSettings, Paste, PreviousPane, PreviousTab, Quit, ResetFontSize, SelectAll, SelectWindow, SplitHorizontally,
  SplitVertically, Zoom, CONTEXT,
};

pub(crate) fn bind_keys(cx: &mut App) {
  cx.bind_keys(
    ["alt-cmd-1", "alt-cmd-2", "alt-cmd-3", "alt-cmd-4", "alt-cmd-5", "alt-cmd-6", "alt-cmd-7", "alt-cmd-8", "alt-cmd-9"]
      .into_iter()
      .enumerate()
      .map(|(index, keys)| KeyBinding::new(keys, SelectWindow(index), None)),
  );
  cx.bind_keys([
    KeyBinding::new("cmd-q", Quit, None),
    KeyBinding::new("cmd-h", Hide, None),
    KeyBinding::new("alt-cmd-h", HideOthers, None),
    KeyBinding::new("cmd-m", Minimize, None),
    KeyBinding::new("cmd-n", NewWindow, None),
    KeyBinding::new("cmd-,", OpenSettings, None),
    KeyBinding::new("cmd-t", NewTab, Some(CONTEXT)),
    KeyBinding::new("cmd-d", SplitVertically, Some(CONTEXT)),
    KeyBinding::new("shift-cmd-d", SplitHorizontally, Some(CONTEXT)),
    KeyBinding::new("cmd-w", ClosePane, Some(CONTEXT)),
    KeyBinding::new("shift-cmd-]", NextTab, Some(CONTEXT)),
    KeyBinding::new("shift-cmd-[", PreviousTab, Some(CONTEXT)),
    KeyBinding::new("cmd-c", Copy, Some(CONTEXT)),
    KeyBinding::new("cmd-v", Paste, Some(CONTEXT)),
    KeyBinding::new("cmd-a", SelectAll, Some(CONTEXT)),
    KeyBinding::new("cmd-f", Find, Some(CONTEXT)),
    KeyBinding::new("cmd-g", FindNext, Some(CONTEXT)),
    KeyBinding::new("shift-cmd-g", FindPrevious, Some(CONTEXT)),
    KeyBinding::new("cmd-k", ClearScreen, Some(CONTEXT)),
    KeyBinding::new("cmd-]", NextPane, Some(CONTEXT)),
    KeyBinding::new("cmd-[", PreviousPane, Some(CONTEXT)),
    KeyBinding::new("cmd-=", IncreaseFontSize, Some(CONTEXT)),
    KeyBinding::new("cmd-+", IncreaseFontSize, Some(CONTEXT)),
    KeyBinding::new("cmd--", DecreaseFontSize, Some(CONTEXT)),
    KeyBinding::new("cmd-0", ResetFontSize, Some(CONTEXT)),
  ]);
  cx.on_action(|_: &Minimize, cx| {
    if let Some(window) = cx.active_window() {
      let _ = window.update(cx, |_, window, _| window.minimize_window());
    }
  });
  cx.on_action(|_: &Zoom, cx| {
    if let Some(window) = cx.active_window() {
      let _ = window.update(cx, |_, window, _| window.zoom_window());
    }
  });
  set_menus(cx, &[]);
  set_dock_menu(cx);
}
