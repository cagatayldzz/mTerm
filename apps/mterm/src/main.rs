#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

//! The mTerm application: windows, menus and application-level actions.

use futures::StreamExt as _;
use gpui_kit::{component::TitleBar, *};
use mterm_storage::{directory_from_url, SavedState, SavedWindow, WindowSize};
use mterm_ui::{
  log_failure, Assets, CheckForUpdates, ClearHistory, ClearScreen, ClosePane, Copy, DecreaseFontSize, Find, FindNext, FindPrevious, Hide, HideOthers,
  IncreaseFontSize, Minimize, NewTab, NewWindow, NextPane, NextTab, OpenSettings, Paste, PreviousPane, PreviousTab, Quit, ResetFontSize, SelectAll,
  SelectWindow, ShowAll, SplitHorizontally, SplitVertically, Workspace, Zoom, CONTEXT, TOOLBAR_HEIGHT,
};
use std::{path::PathBuf, time::Duration};

/// The open windows in the order they were opened, for the numbered entries of the Window menu.
#[derive(Default)]
struct OpenWindows {
  windows: Vec<(AnyWindowHandle, Entity<Workspace>)>,
  /// The titles and front state the menus were last built from.
  shown: Vec<(String, bool)>,
  /// What was last written to disk, so only real changes are saved.
  saved: SavedState,
  /// Set once the app is quitting: windows closing then are not the user leaving them.
  quitting: bool,
}

impl Global for OpenWindows {}

/// How far a new window is offset from the one it opens next to.
const CASCADE: f32 = 28.;

/// How long to wait for a folder to arrive from the system before opening a plain window.
const OPEN_URLS_GRACE: Duration = Duration::from_millis(250);

fn main() {
  // Folders handed over by `open -a mTerm <folder>`, Finder, or another app's "open in terminal".
  let (url_sender, mut url_receiver) = futures::channel::mpsc::unbounded::<Vec<String>>();
  let app = gpui_kit::application().with_assets(Assets);
  app.on_open_urls(move |urls| {
    let _ = url_sender.unbounded_send(urls);
  });
  app.run(move |cx| {
    gpui_kit::init(cx);
    mterm_ui::init(cx);
    cx.activate(true);
    cx.set_global(OpenWindows::default());
    bind_keys(cx);

    cx.on_action(|_: &Quit, cx| cx.quit());
    cx.on_action(|_: &Hide, cx| cx.hide());
    cx.on_action(|_: &HideOthers, cx| cx.hide_other_apps());
    cx.on_action(|_: &ShowAll, cx| cx.unhide_other_apps());
    cx.on_action(|_: &NewWindow, cx| open_window(None, None, cx));
    cx.on_action(|action: &SelectWindow, cx| select_window(action.0, cx));
    // Closing the last window quits.
    cx.on_window_closed(|cx, _| {
      if cx.windows().is_empty() {
        cx.quit();
      }
      refresh_window_menu(cx);
      save_state(cx);
    })
    .detach();
    cx.on_app_quit(|cx| {
      save_state(cx);
      cx.global_mut::<OpenWindows>().quitting = true;
      if let Some(window) = cx.active_window() {
        let _ = window.update(cx, |_, window, _| save_window_size(window));
      }
      async {}
    })
    .detach();

    cx.spawn(async move |cx| {
      while let Some(urls) = url_receiver.next().await {
        cx.update(|cx| open_folders(&urls, cx));
      }
    })
    .detach();

    // Continue where the last run left off: the same windows, tabs and splits, each shell in the
    // directory it was in.
    let restored = SavedState::load(&SavedState::default_path());
    cx.global_mut::<OpenWindows>().saved = restored.clone();
    for window in restored.windows {
      open_window(None, Some(window), cx);
    }

    // A folder on the command line opens a window of its own; otherwise give the system a moment
    // to hand one over before opening a plain window.
    if let Some(folder) = std::env::args().nth(1).and_then(|argument| directory_from_url(&argument)) {
      open_window(Some(folder), None, cx);
    } else {
      cx.spawn(async move |cx| {
        cx.background_executor().timer(OPEN_URLS_GRACE).await;
        cx.update(|cx| {
          if cx.windows().is_empty() {
            open_window(None, None, cx);
          }
        });
      })
      .detach();
    }
  });
}

fn open_folders(urls: &[String], cx: &mut App) {
  let folders: Vec<_> = urls.iter().filter_map(|url| directory_from_url(url)).collect();
  if folders.is_empty() {
    open_window(None, None, cx);
  }
  for folder in folders {
    open_window(Some(folder), None, cx);
  }
  cx.activate(true);
}

