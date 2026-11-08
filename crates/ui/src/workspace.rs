use crate::{
  layout::{Node, PaneId, Split},
  settings::{render_settings, SettingsPage},
  CheckForUpdates, ClearHistory, ClearScreen, ClosePane, Copy, DecreaseFontSize, Find, FindNext, FindPrevious, IncreaseFontSize, NewTab, NextPane,
  NextTab, OpenSettings, Paste, PreviousPane, PreviousTab, ResetFontSize, SelectAll, SplitHorizontally, SplitVertically,
};
use crate::{
  model::{AppGlobal, AppModel, AppUpdate},
  theme::apply_theme,
};
use futures::StreamExt as _;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
  component::{
    button::Button,
    h_flex,
    input::{Input, InputState},
    v_flex, ActiveTheme, Sizable, WindowExt as _,
  },
  *,
};
use mterm_storage::{History, SavedSession, ThemeChoice};
use mterm_terminal::{encode_key, paste_bytes, Modifiers, Palette, Session, SessionEvent, SessionOptions};
use std::{
  cell::{Cell, RefCell},
  collections::HashMap,
  path::PathBuf,
  rc::Rc,
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
};

/// Height of the top band: the traffic lights and the title or tabs.
pub const TOOLBAR_HEIGHT: Pixels = px(44.);

/// How dark the veil over an inactive window is (0 is clear, 1 is black).
pub(crate) const INACTIVE_VEIL: f32 = 0.35;

/// The key context key bindings use to reach a window's workspace.
pub const CONTEXT: &str = "Workspace";

/// One shell and its screen.
pub(crate) struct Pane {
  pub session: Rc<Session>,
  pub title: String,
  /// The title shown until a program sets its own, and again once it resets it.
  default_title: String,
  pub directory: Option<PathBuf>,
  pub content_bounds: Rc<std::cell::Cell<Option<Bounds<Pixels>>>>,
  line: String,
  line_unreliable: bool,
  /// Fractional lines left over from smooth scrolling.
  pub scroll_remainder: f32,
  pub selection: Option<((usize, usize), (usize, usize))>,
  pub selection_dragging: bool,
  pub hover_cell: Option<(usize, usize)>,
  /// The button held while the program receives mouse events, to report its release.
  pub mouse_button: Option<mterm_terminal::MouseButton>,
  /// The cell of the last mouse event sent to the program, so motion is reported once per cell.
  pub mouse_cell: Option<(usize, usize)>,
  wakeup_pending: Arc<AtomicBool>,
}

/// A tab: an arrangement of panes and the one that has the keyboard.
pub(crate) struct Tab {
  pub root: Node,
  pub focused: PaneId,
  pub saved: SavedSession,
  pub(crate) history: Rc<RefCell<History>>,
}

pub struct Workspace {
  pub(crate) panes: HashMap<PaneId, Pane>,
  pub(crate) tabs: Vec<Tab>,
  pub(crate) active: usize,
  session_tabs: HashMap<String, PaneId>,
  next_pane: PaneId,
  directory: Option<PathBuf>,
  events: futures::channel::mpsc::UnboundedSender<(PaneId, SessionEvent)>,
  pub(crate) error: Option<String>,
  pub(crate) saved_sessions: Vec<SavedSession>,
  /// The Settings page shown in place of the terminal, when open.
  pub(crate) settings: Option<SettingsPage>,
  /// The find bar, when open.
  pub(crate) find: Option<crate::find::Find>,
  settings_epoch: u64,
  pub(crate) model: Entity<AppModel>,
  applied_theme: ThemeChoice,
  pub(crate) focus_handle: FocusHandle,
  _subscriptions: Vec<Subscription>,
}

impl Workspace {
  /// `directory` is where shells start; `None` is the home folder.
  pub fn new(directory: Option<PathBuf>, window: &mut Window, cx: &mut Context<Self>) -> Self {
    let model = cx.global::<AppGlobal>().0.clone();
    let applied_theme = model.read(cx).theme();
    apply_theme(applied_theme, window, cx);
    let subscriptions = vec![
      cx.observe_window_appearance(window, |this, window, cx| apply_theme(this.applied_theme, window, cx)),
      // The dimming of inactive windows follows activation. Every window re-renders, since one
      // gaining focus changes whether the others are dimmed.
      cx.observe_window_activation(window, |this, window, cx| {
        if let Some(pane) = this.focused_pane() {
          pane.session.report_focus(window.is_window_active());
        }
        if window.is_window_active() {
          this.sync_saved_sessions(cx);
        }
        cx.notify();
        cx.defer(|cx| {
          for handle in cx.windows() {
            let _ = handle.update(cx, |_, window, _| window.refresh());
          }
        });
      }),
      // Settings changes (the theme) and update news reach every window.
      cx.observe_in(&model, window, |this, model, window, cx| {
        let theme = model.read(cx).theme();
        if theme != this.applied_theme {
          this.applied_theme = theme;
          apply_theme(theme, window, cx);
        }
        this.sync_saved_sessions(cx);
        this.announce_update(window, cx);
        cx.notify();
      }),
    ];

    let (events, mut receiver) = futures::channel::mpsc::unbounded();
    cx.spawn_in(window, async move |this, cx| {
      while let Some(first) = receiver.next().await {
        // Output arrives in bursts: handle everything queued in one update.
        let mut batch = vec![first];
        while let Ok(event) = receiver.try_recv() {
          batch.push(event);
        }
        if this.update_in(cx, |this, window, cx| this.handle_events(batch, window, cx)).is_err() {
          break;
        }
      }
    })
    .detach();

    let focus_handle = cx.focus_handle();
    focus_handle.focus(window, cx);
    let mut workspace = Self {
      panes: HashMap::new(),
      tabs: Vec::new(),
      active: 0,
      next_pane: 0,
      session_tabs: HashMap::new(),
      directory,
      events,
      error: None,
      saved_sessions: SavedSession::list(&SavedSession::directory_path()),
      settings: None,
      find: None,
      settings_epoch: 0,
      model,
      applied_theme,
      focus_handle,
      _subscriptions: subscriptions,
    };
    if workspace.directory.is_some() || workspace.saved_sessions.is_empty() || cx.windows().len() > 1 {
      workspace.new_session(cx);
    } else {
      let last = SavedSession::last_selected(&SavedSession::directory_path());
      let saved =
        workspace.saved_sessions.iter().find(|saved| Some(&saved.id) == last.as_ref()).or_else(|| workspace.saved_sessions.first()).cloned();
      if let Some(saved) = saved {
        workspace.open_session(saved, cx);
      }
    }

    workspace
  }

