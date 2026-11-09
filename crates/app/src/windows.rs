//! The open windows: the registry the menus and state saving read, and opening, placing and selecting windows.

use crate::{menus::set_menus, persistence::save_state};
use gpui_kit::{component::TitleBar, *};
use mterm_storage::{directory_from_url, SavedState, SavedWindow, WindowSize};
use mterm_ui::{log_failure, Workspace, TOOLBAR_HEIGHT};
use std::path::PathBuf;

/// The open windows in the order they were opened, for the numbered entries of the Window menu.
#[derive(Default)]
pub(crate) struct OpenWindows {
  pub(crate) windows: Vec<(AnyWindowHandle, Entity<Workspace>)>,
  /// The titles and front state the menus were last built from.
  pub(crate) shown: Vec<(String, bool)>,
  /// What was last written to disk, so only real changes are saved.
  pub(crate) saved: SavedState,
  /// Set once the app is quitting: windows closing then are not the user leaving them.
  pub(crate) quitting: bool,
}

impl Global for OpenWindows {}

/// How far a new window is offset from the one it opens next to.
const CASCADE: f32 = 28.;

pub(crate) fn open_folders(urls: &[String], cx: &mut App) {
  let folders: Vec<_> = urls.iter().filter_map(|url| directory_from_url(url)).collect();
  if folders.is_empty() {
    open_window(None, None, cx);
  }
  for folder in folders {
    open_window(Some(folder), None, cx);
  }
  cx.activate(true);
}

pub(crate) fn open_window(directory: Option<PathBuf>, restore: Option<SavedWindow>, cx: &mut App) {
  let saved_size = WindowSize::load(&WindowSize::default_path());
  let size = size(px(saved_size.width), px(saved_size.height));
  // A restored window goes back where it was, when that place is still on a screen. A new window
  // cascades from the active one; the first is centered.
  let bounds = restore
    .as_ref()
    .and_then(|window| window.bounds)
    .map(|saved| Bounds::new(point(px(saved.x), px(saved.y)), gpui_kit::size(px(saved.width), px(saved.height))))
    .filter(|bounds| on_a_screen(bounds, cx))
    .or_else(|| {
      cx.active_window()
        .and_then(|window| window.update(cx, |_, window, _| window.window_bounds().get_bounds()).ok())
        .map(|active| Bounds { origin: active.origin + point(px(CASCADE), px(CASCADE)), size })
    })
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
    window.on_window_should_close(cx, |window, cx| {
      save_window_size(window);
      if cx.global::<OpenWindows>().quitting {
        true
      } else {
        window.minimize_window();
        false
      }
    });
    cx.new(|cx| {
      let mut last_size = None;
      cx.observe_window_bounds(window, move |_, window, cx| {
        let size = window.window_bounds().get_bounds().size;
        if last_size != Some(size) {
          save_window_size(window);
          last_size = Some(size);
        }
        // Moving or resizing changes what the next launch restores. Deferred: this workspace is
        // being updated right now, so it cannot be read here.
        cx.defer(save_state);
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

/// Whether a saved window still has a visible part. Displays report their sizes but not reliably
/// where they sit next to each other, so this only rejects windows far beyond the total area.
fn on_a_screen(bounds: &Bounds<Pixels>, cx: &App) -> bool {
  let displays = cx.displays();
  let width: Pixels = displays.iter().map(|display| display.bounds().size.width).fold(px(0.), |total, width| total + width);
  let height: Pixels = displays.iter().map(|display| display.bounds().size.height).fold(px(0.), |most, height| most.max(height));
  bounds.right() > px(0.) && bounds.bottom() > px(0.) && bounds.origin.x < width && bounds.origin.y < height * 2.
}

/// Rebuilds the menu bar with one numbered entry per open window, the front one checked.
pub(crate) fn refresh_window_menu(cx: &mut App) {
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

pub(crate) fn select_window(index: usize, cx: &mut App) {
  let handle = cx.global::<OpenWindows>().windows.get(index).map(|(handle, _)| *handle);
  if let Some(handle) = handle {
    let _ = handle.update(cx, |_, window, _| window.activate_window());
  }
}

pub(crate) fn save_window_size(window: &Window) {
  let size = window.window_bounds().get_bounds().size;
  let saved_size = WindowSize { width: size.width.into(), height: size.height.into() };
  log_failure("save the window size", saved_size.save(&WindowSize::default_path()));
}
