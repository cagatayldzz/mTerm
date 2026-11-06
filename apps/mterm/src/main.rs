#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

//! The mTerm application: window, menus and application-level actions.

use gpui_kit::{component::TitleBar, *};
use mterm_storage::WindowSize;
use mterm_ui::{Assets, ClearHistory, ClearScreen, Hide, HideOthers, Minimize, Paste, Quit, ShowAll, ToggleSidebar, Workspace, Zoom, TOOLBAR_HEIGHT};

fn main() {
  gpui_kit::application().with_assets(Assets).run(|cx| {
    gpui_kit::init(cx);
    cx.activate(true);
    bind_keys(cx);

    cx.on_action(|_: &Quit, cx| cx.quit());
    cx.on_action(|_: &Hide, cx| cx.hide());
    cx.on_action(|_: &HideOthers, cx| cx.hide_other_apps());
    cx.on_action(|_: &ShowAll, cx| cx.unhide_other_apps());
    // mTerm has a single window, so closing it quits.
    cx.on_window_closed(|cx, _| {
      if cx.windows().is_empty() {
        cx.quit();
      }
    })
    .detach();

    let saved_size = WindowSize::load(&WindowSize::default_path());
    let bounds = Bounds::centered(None, size(px(saved_size.width), px(saved_size.height)), cx);
    let options = WindowOptions {
      window_bounds: Some(WindowBounds::Windowed(bounds)),
      window_min_size: Some(size(px(WindowSize::MIN.width), px(WindowSize::MIN.height))),
      app_id: Some("com.cagatayldzz.mterm".into()),
      // Center the traffic lights in the toolbar.
      titlebar: Some(TitlebarOptions { traffic_light_position: Some(point(px(18.), TOOLBAR_HEIGHT / 2. - px(6.))), ..TitleBar::title_bar_options() }),
      ..TitleBar::window_options()
    };
    let window = gpui_kit::open_window(options, cx, |window, cx| {
      window.on_window_should_close(cx, |window, _| {
        save_window_size(window);
        true
      });
      cx.new(|cx| Workspace::new(window, cx))
    })
    .expect("Failed to open the mTerm window");
    cx.on_app_quit(move |cx| {
      let _ = window.0.update(cx, |_, window, _| save_window_size(window));
      async {}
    })
    .detach();
  });
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
    Menu::new("Edit").items([MenuItem::action("Paste", Paste), MenuItem::separator(), MenuItem::action("Clear Screen", ClearScreen)]),
    Menu::new("View").items([MenuItem::action("Toggle History", ToggleSidebar), MenuItem::action("Clear History", ClearHistory)]),
    Menu::new("Window").items([MenuItem::action("Minimize", Minimize), MenuItem::action("Zoom", Zoom)]),
  ]);
}