  /// Shows Settings, or closes them when already open.
  pub(crate) fn toggle_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    if self.settings.is_some() {
      self.close_settings(window, cx);
    } else {
      self.open_settings(SettingsPage::General, window, cx);
    }
  }

  pub(crate) fn open_settings(&mut self, page: SettingsPage, window: &mut Window, cx: &mut Context<Self>) {
    self.settings = Some(page);
    self.settings_epoch += 1;
    self.focus_handle.focus(window, cx);
    cx.notify();
  }

  pub(crate) fn close_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    self.settings = None;
    self.focus_handle.focus(window, cx);
    cx.notify();
  }

  /// Shows a notification, in one window, for each newly found release.
  fn announce_update(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let AppUpdate::Available(release) = self.model.read(cx).update.clone() else { return };
    if self.model.read(cx).announced.as_deref() == Some(release.version.as_str()) {
      return;
    }
    self.model.update(cx, |model, _| model.announced = Some(release.version.clone()));
    let workspace = cx.weak_entity();
    window.push_notification(
      component::notification::Notification::info(format!("mTerm {} is available.", release.version)).title("Update available").action(
        move |_, _, cx| {
          let workspace = workspace.clone();
          Button::new("open-update-settings").label("View update").small().on_click(cx.listener(move |notification, _, window, cx| {
            let _ = workspace.update(cx, |workspace, cx| workspace.open_settings(SettingsPage::Updates, window, cx));
            notification.dismiss(window, cx);
          }))
        },
      ),
      cx,
    );
  }

  fn handle_events(&mut self, events: Vec<(PaneId, SessionEvent)>, window: &mut Window, cx: &mut Context<Self>) {
    let visible_panes =
      if self.settings.is_none() { self.tabs.get(self.active).map(|tab| tab.root.leaves()).unwrap_or_default() } else { Vec::new() };
    let mut redraw = false;
    for (id, event) in events {
      match event {
        SessionEvent::Wakeup => {
          if let Some(pane) = self.panes.get(&id) {
            pane.wakeup_pending.store(false, Ordering::Release);
            redraw |= visible_panes.contains(&id);
          }
        }
        SessionEvent::Title(title) => {
          if let Some(pane) = self.panes.get_mut(&id) {
            // An empty title is a reset: go back to the shell's name once a program like vim exits.
            pane.title = if title.is_empty() { pane.default_title.clone() } else { title };
            redraw = true;
          }
        }
        SessionEvent::ClipboardStore(text) => cx.write_to_clipboard(ClipboardItem::new_string(text)),
        SessionEvent::Directory(directory) => {
          redraw = true;
          if let Some(pane) = self.panes.get_mut(&id) {
            pane.directory = Some(directory.clone());
          }
          if let Some(tab) = self.tabs.iter().find(|tab| tab.root.leaves().contains(&id)) {
            if tab.focused == id && tab.saved.directory != directory {
              let mut updated = tab.saved.clone();
              updated.directory = directory;
              if let Err(error) = updated.save(&SavedSession::directory_path()) {
                eprintln!("Could not save the session directory: {error}");
              }
              if let Some(saved) = self.saved_sessions.iter_mut().find(|saved| saved.id == updated.id) {
                *saved = updated.clone();
              }
              for tab in self.tabs.iter_mut().filter(|tab| tab.saved.id == updated.id) {
                tab.saved = updated.clone();
              }
            }
          }
        }
        // Leaving the shell closes its pane, like iTerm2.
        SessionEvent::Exit => {
          self.close_pane(id, window);
          redraw = true;
        }
      }
    }
    if redraw {
      cx.notify();
    }
  }

  // Panes, tabs and splits

  fn spawn_pane(&mut self, directory: Option<PathBuf>, cx: &App) -> Option<PaneId> {
    let id = self.next_pane;
    let sender = self.events.clone();
    let wakeup_pending = Arc::new(AtomicBool::new(false));
    let pending = wakeup_pending.clone();
    let options = SessionOptions {
      program: None,
      working_directory: directory.or_else(|| std::env::var_os("HOME").map(Into::into)),
      palette: Some(Self::palette(cx)),
    };
    let pane_directory = options.working_directory.clone();
    match Session::spawn(options, 80, 24, move |event| {
      if event == SessionEvent::Wakeup && pending.swap(true, Ordering::AcqRel) {
        return;
      }
      let _ = sender.unbounded_send((id, event));
    }) {
      Ok(session) => {
        self.next_pane += 1;
        let title =
          std::env::var("SHELL").ok().and_then(|path| path.rsplit('/').next().map(|name| format!("-{name}"))).unwrap_or_else(|| "mTerm".into());
        self.panes.insert(
          id,
          Pane {
            session: Rc::new(session),
            default_title: title.clone(),
            title,
            directory: pane_directory,
            content_bounds: Rc::new(std::cell::Cell::new(None)),
            line: String::new(),
            line_unreliable: false,
            scroll_remainder: 0.,
            selection: None,
            selection_dragging: false,
            hover_cell: None,
            mouse_button: None,
            mouse_cell: None,
            wakeup_pending,
          },
        );
        Some(id)
      }
      Err(error) => {
        self.error = Some(format!("Could not start the shell: {error}"));
        None
      }
    }
  }

  pub(crate) fn new_session(&mut self, cx: &App) {
    let migrate_legacy = !SavedSession::directory_path().exists();
    let directory = self.directory.clone().or_else(|| std::env::var_os("HOME").map(Into::into)).unwrap_or_default();
    let saved = SavedSession::new(format!("Session {}", self.saved_sessions.len() + 1), directory);
    if let Err(error) = saved.save(&SavedSession::directory_path()) {
      self.error = Some(format!("Could not save the session: {error}"));
      return;
    }
    if migrate_legacy {
      let old_history = History::load(&History::default_path());
      if !old_history.entries().is_empty() {
        if let Err(error) = old_history.save(&saved.history_path(&SavedSession::directory_path())) {
          eprintln!("Could not migrate command history: {error}");
        }
      }
      if let Some(layout) = mterm_storage::SavedLayout::load() {
        if let Err(error) = saved.save_layout(&SavedSession::directory_path(), &layout) {
          eprintln!("Could not migrate pane layout: {error}");
        }
      }
    }
    self.saved_sessions.push(saved.clone());
    SavedSession::sort(&mut self.saved_sessions);
    self.open_session(saved, cx);
  }

  fn open_session(&mut self, saved: SavedSession, cx: &App) {
    if let Some(tab) = self.tabs.get(self.active) {
      self.session_tabs.insert(tab.saved.id.clone(), tab.focused);
    }
    let preferred = self.session_tabs.get(&saved.id);
    if let Some(index) = self
      .tabs
      .iter()
      .position(|tab| tab.saved.id == saved.id && Some(&tab.focused) == preferred)
      .or_else(|| self.tabs.iter().position(|tab| tab.saved.id == saved.id))
    {
      self.active = index;
      if let Err(error) = saved.remember_selected(&SavedSession::directory_path()) {
        eprintln!("Could not save the selected session: {error}");
      }
      return;
    }
    let directory = saved.directory.is_dir().then(|| saved.directory.clone());
    if let Some(id) = self.spawn_pane(directory.clone(), cx) {
      let history = Rc::new(RefCell::new(saved.history(&SavedSession::directory_path())));
      let mut root = Node::Leaf(id);
      if let Some(layout) = saved.load_layout(&SavedSession::directory_path()).and_then(|text| Node::decode(&text)) {
        let mut ids = vec![id];
        while ids.len() < layout.leaves().len() {
          let Some(id) = self.spawn_pane(directory.clone(), cx) else { break };
          ids.push(id);
        }
        if ids.len() == layout.leaves().len() {
          root = layout.map_leaves(&mut ids.into_iter());
        } else {
          for id in ids.into_iter().skip(1) {
            self.panes.remove(&id);
          }
        }
      }
      if let Err(error) = saved.remember_selected(&SavedSession::directory_path()) {
        eprintln!("Could not save the selected session: {error}");
      }
      self.tabs.push(Tab { root, focused: id, saved, history });
      self.active = self.tabs.len() - 1;
    }
  }

  pub(crate) fn new_tab(&mut self, cx: &App) {
    let Some(tab) = self.tabs.get(self.active) else { return };
    let saved = tab.saved.clone();
    let history = tab.history.clone();
    let directory = saved.directory.is_dir().then(|| saved.directory.clone());
    if let Some(id) = self.spawn_pane(directory, cx) {
      self.tabs.push(Tab { root: Node::Leaf(id), focused: id, saved, history });
      self.active = self.tabs.len() - 1;
    }
  }

  pub(crate) fn select_session(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
    if let Some(saved) = self.saved_sessions.iter().find(|session| session.id == id).cloned() {
      self.open_session(saved, cx);
      self.settings = None;
      self.focus_handle.focus(window, cx);
      cx.notify();
    }
  }

  /// Reconciles saved sessions across windows without discarding other live terminals.
  fn sync_saved_sessions(&mut self, cx: &App) {
    let saved = match SavedSession::try_list(&SavedSession::directory_path()) {
      Ok(saved) => saved,
      Err(error) => {
        eprintln!("Could not read saved sessions: {error}");
        return;
      }
    };
    let active_pane = self.tabs.get(self.active).map(|tab| tab.focused);
    let active_id = self.tabs.get(self.active).map(|tab| tab.saved.id.clone());
    let removed: Vec<_> =
      self.tabs.iter().filter(|tab| !saved.iter().any(|item| item.id == tab.saved.id)).flat_map(|tab| tab.root.leaves()).collect();
    for id in removed {
      self.panes.remove(&id);
    }
    self.tabs.retain(|tab| saved.iter().any(|item| item.id == tab.saved.id));
    for tab in &mut self.tabs {
      if let Some(item) = saved.iter().find(|item| item.id == tab.saved.id) {
        tab.saved = item.clone();
      }
    }
    self.saved_sessions = saved;
    let active_removed = active_id.as_ref().is_some_and(|id| !self.tabs.iter().any(|tab| &tab.saved.id == id));
    self.active = active_id
      .and_then(|id| {
        self
          .tabs
          .iter()
          .position(|tab| tab.saved.id == id && Some(tab.focused) == active_pane)
          .or_else(|| self.tabs.iter().position(|tab| tab.saved.id == id))
      })
      .unwrap_or_else(|| self.active.min(self.tabs.len().saturating_sub(1)));
    if active_removed {
      if let Some(tab) = self.tabs.get(self.active) {
        if let Err(error) = tab.saved.remember_selected(&SavedSession::directory_path()) {
          eprintln!("Could not save the selected session: {error}");
        }
      }
    }
    if self.tabs.is_empty() {
      if let Some(saved) = self.saved_sessions.first().cloned() {
        self.open_session(saved, cx);
      } else {
        self.new_session(cx);
      }
    }
  }

  pub(crate) fn toggle_session_pin(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
    let Some(mut saved) = self.saved_sessions.iter().find(|session| session.id == id).cloned() else { return };
    saved.pinned = !saved.pinned;
    if let Err(error) = saved.save(&SavedSession::directory_path()) {
      window.push_notification(component::notification::Notification::error(format!("Could not update the session: {error}")), cx);
      return;
    }
    self.sync_saved_sessions(cx);
    self.model.update(cx, |_, cx| cx.notify());
    cx.notify();
  }

  pub(crate) fn rename_session(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
    let Some(saved) = self.saved_sessions.iter().find(|saved| saved.id == id) else { return };
    let name = saved.name.clone();
    let input = cx.new(|cx| InputState::new(window, cx).default_value(name));
    let workspace = cx.weak_entity();
    let initial_focus = Cell::new(true);
    window.open_alert_dialog(cx, move |dialog, window, cx| {
      if initial_focus.replace(false) {
        input.focus_handle(cx).focus(window, cx);
      }
      let input_for_save = input.clone();
      let workspace = workspace.clone();
      let id = id.clone();
      dialog.confirm().title("Rename session").child(Input::new(&input)).ok_text("Save").on_ok(move |_, window, cx| {
        let name = input_for_save.read(cx).value().trim().to_owned();
        if name.is_empty() {
          window.push_notification(component::notification::Notification::error("Session name cannot be empty."), cx);
          return false;
        }
        workspace
          .update(cx, |this, cx| {
            let sessions = match SavedSession::try_list(&SavedSession::directory_path()) {
              Ok(sessions) => sessions,
              Err(error) => {
                window.push_notification(component::notification::Notification::error(format!("Could not read sessions: {error}")), cx);
                return false;
              }
            };
            let Some(mut saved) = sessions.into_iter().find(|saved| saved.id == id) else {
              window.push_notification(component::notification::Notification::error("The session no longer exists."), cx);
              return false;
            };
            saved.name = name;
            if let Err(error) = saved.save(&SavedSession::directory_path()) {
              window.push_notification(component::notification::Notification::error(format!("Could not rename the session: {error}")), cx);
              return false;
            }
            this.sync_saved_sessions(cx);
            this.model.update(cx, |_, cx| cx.notify());
            cx.notify();
            true
          })
          .unwrap_or(true)
      })
    });
  }

  pub(crate) fn confirm_delete_session(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
    let Some(saved) = self.saved_sessions.iter().find(|session| session.id == id) else { return };
    let title = format!("Delete {}?", saved.name);
    let detail = "This deletes the saved session, its command history and layout. Running terminals in this session will close. Files in its working directory are not deleted.";
    let answer = window.prompt(PromptLevel::Warning, &title, Some(detail), &["Cancel", "Delete Session"], cx);
    cx.spawn_in(window, async move |this, cx| {
      if answer.await == Ok(1) {
        let _ = this.update_in(cx, |this, window, cx| {
          let Some(saved) = this.saved_sessions.iter().find(|session| session.id == id).cloned() else { return };
          if let Err(error) = saved.delete(&SavedSession::directory_path()) {
            window.push_notification(component::notification::Notification::error(format!("Could not delete the session: {error}")), cx);
            return;
          }
          this.sync_saved_sessions(cx);
          this.model.update(cx, |_, cx| cx.notify());
          cx.notify();
        });
      }
    })
    .detach();
  }

  pub(crate) fn split(&mut self, split: Split, cx: &App) {
    let Some(focused) = self.tabs.get(self.active).map(|tab| tab.focused) else { return };
    let directory = self.tabs.get(self.active).map(|tab| tab.saved.directory.clone());
    if let Some(id) = self.spawn_pane(directory, cx) {
      let tab = &mut self.tabs[self.active];
      if tab.root.split(focused, id, split) {
        tab.focused = id;
        self.save_layout();
      } else {
        self.panes.remove(&id);
      }
    }
  }

  pub(crate) fn save_layout(&self) {
    if let Some(tab) = self.tabs.get(self.active) {
      if let Err(error) = tab.saved.save_layout(&SavedSession::directory_path(), &tab.root.encode()) {
        eprintln!("Could not save pane layout: {error}");
      }
    }
  }

  /// Closes a pane; its tab closes with the last pane, and the window with the last tab.
  pub(crate) fn close_pane(&mut self, id: PaneId, window: &mut Window) {
    if self.panes.remove(&id).is_none() {
      return;
    }
    if let Some(index) = self.tabs.iter().position(|tab| tab.root.leaves().contains(&id)) {
      let tab = self.tabs.remove(index);
      if let Some(root) = tab.root.remove(id) {
        let focused = if tab.focused == id { root.leaves()[0] } else { tab.focused };
        if let Err(error) = tab.saved.save_layout(&SavedSession::directory_path(), &root.encode()) {
          eprintln!("Could not save pane layout: {error}");
        }
        self.tabs.insert(index, Tab { root, focused, saved: tab.saved, history: tab.history });
      } else if self.active == index {
        self.active =
          self.tabs.iter().position(|other| other.saved.id == tab.saved.id).unwrap_or_else(|| index.min(self.tabs.len().saturating_sub(1)));
      } else if self.active > index {
        self.active -= 1;
      }
    }
    self.active = self.active.min(self.tabs.len().saturating_sub(1));
    if !self.tabs.is_empty() {
      self.save_layout();
    }
    if self.tabs.is_empty() {
      window.remove_window();
    }
  }

  pub(crate) fn close_tab(&mut self, index: usize, window: &mut Window) {
    if let Some(tab) = self.tabs.get(index) {
      for id in tab.root.leaves() {
        self.close_pane(id, window);
      }
    }
  }

  pub(crate) fn focus_pane(&mut self, id: PaneId, window: &mut Window, cx: &mut Context<Self>) {
    let previous = self.focused_pane_id();
    if let Some(tab) = self.tabs.get_mut(self.active) {
      if tab.root.leaves().contains(&id) {
        tab.focused = id;
      }
    }
    if previous != self.focused_pane_id() {
      if let Some(pane) = previous.and_then(|id| self.panes.get(&id)) {
        pane.session.report_focus(false);
      }
      if let Some(pane) = self.focused_pane() {
        pane.session.report_focus(true);
      }
    }
    self.focus_handle.focus(window, cx);
    cx.notify();
  }

  pub(crate) fn select_tab(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
    if let Some(tab) = self.tabs.get(index) {
      self.active = index;
      if let Err(error) = tab.saved.remember_selected(&SavedSession::directory_path()) {
        eprintln!("Could not save the selected session: {error}");
      }
    }
    self.focus_handle.focus(window, cx);
    cx.notify();
  }

  fn step_tab(&mut self, step: isize, window: &mut Window, cx: &mut Context<Self>) {
    let indices = self.active_tab_indices();
    if let Some(current) = indices.iter().position(|index| *index == self.active) {
      self.select_tab(indices[(current as isize + step).rem_euclid(indices.len() as isize) as usize], window, cx);
    }
  }

  fn step_pane(&mut self, step: isize, window: &mut Window, cx: &mut Context<Self>) {
    let Some(tab) = self.tabs.get(self.active) else { return };
    let leaves = tab.root.leaves();
    if leaves.len() < 2 {
      return;
    }
    let current = leaves.iter().position(|id| *id == tab.focused).unwrap_or(0);
    self.focus_pane(leaves[(current as isize + step).rem_euclid(leaves.len() as isize) as usize], window, cx);
  }

  /// Drops the focused pane's selection, which would no longer line up with the text after input or scrolling.
  pub(crate) fn clear_selection(&mut self) {
    if let Some(pane) = self.focused_pane_id().and_then(|id| self.panes.get_mut(&id)) {
      pane.selection = None;
    }
  }

  pub(crate) fn active_tab_indices(&self) -> Vec<usize> {
    let Some(active) = self.tabs.get(self.active) else { return Vec::new() };
    self.tabs.iter().enumerate().filter_map(|(index, tab)| (tab.saved.id == active.saved.id).then_some(index)).collect()
  }

  pub(crate) fn focused_pane_id(&self) -> Option<PaneId> {
    self.tabs.get(self.active).map(|tab| tab.focused)
  }

  pub(crate) fn focused_pane(&self) -> Option<&Pane> {
    self.focused_pane_id().and_then(|id| self.panes.get(&id))
  }

  /// The title of the focused pane, which names this window in the Window menu.
  pub fn title(&self) -> String {
    self.focused_pane().map(|pane| pane.title.clone()).unwrap_or_else(|| "mTerm".into())
  }

  // Input and history

  fn write(&self, bytes: Vec<u8>) {
    if self.settings.is_some() {
      return;
    }
    if let Some(pane) = self.focused_pane() {
      pane.session.write(bytes);
    }
  }

  fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
    // Settings own the keyboard while they are open; Escape returns to the terminal.
    if self.settings.is_some() {
      if event.keystroke.key == "escape" {
        self.close_settings(window, cx);
        cx.stop_propagation();
      }
      return;
    }
    let keystroke = &event.keystroke;
    // While the find bar has the keyboard, typing belongs to it; Escape closes it.
    if self.find.is_some() && !self.focus_handle.is_focused(window) {
      if keystroke.key == "escape" {
        self.close_find(window, cx);
        cx.stop_propagation();
      }
      return;
    }
    let held = keystroke.modifiers;
    // ⌘1…⌘9 pick a tab; ⌘9 is the last one, like iTerm2.
    if held.platform && !held.control && !held.alt && !held.shift {
      if let Some(digit) = keystroke.key.parse::<usize>().ok().filter(|digit| (1..=9).contains(digit)) {
        let indices = self.active_tab_indices();
        if let Some(index) = if digit == 9 { indices.last() } else { indices.get(digit - 1) }.copied() {
          self.select_tab(index, window, cx);
        }
        cx.stop_propagation();
        return;
      }
    }
    // ⇧PageUp, ⇧PageDown, ⇧Home and ⇧End move through the history instead of reaching the program.
    if held.shift && !held.control && !held.alt && !held.platform && matches!(keystroke.key.as_str(), "pageup" | "pagedown" | "home" | "end") {
      if let Some(pane) = self.focused_pane().filter(|pane| !pane.session.modes().alt_screen) {
        match keystroke.key.as_str() {
          "pageup" => pane.session.scroll_page(true),
          "pagedown" => pane.session.scroll_page(false),
          "home" => pane.session.scroll_to(usize::MAX),
          _ => pane.session.scroll_to(0),
        };
        self.clear_selection();
        cx.stop_propagation();
        cx.notify();
        return;
      }
    }
    if keystroke.modifiers.shift
      && !keystroke.modifiers.control
      && !keystroke.modifiers.alt
      && !keystroke.modifiers.platform
      && matches!(keystroke.key.as_str(), "up" | "down" | "left" | "right")
    {
      if let Some(id) = self.focused_pane_id() {
        let palette = Self::palette(cx);
        if let Some(pane) = self.panes.get_mut(&id) {
          if let Some((anchor, end)) = pane.selection {
            let snapshot = pane.session.snapshot(&palette);
            let lines: Vec<_> = (0..snapshot.lines.len()).map(|line| snapshot.line_text(line)).collect();
            pane.selection = Some((anchor, crate::selection::move_endpoint(&lines, end, &keystroke.key)));
            cx.stop_propagation();
            cx.notify();
            return;
          }
        }
      }
    }
    let Some(pane) = self.focused_pane() else { return };
    let modifiers = Modifiers {
      control: keystroke.modifiers.control,
      alt: keystroke.modifiers.alt,
      shift: keystroke.modifiers.shift,
      command: keystroke.modifiers.platform,
    };
    let text = keystroke.key_char.as_deref();
    let Some(bytes) = encode_key(&keystroke.key, text, modifiers, pane.session.modes().app_cursor) else { return };
    pane.session.write(bytes);
    self.track_input(&keystroke.key, text, modifiers);
    self.clear_selection();
    cx.stop_propagation();
    cx.notify();
  }

  /// Follows what is typed so a finished line can be added to the history.
  fn track_input(&mut self, key: &str, text: Option<&str>, modifiers: Modifiers) {
    let Some(id) = self.focused_pane_id() else { return };
    let Some(pane) = self.panes.get_mut(&id) else { return };
    match key {
      "enter" if modifiers.shift && !modifiers.alt && !modifiers.control && !modifiers.command => {
        pane.line_unreliable = true;
      }
      "enter" => {
        let line = std::mem::take(&mut pane.line);
        if !pane.line_unreliable && !line.trim().is_empty() {
          if let Some(tab) = self.tabs.get_mut(self.active) {
            let mut history = tab.history.borrow_mut();
            if history.push(&line) {
              if let Err(error) = history.save(&tab.saved.history_path(&SavedSession::directory_path())) {
                eprintln!("Could not save the session history: {error}");
              }
            }
          }
        }
        pane.line_unreliable = false;
        pane.session.refresh_directory();
      }
      "backspace" if modifiers.alt || modifiers.command => pane.line_unreliable = true,
      "backspace" => {
        pane.line.pop();
      }
      "up" | "down" | "left" | "right" | "home" | "end" | "tab" | "delete" | "escape" => pane.line_unreliable = true,
      "c" | "u" | "g" if modifiers.control => {
        pane.line.clear();
        pane.line_unreliable = false;
      }
      _ if modifiers.control || modifiers.command => pane.line_unreliable = true,
      _ => match text {
        Some(text) => pane.line.push_str(text),
        None if key == "space" => pane.line.push(' '),
        None => {}
      },
    }
  }

  /// Types text at the prompt without running it.
  pub(crate) fn insert_text(&mut self, text: &str, window: &mut Window, cx: &mut Context<Self>) {
    // Typing into the terminal brings it back from Settings.
    self.settings = None;
    if let Some(pane) = self.focused_pane() {
      pane.session.write(paste_bytes(text, pane.session.modes().bracketed_paste));
    }
    if let Some(id) = self.focused_pane_id() {
      if let Some(pane) = self.panes.get_mut(&id) {
        pane.line.push_str(text);
      }
    }
    self.focus_handle.focus(window, cx);
    cx.notify();
  }

  /// Clears only the active session's recorded commands after confirmation.
  pub(crate) fn confirm_clear_history(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let Some(tab) = self.tabs.get(self.active) else { return };
    if tab.history.borrow().entries().is_empty() {
      return;
    }
    let id = tab.saved.id.clone();
    let answer = window.prompt(
      PromptLevel::Warning,
      "Clear session history?",
      Some("This removes the recorded commands for this session."),
      &["Cancel", "Clear History"],
      cx,
    );
    cx.spawn(async move |this, cx| {
      if answer.await == Ok(1) {
        let _ = this.update(cx, |this, cx| {
          if let Some(tab) = this.tabs.iter_mut().find(|tab| tab.saved.id == id) {
            let mut history = tab.history.borrow_mut();
            history.clear();
            if let Err(error) = history.save(&tab.saved.history_path(&SavedSession::directory_path())) {
              eprintln!("Could not clear the session history: {error}");
            }
          }
          cx.notify();
        });
      }
    })
    .detach();
  }

  fn copy(&mut self, cx: &mut Context<Self>) {
    let Some(pane) = self.focused_pane() else { return };
    let Some((anchor, end)) = pane.selection else { return };
    if anchor == end {
      return;
    }
    let (start, end) = if anchor < end { (anchor, end) } else { (end, anchor) };
    let snapshot = pane.session.snapshot(&Self::palette(cx));
    let last = end.0.min(snapshot.lines.len().saturating_sub(1));
    let mut copied = String::new();
    for line in start.0..=last {
      let text: String = snapshot.lines[line].iter().map(|span| span.text.as_str()).collect();
      let from = if line == start.0 { start.1 } else { 0 };
      let to = if line == end.0 { end.1 } else { snapshot.columns };
      let mut column = 0;
      let selected: String = text
        .chars()
        .filter(|character| {
          let width = unicode_width::UnicodeWidthChar::width(*character).unwrap_or(0);
          let include = column < to && column + width > from;
          column += width;
          include
        })
        .collect();
      // A line that only wrapped at the screen edge is one line of text: keep it whole.
      if line < last && snapshot.wrapped.get(line).copied().unwrap_or(false) {
        copied.push_str(&selected);
      } else {
        copied.push_str(selected.trim_end());
        copied.push('\n');
      }
    }
    cx.write_to_clipboard(ClipboardItem::new_string(copied.trim_end_matches('\n').to_owned()));
  }

  pub(crate) fn paste(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    if self.settings.is_some() {
      return;
    }
    let Some(item) = cx.read_from_clipboard() else { return };
    if item.entries.iter().any(|entry| matches!(entry, ClipboardEntry::Image(_))) {
      // Terminal applications such as Codex read image data from the clipboard themselves.
      self.write(vec![0x16]);
    } else if let Some(text) = item.text() {
      if text.contains('\n') {
        if let Some(id) = self.focused_pane_id() {
          if let Some(pane) = self.panes.get_mut(&id) {
            pane.line_unreliable = true;
          }
        }
      }
      self.insert_text(&text, window, cx);
    }
  }

  pub(crate) fn palette(cx: &App) -> Palette {
    let choice = cx.global::<AppGlobal>().0.read(cx).theme();
    if matches!(choice, ThemeChoice::GithubDark | ThemeChoice::GithubLight) {
      let dark = choice == ThemeChoice::GithubDark;
      let mut palette = if dark { Palette::dark() } else { Palette::light() };
      palette.background = if dark { mterm_terminal::Rgb::new(0x0d, 0x11, 0x17) } else { mterm_terminal::Rgb::new(0xff, 0xff, 0xff) };
      palette.foreground = if dark { mterm_terminal::Rgb::new(0xc9, 0xd1, 0xd9) } else { mterm_terminal::Rgb::new(0x24, 0x29, 0x2f) };
      palette
    } else if choice == ThemeChoice::SlateDark {
      let mut palette = Palette::dark();
      palette.background = mterm_terminal::Rgb::new(0x1e, 0x24, 0x28);
      palette.foreground = mterm_terminal::Rgb::new(0xe4, 0xe8, 0xed);
      palette.ansi[0] = palette.background;
      palette.ansi[7] = palette.foreground;
      palette
    } else if choice == ThemeChoice::SlateLight {
      let mut palette = Palette::light();
      palette.background = mterm_terminal::Rgb::new(0xfa, 0xfa, 0xfa);
      palette.foreground = mterm_terminal::Rgb::new(0x1e, 0x29, 0x3b);
      palette.ansi[0] = palette.foreground;
      palette.ansi[7] = palette.background;
      palette
    } else if cx.theme().is_dark() {
      Palette::dark()
    } else {
      Palette::light()
    }
  }
}

