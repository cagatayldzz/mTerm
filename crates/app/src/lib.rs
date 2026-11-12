//! The mTerm application: startup, windows, menus, key bindings and saved state.
//!
//! `apps/mterm` only calls [`run`].

mod diagnostics;
mod keys;
mod menus;
mod persistence;
mod windows;

use crate::{
  diagnostics::{announce_previous_crash, install_panic_hook, log_startup, register_actions},
  keys::bind_keys,
  persistence::save_state,
  windows::{open_folders, open_window, refresh_window_menu, save_window_size, select_window, OpenWindows},
};
use futures::StreamExt as _;
use mterm_storage::{directory_from_url, SavedState};
use mterm_ui::{Assets, Hide, HideOthers, NewWindow, Quit, SelectWindow, ShowAll};
use std::time::Duration;

/// How long to wait for a folder to arrive from the system before opening a plain window.
const OPEN_URLS_GRACE: Duration = Duration::from_millis(250);

/// Starts the application and runs it until it quits.
pub fn run() {
  install_panic_hook();
  log_startup();
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
    register_actions(cx);

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
      // The directory is tracked after each Enter; read it fresh so the last `cd` is not lost.
      let workspaces: Vec<_> = cx.global::<OpenWindows>().windows.iter().map(|(_, workspace)| workspace.clone()).collect();
      for workspace in workspaces {
        workspace.update(cx, |workspace, _| workspace.capture_directories());
      }
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

    announce_previous_crash(cx);

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
