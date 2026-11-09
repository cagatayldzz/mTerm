//! Writing what is open to disk, so the next launch can continue from it.

use crate::windows::OpenWindows;
use gpui_kit::*;
use mterm_storage::{SavedBounds, SavedState};
use mterm_ui::log_failure;

/// Writes what is open to disk when it differs from what was last written. Closing the last
/// window keeps the previous state, so the next launch still continues from it.
pub(crate) fn save_state(cx: &mut App) {
  let open = cx.global::<OpenWindows>();
  if open.quitting || open.windows.is_empty() {
    return;
  }
  let windows: Vec<_> = open.windows.clone();
  let state = SavedState {
    windows: windows
      .iter()
      .enumerate()
      .map(|(index, (handle, workspace))| {
        let mut saved = workspace.read(cx).saved_window();
        // A window that is being updated right now cannot be asked; keep what was saved for it.
        saved.bounds = handle
          .update(cx, |_, window, _| window.window_bounds().get_bounds())
          .ok()
          .map(|bounds| SavedBounds {
            x: bounds.origin.x.into(),
            y: bounds.origin.y.into(),
            width: bounds.size.width.into(),
            height: bounds.size.height.into(),
          })
          .or_else(|| cx.global::<OpenWindows>().saved.windows.get(index).and_then(|window| window.bounds));
        saved
      })
      .collect(),
  };
  if cx.global::<OpenWindows>().saved == state {
    return;
  }
  log_failure("save the window state", state.save(&SavedState::default_path()));
  cx.global_mut::<OpenWindows>().saved = state;
}