impl Render for Workspace {
  fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let theme = cx.theme().clone();
    let content = self.render_tab_content(window, cx);
    let page = v_flex().flex_1().min_w_0().h_full().child(self.render_band(cx)).child(content);
    // Only dim while another mTerm window has the focus. When the whole app goes to the background
    // (another app in front), the windows keep their normal look.
    let inactive = !window.is_window_active() && another_window_is_active(window, cx);
    h_flex()
      .id("workspace")
      .relative()
      .key_context(CONTEXT)
      .track_focus(&self.focus_handle)
      .on_key_down(cx.listener(Self::on_key_down))
      .on_action(cx.listener(|this, _: &Copy, _, cx| this.copy(cx)))
      .on_action(cx.listener(|this, _: &Paste, window, cx| this.paste(window, cx)))
      .on_action(cx.listener(|this, _: &OpenSettings, window, cx| this.toggle_settings(window, cx)))
      .on_action(cx.listener(|this, _: &CheckForUpdates, window, cx| {
        this.open_settings(SettingsPage::Updates, window, cx);
        this.model.update(cx, |model, cx| model.check_update(cx));
      }))
      .on_action(cx.listener(|this, _: &Find, window, cx| this.open_find(window, cx)))
      .on_action(cx.listener(|this, _: &FindNext, _, cx| this.step_find(true, cx)))
      .on_action(cx.listener(|this, _: &FindPrevious, _, cx| this.step_find(false, cx)))
      .on_action(cx.listener(|this, _: &SelectAll, _, cx| {
        let Some(pane) = this.focused_pane_id().and_then(|id| this.panes.get_mut(&id)) else { return };
        let snapshot = pane.session.snapshot(&Self::palette(cx));
        pane.selection = Some(((0, 0), (snapshot.lines.len().saturating_sub(1), snapshot.columns)));
        cx.notify();
      }))
      .on_action(cx.listener(|this, _: &ClearScreen, _, cx| {
        // Like iTerm2's Clear Buffer: the scrollback goes too, and the prompt redraws on a clean screen.
        if let Some(pane) = this.focused_pane() {
          pane.session.clear_history();
        }
        this.clear_selection();
        this.write(vec![0x0c]);
        cx.notify();
      }))
      .on_action(cx.listener(|this, _: &IncreaseFontSize, _, cx| this.model.update(cx, |model, cx| model.zoom_terminal_font(1, cx))))
      .on_action(cx.listener(|this, _: &DecreaseFontSize, _, cx| this.model.update(cx, |model, cx| model.zoom_terminal_font(-1, cx))))
      .on_action(cx.listener(|this, _: &ResetFontSize, _, cx| this.model.update(cx, |model, cx| model.reset_terminal_font(cx))))
      .on_action(cx.listener(|this, _: &NextPane, window, cx| this.step_pane(1, window, cx)))
      .on_action(cx.listener(|this, _: &PreviousPane, window, cx| this.step_pane(-1, window, cx)))
      .on_action(cx.listener(|this, _: &ClearHistory, window, cx| this.confirm_clear_history(window, cx)))
      .on_action(cx.listener(|this, _: &NewTab, window, cx| {
        this.new_tab(cx);
        this.focus_handle.focus(window, cx);
        cx.notify();
      }))
      .on_action(cx.listener(|this, _: &SplitVertically, _, cx| {
        this.split(Split::Vertically, cx);
        cx.notify();
      }))
      .on_action(cx.listener(|this, _: &SplitHorizontally, _, cx| {
        this.split(Split::Horizontally, cx);
        cx.notify();
      }))
      .on_action(cx.listener(|this, _: &ClosePane, window, cx| {
        if let Some(id) = this.focused_pane_id() {
          this.close_pane(id, window);
        }
        cx.notify();
      }))
      .on_action(cx.listener(|this, _: &NextTab, window, cx| this.step_tab(1, window, cx)))
      .on_action(cx.listener(|this, _: &PreviousTab, window, cx| this.step_tab(-1, window, cx)))
      .size_full()
      .bg(theme.background)
      .text_color(theme.foreground)
      .child(page)
      .when_some(self.settings, |this, settings_page| {
        this.child(
          div().absolute().inset_0().flex().items_center().justify_center().bg(black().opacity(0.45)).child(
            v_flex()
              .w(px(600.))
              .max_w_full()
              .h(px(600.))
              .max_h_full()
              .overflow_hidden()
              .rounded_lg()
              .border_1()
              .border_color(theme.border)
              .bg(theme.background)
              .child(
                h_flex()
                  .h(px(50.))
                  .px_4()
                  .items_center()
                  .border_b_1()
                  .border_color(theme.border)
                  .child(div().flex_1().font_weight(FontWeight::SEMIBOLD).child("Settings"))
                  .child(crate::history::icon_button(
                    "close-settings",
                    gpui_kit::component::IconName::Close,
                    "Close Settings (Esc)",
                    cx.listener(|this, _, window, cx| this.close_settings(window, cx)),
                  )),
              )
              .child(div().flex_1().min_h_0().child(render_settings(&self.model, settings_page, self.settings_epoch, cx))),
          ),
        )
      })
      .when_some(self.render_find_bar(cx), |this, bar| this.child(bar))
      // A soft black veil over every window that is not the one in use.
      .when(inactive, |this| this.child(div().absolute().top_0().left_0().size_full().bg(black().opacity(INACTIVE_VEIL))))
  }
}

/// Whether a different mTerm window is the one in use.
fn another_window_is_active(window: &Window, cx: &mut App) -> bool {
  let this = window.window_handle();
  cx.windows().into_iter().filter(|handle| *handle != this).any(|handle| handle.update(cx, |_, other, _| other.is_window_active()).unwrap_or(false))
}