fn open_window(directory: Option<PathBuf>, restore: Option<SavedWindow>, cx: &mut App) {
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
    app_id: Some(if cfg!(debug_assertions) { "com.cagatayldzz.mterm.dev" } else { "com.cagatayldzz.mterm" }.into()),
    // Center the traffic lights in the toolbar.
    titlebar: Some(TitlebarOptions { traffic_light_position: Some(point(px(18.), TOOLBAR_HEIGHT / 2. - px(6.))), ..TitleBar::title_bar_options() }),
    ..TitleBar::window_options()
  };
  let (handle, workspace) = gpui_kit::open_window(options, cx, |window, cx| {
    window.on_window_should_close(cx, |window, _| {
      save_window_size(window);
      true
    });
    cx.new(|cx| {
      let mut last_size = None;
      cx.observe_window_bounds(window, move |_, window, _| {
        let size = window.window_bounds().get_bounds().size;
        if last_size != Some(size) {
          save_window_size(window);
          last_size = Some(size);
        }
      })
      .detach();
      cx.observe_window_activation(window, |_, _, cx| cx.defer(refresh_window_menu)).detach();
      Workspace::new(directory.clone(), restore.clone(), window, cx)
    })
  })
  .expect("Failed to open the mTerm window");
  cx.global_mut::<OpenWindows>().windows.push((handle, workspace.clone()));
  // The menu follows each window's title, and which window is in front.
  cx.observe(&workspace, |_, cx| {
    refresh_window_menu(cx);
    save_state(cx);
  })
  .detach();
  refresh_window_menu(cx);
}

/// Rebuilds the menu bar with one numbered entry per open window, the front one checked.
fn refresh_window_menu(cx: &mut App) {
  let open = cx.windows();
  cx.global_mut::<OpenWindows>().windows.retain(|(handle, _)| open.contains(handle));
  let windows: Vec<_> = cx.global::<OpenWindows>().windows.clone();
  let entries: Vec<(String, bool)> = windows
    .iter()
    .map(|(handle, workspace)| (workspace.read(cx).title(), handle.update(cx, |_, window, _| window.is_window_active()).unwrap_or(false)))
    .collect();
  if cx.global::<OpenWindows>().shown == entries {
    return;
  }
  // The Dock lists the native title of each window; the number is the shortcut that selects it.
  for (index, ((handle, _), (title, _))) in windows.iter().zip(&entries).enumerate() {
    let title = if index < 9 { format!("{title} — ⌥⌘{}", index + 1) } else { title.clone() };
    let _ = handle.update(cx, |_, window, _| window.set_window_title(&title));
  }
  set_menus(cx, &entries);
  cx.global_mut::<OpenWindows>().shown = entries;
}

/// Writes what is open to disk when it differs from what was last written. Closing the last
/// window keeps the previous state, so the next launch still continues from it.
fn save_state(cx: &mut App) {
  let open = cx.global::<OpenWindows>();
  if open.quitting || open.windows.is_empty() {
    return;
  }
  let windows: Vec<_> = open.windows.clone();
  let state = SavedState { windows: windows.iter().map(|(_, workspace)| workspace.read(cx).saved_window()).collect() };
  if cx.global::<OpenWindows>().saved == state {
    return;
  }
  log_failure("save the window state", state.save(&SavedState::default_path()));
  cx.global_mut::<OpenWindows>().saved = state;
}

fn select_window(index: usize, cx: &mut App) {
  let handle = cx.global::<OpenWindows>().windows.get(index).map(|(handle, _)| *handle);
  if let Some(handle) = handle {
    let _ = handle.update(cx, |_, window, _| window.activate_window());
  }
}

fn save_window_size(window: &Window) {
  let size = window.window_bounds().get_bounds().size;
  let saved_size = WindowSize { width: size.width.into(), height: size.height.into() };
  log_failure("save the window size", saved_size.save(&WindowSize::default_path()));
}

fn bind_keys(cx: &mut App) {
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
  cx.set_dock_menu(vec![
    MenuItem::action("New Window", NewWindow),
    MenuItem::action("New Tab", NewTab),
    MenuItem::separator(),
    MenuItem::action("Settings…", OpenSettings),
  ]);
}

/// `windows` holds the title and front state of each open window, in opening order.
fn set_menus(cx: &mut App, windows: &[(String, bool)]) {
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
  ]);
}
