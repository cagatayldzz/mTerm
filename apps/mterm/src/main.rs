#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

//! The mTerm application: windows, menus and application-level actions.

use gpui_kit::{component::TitleBar, *};
use mterm_storage::WindowSize;
use mterm_ui::{
  Assets, ClearHistory, ClearScreen, ClosePane, Hide, HideOthers, Minimize, NewTab, NewWindow, NextTab, Paste, PreviousTab, Quit, ShowAll,
  SplitHorizontally, SplitVertically, ToggleSidebar, Workspace, Zoom, CONTEXT, TOOLBAR_HEIGHT,
};

/// How far a new window is offset from the one it opens next to.
const CASCADE: f32 = 28.;

fn main() {
  gpui_kit::application().with_assets(Assets).run(|cx| {
    gpui_kit::init(cx);
    cx.activate(true);
    bind_keys(cx);

    cx.on_action(|_: &Quit, cx| cx.quit());
    cx.on_action(|_: &Hide, cx| cx.hide());
    cx.on_action(|_: &HideOthers, cx| cx.hide_other_apps());
    cx.on_action(|_: &ShowAll, cx| cx.unhide_other_apps());
    cx.on_action(|_: &NewWindow, cx| open_window(cx));
    // Closing the last window quits.
    cx.on_window_closed(|cx, _| {
      if cx.windows().is_empty() {
        cx.quit();
      }
    })
    .detach();
    cx.on_app_quit(|cx| {
      if let Some(window) = cx.active_window() {
        let _ = window.update(cx, |_, window, _| save_window_size(window));
      }
      async {}
    })
    .detach();

    open_window(cx);
  });
}

fn open_window(cx: &mut App) {
  let saved_size = WindowSize::load(&WindowSize::default_path());
  let size = size(px(saved_size.width), px(saved_size.height));
  // A new window cascades from the active one; the first is centered.
  let bounds = cx
    .active_window()
    .and_then(|window| window.update(cx, |_, window, _| window.window_bounds().get_bounds()).ok())
    .map(|active| Bounds { origin: active.origin + point(px(CASCADE), px(CASCADE)), size })
    .unwrap_or_else(|| Bounds::centered(None, size, cx));
  let options = WindowOptions {
    window_bounds: Some(WindowBounds::Windowed(bounds)),
    window_min_size: Some(gpui_kit::size(px(WindowSize::MIN.width), px(WindowSize::MIN.height))),
    app_id: Some("com.cagatayldzz.mterm".into()),
    // Center the traffic lights in the toolbar.
    titlebar: Some(TitlebarOptions { traffic_light_position: Some(point(px(18.), TOOLBAR_HEIGHT / 2. - px(6.))), ..TitleBar::title_bar_options() }),
    ..TitleBar::window_options()
  };
  gpui_kit::open_window(options, cx, |window, cx| {
    window.on_window_should_close(cx, |window, _| {
      save_window_size(window);
      true
    });
    cx.new(|cx| Workspace::new(window, cx))
  })
  .expect("Failed to open the mTerm window");
}

fn save_window_size(window: &Window) {
  let size = window.window_bounds().get_bounds().size;
  let saved_size = WindowSize { width: size.width.into(), height: size.height.into() };
  if let Err(error) = saved_size.save(&WindowSize::default_path()) {
    eprintln!("Could not save window size: {error}");
  }
}

fn bind_keys(cx: &mut App) {
  cx.bind_keys([
    KeyBinding::new("cmd-q", Quit, None),
    KeyBinding::new("cmd-h", Hide, None),
    KeyBinding::new("alt-cmd-h", HideOthers, None),
    KeyBinding::new("cmd-m", Minimize, None),
    KeyBinding::new("cmd-n", NewWindow, None),
    KeyBinding::new("cmd-t", NewTab, Some(CONTEXT)),
    KeyBinding::new("cmd-d", SplitVertically, Some(CONTEXT)),
    KeyBinding::new("shift-cmd-d", SplitHorizontally, Some(CONTEXT)),
    KeyBinding::new("cmd-w", ClosePane, Some(CONTEXT)),
    KeyBinding::new("shift-cmd-]", NextTab, Some(CONTEXT)),
    KeyBinding::new("shift-cmd-[", PreviousTab, Some(CONTEXT)),
    KeyBinding::new("cmd-b", ToggleSidebar, Some(CONTEXT)),
    KeyBinding::new("cmd-v", Paste, Some(CONTEXT)),
    KeyBinding::new("cmd-k", ClearScreen, Some(CONTEXT)),
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
  set_menus(cx);
}

fn set_menus(cx: &mut App) {
  cx.set_menus([
    Menu::new("mTerm").items([
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
      MenuItem::action("Close", ClosePane),
    ]),
    Menu::new("Edit").items([MenuItem::action("Paste", Paste), MenuItem::separator(), MenuItem::action("Clear Screen", ClearScreen)]),
    Menu::new("View").items([
      MenuItem::action("Toggle History", ToggleSidebar),
      MenuItem::action("Clear History", ClearHistory),
      MenuItem::separator(),
      MenuItem::action("Show Next Tab", NextTab),
      MenuItem::action("Show Previous Tab", PreviousTab),
    ]),
    Menu::new("Window").items([MenuItem::action("Minimize", Minimize), MenuItem::action("Zoom", Zoom)]),
  ]);
}
