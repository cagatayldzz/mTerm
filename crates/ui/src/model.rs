//! State shared by every window: the saved preferences and mTerm's own updates.

use futures::{channel::mpsc, StreamExt};
use gpui_kit::*;
use mterm_storage::{Preferences, ThemeChoice};
use mterm_updater::{Release, Stage, Staged};
use std::time::SystemTime;

/// The version of the running application.
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Developer builds never check for or install published releases.
pub const APP_UPDATES_ENABLED: bool = !cfg!(debug_assertions) && cfg!(target_os = "macos");

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AppUpdate {
  Idle,
  Checking,
  UpToDate,
  Available(Release),
  /// Preparing the update; the release is kept so a failure can offer a retry.
  Working(Release, Stage),
  /// Downloaded and verified; restarting installs it.
  Ready(Release),
  Failed(String),
}

impl AppUpdate {
  pub fn is_working(&self) -> bool {
    matches!(self, Self::Checking | Self::Working(..))
  }
}

enum Message<T> {
  Stage(Stage),
  Done(T),
}

pub struct AppModel {
  pub preferences: Preferences,
  pub update: AppUpdate,
  pub update_checked: Option<SystemTime>,
  /// The release a window already announced, so each is announced once.
  pub announced: Option<String>,
  last_release: Option<Release>,
  staged: Option<Staged>,
}

/// The shared model, reachable from any window.
#[derive(Clone)]
pub struct AppGlobal(pub Entity<AppModel>);

impl Global for AppGlobal {}

impl AppModel {
  pub fn new() -> Self {
    Self {
      preferences: Preferences::load(&Preferences::default_path()),
      update: AppUpdate::Idle,
      update_checked: None,
      announced: None,
      last_release: None,
      staged: None,
    }
  }

  pub fn theme(&self) -> ThemeChoice {
    self.preferences.theme
  }

  pub fn set_theme(&mut self, choice: ThemeChoice, cx: &mut Context<Self>) {
    self.preferences.theme = choice;
    self.save(cx);
  }

  pub fn set_check_updates(&mut self, value: bool, cx: &mut Context<Self>) {
    self.preferences.check_updates = value;
    self.save(cx);
  }

  /// Remembers whether the history sidebar was left open.
  pub fn set_sidebar_open(&mut self, value: bool, cx: &mut Context<Self>) {
    self.preferences.sidebar_open = value;
    self.save(cx);
  }

  fn save(&self, cx: &mut Context<Self>) {
    if let Err(error) = self.preferences.save(&Preferences::default_path()) {
      eprintln!("Could not save the preferences: {error}");
    }
    cx.notify();
  }

  /// Looks for a newer mTerm release.
  pub fn check_update(&mut self, cx: &mut Context<Self>) {
    if !APP_UPDATES_ENABLED || self.update.is_working() || matches!(self.update, AppUpdate::Ready(_)) {
      return;
    }
    self.update = AppUpdate::Checking;
    cx.notify();
    Self::run_update_task(
      cx,
      |_| mterm_updater::check(APP_VERSION),
      |_, _, _| {},
      |model, result, _| {
        model.update_checked = Some(SystemTime::now());
        model.update = match result {
          Ok(Some(release)) => AppUpdate::Available(release),
          Ok(None) => AppUpdate::UpToDate,
          Err(error) => AppUpdate::Failed(error.to_string()),
        };
      },
    );
  }

  /// Downloads and verifies the available release.
  pub fn download_update(&mut self, cx: &mut Context<Self>) {
    if !APP_UPDATES_ENABLED {
      return;
    }
    let release = match &self.update {
      AppUpdate::Available(release) => release.clone(),
      AppUpdate::Failed(_) => match &self.last_release {
        Some(release) => release.clone(),
        None => return,
      },
      _ => return,
    };
    let current = match mterm_updater::current_app_bundle() {
      Ok(path) => path,
      Err(error) => {
        self.update = AppUpdate::Failed(error.to_string());
        cx.notify();
        return;
      }
    };
    self.last_release = Some(release.clone());
    self.update = AppUpdate::Working(release.clone(), Stage::Downloading);
    cx.notify();
    let staged_release = release.clone();
    Self::run_update_task(
      cx,
      move |stage| mterm_updater::prepare(&release, &current, stage),
      move |model, stage, _| model.update = AppUpdate::Working(staged_release.clone(), stage),
      |model, result, _| match result {
        Ok(staged) => {
          model.staged = Some(staged);
          model.update = model.last_release.clone().map(AppUpdate::Ready).unwrap_or(AppUpdate::Idle);
        }
        Err(error) => model.update = AppUpdate::Failed(error.to_string()),
      },
    );
  }

  /// Quits and lets the installer swap in the verified update.
  pub fn restart_to_update(&mut self, cx: &mut Context<Self>) {
    if !APP_UPDATES_ENABLED {
      return;
    }
    let Some(staged) = self.staged.take() else { return };
    match mterm_updater::current_app_bundle().and_then(|current| mterm_updater::install(&staged, &current)) {
      Ok(()) => cx.quit(),
      Err(error) => {
        mterm_updater::discard(&staged);
        self.update = AppUpdate::Failed(error.to_string());
        cx.notify();
      }
    }
  }

  /// Runs `work` on a background thread. `on_stage` receives progress reports on
  /// the foreground, then `done` receives the result.
  fn run_update_task<T: Send + 'static>(
    cx: &mut Context<Self>,
    work: impl FnOnce(&dyn Fn(Stage)) -> T + Send + 'static,
    on_stage: impl Fn(&mut Self, Stage, &mut Context<Self>) + 'static,
    done: impl FnOnce(&mut Self, T, &mut Context<Self>) + 'static,
  ) {
    let (sender, mut receiver) = mpsc::unbounded();
    std::thread::spawn(move || {
      let stages = sender.clone();
      let result = work(&|stage| {
        let _ = stages.unbounded_send(Message::Stage(stage));
      });
      let _ = sender.unbounded_send(Message::Done(result));
    });
    cx.spawn(async move |this, cx| {
      let mut done = Some(done);
      while let Some(message) = receiver.next().await {
        let alive = this.update(cx, |model, cx| {
          match message {
            Message::Stage(stage) => on_stage(model, stage, cx),
            Message::Done(result) => {
              if let Some(done) = done.take() {
                done(model, result, cx);
              }
            }
          }
          cx.notify();
        });
        if alive.is_err() {
          break;
        }
      }
    })
    .detach();
  }
}

impl Default for AppModel {
  fn default() -> Self {
    Self::new()
  }
}
