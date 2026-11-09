use crate::{
  components::log_failure,
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
use mterm_storage::{History, SavedTab, SavedWindow, ThemeChoice};
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

/// The width of the Settings window.
const SETTINGS_SIZE: Pixels = px(600.);

/// The height of the settings below the dialog's title row.
const SETTINGS_BODY_HEIGHT: Pixels = px(540.);

/// How dark the veil over an inactive window is (0 is clear, 1 is black).
pub(crate) const INACTIVE_VEIL: f32 = 0.35;

/// The key context key bindings use to reach a window's workspace.
pub const CONTEXT: &str = "Workspace";

/// One shell and its screen.
pub(crate) struct Pane {
  pub session: Rc<Session>,
  pub pinned: bool,
  pub title: SharedString,
  /// The title shown until a program sets its own, and again once it resets it.
  default_title: SharedString,
  pub directory: Option<PathBuf>,
  pub content_bounds: Rc<std::cell::Cell<Option<Bounds<Pixels>>>>,
  pub(crate) line: String,
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
  pub pinned: bool,
  /// A name chosen by the user, shown instead of the focused pane's title.
  pub title: Option<SharedString>,
  /// Identifies the tab for as long as it lives, whatever its position.
  pub id: u64,
}

pub struct Workspace {
  pub(crate) panes: HashMap<PaneId, Pane>,
  pub(crate) tabs: Vec<Tab>,
  pub(crate) active: usize,
  next_pane: PaneId,
  next_tab: u64,
  directory: Option<PathBuf>,
  events: futures::channel::mpsc::UnboundedSender<(PaneId, SessionEvent)>,
  pub(crate) error: Option<String>,
  /// The commands entered in any tab of this window, shared by all of them.
  pub(crate) history: Rc<RefCell<History>>,
  /// The Settings page shown in place of the terminal, when open.
  pub(crate) settings: Option<SettingsPage>,
  /// The find bar, when open.
  pub(crate) find: Option<crate::find::Find>,
  settings_epoch: u64,
  /// Whether the Settings dialog has appeared, so its closing by Esc or its button is noticed.
  settings_shown: bool,
  pub(crate) model: Entity<AppModel>,
  applied_theme: ThemeChoice,
  pub(crate) focus_handle: FocusHandle,
  _subscriptions: Vec<Subscription>,
  _events_task: Task<()>,
  pub(crate) drop_tasks: HashMap<u64, Task<()>>,
  pub(crate) next_drop: u64,
}

impl Workspace {
  /// `directory` is where new shells start; `None` is the home folder. `restore` brings back the
  /// tabs and panes of a window from the last run.
  pub fn new(directory: Option<PathBuf>, restore: Option<SavedWindow>, window: &mut Window, cx: &mut Context<Self>) -> Self {
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
        this.announce_update(window, cx);
        cx.notify();
      }),
    ];

    let (events, mut receiver) = futures::channel::mpsc::unbounded();
    // Kept on the workspace: closing the window drops the task and stops listening.
    let events_task = cx.spawn_in(window, async move |this, cx| {
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
    });

    let focus_handle = cx.focus_handle();
    focus_handle.focus(window, cx);
    let mut workspace = Self {
      panes: HashMap::new(),
      tabs: Vec::new(),
      active: 0,
      next_pane: 0,
      next_tab: 0,
      directory,
      events,
      error: None,
      history: Rc::new(RefCell::new(History::load(&History::default_path()))),
      settings: None,
      find: None,
      settings_epoch: 0,
      settings_shown: false,
      model,
      applied_theme,
      focus_handle,
      _subscriptions: subscriptions,
      _events_task: events_task,
      drop_tasks: HashMap::new(),
      next_drop: 0,
    };
    if let Some(saved) = restore {
      for tab in &saved.tabs {
        workspace.restore_tab(tab, cx);
      }
      workspace.active = saved.active.min(workspace.tabs.len().saturating_sub(1));
    }
    if workspace.tabs.is_empty() {
      workspace.new_tab(cx);
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
    self.settings_shown = false;
    window.close_all_dialogs(cx);
    let (model, epoch) = (self.model.clone(), self.settings_epoch);
    window.open_dialog(cx, move |dialog, _, cx| {
      let border = cx.theme().border;
      // No dialog padding: the title row and the settings run edge to edge, split by a hairline.
      dialog
        .p_0()
        .title(div().px_4().py_3().font_weight(FontWeight::SEMIBOLD).child("Settings"))
        .width(SETTINGS_SIZE)
        .max_w(SETTINGS_SIZE)
        .child(div().h(SETTINGS_BODY_HEIGHT).border_t_1().border_color(border).child(render_settings(&model, page, epoch, cx)))
    });
    cx.notify();
  }

  pub(crate) fn close_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    self.settings = None;
    self.settings_shown = false;
    window.close_all_dialogs(cx);
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
            pane.title = if title.is_empty() { pane.default_title.clone() } else { title.into() };
            redraw = true;
          }
        }
        SessionEvent::ClipboardStore(text) => cx.write_to_clipboard(ClipboardItem::new_string(text)),
        SessionEvent::Directory(directory) => {
          redraw = true;
          if let Some(pane) = self.panes.get_mut(&id) {
            pane.directory = Some(directory.clone());
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
            pinned: false,
            default_title: title.clone().into(),
            title: title.into(),
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

  /// Opens a tab in the directory of the focused pane.
  pub(crate) fn new_tab(&mut self, cx: &App) {
    let directory = self.focused_pane().and_then(|pane| pane.directory.clone()).or_else(|| self.directory.clone());
    if let Some(id) = self.spawn_pane(directory, cx) {
      let tab = self.new_tab_value(Node::Leaf(id), id);
      self.tabs.push(tab);
      self.active = self.tabs.len() - 1;
    }
  }

  /// A tab with a fresh identity.
  fn new_tab_value(&mut self, root: Node, focused: PaneId) -> Tab {
    self.next_tab += 1;
    Tab { root, focused, pinned: false, title: None, id: self.next_tab }
  }

  /// Opens a saved tab: its split layout with a fresh shell in each pane's last directory.
  /// A directory that no longer exists falls back to the home folder.
  fn spawn_tab(&mut self, saved: &SavedTab, cx: &App) -> Option<Tab> {
    let directory = |index: usize| saved.directories.get(index).filter(|directory| directory.is_dir()).cloned();
    let layout = Node::decode(&saved.layout).filter(|layout| layout.leaves().len() == saved.directories.len()).unwrap_or(Node::Leaf(0));
    let mut ids = Vec::new();
    for index in 0..layout.leaves().len() {
      let Some(id) = self.spawn_pane(directory(index), cx) else { break };
      ids.push(id);
    }
    if ids.len() != layout.leaves().len() {
      for id in ids {
        self.panes.remove(&id);
      }
      return None;
    }
    for (index, id) in ids.iter().enumerate() {
      if let Some(pane) = self.panes.get_mut(id) {
        pane.pinned = saved.pinned_panes.get(index).copied().unwrap_or(false);
      }
    }
    let focused = ids[saved.focused.min(ids.len() - 1)];
    let mut tab = self.new_tab_value(layout.map_leaves(&mut ids.into_iter()), focused);
    tab.pinned = saved.pinned;
    tab.title = saved.title.clone().map(Into::into);
    Some(tab)
  }

  fn restore_tab(&mut self, saved: &SavedTab, cx: &App) {
    if let Some(tab) = self.spawn_tab(saved, cx) {
      self.tabs.push(tab);
    }
  }

  fn saved_tab(&self, tab: &Tab) -> SavedTab {
    let leaves = tab.root.leaves();
    SavedTab {
      layout: tab.root.encode(),
      directories: leaves.iter().map(|id| self.panes.get(id).and_then(|pane| pane.directory.clone()).unwrap_or_default()).collect(),
      focused: leaves.iter().position(|id| *id == tab.focused).unwrap_or(0),
      pinned: tab.pinned,
      pinned_panes: leaves.iter().map(|id| self.panes.get(id).is_some_and(|pane| pane.pinned)).collect(),
      title: tab.title.as_ref().map(|title| title.to_string()),
    }
  }

  /// Opens a new tab right after `index`.
  pub(crate) fn new_tab_after(&mut self, index: usize, cx: &App) {
    let Some(tab) = self.tabs.get(index) else { return };
    let directory = self.panes.get(&tab.focused).and_then(|pane| pane.directory.clone());
    if let Some(id) = self.spawn_pane(directory, cx) {
      let tab = self.new_tab_value(Node::Leaf(id), id);
      self.tabs.insert(index + 1, tab);
      self.active = index + 1;
    }
  }

  /// Asks for a new name for a tab; an empty name goes back to the shell's title.
  pub(crate) fn rename_tab(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
    let Some(tab) = self.tabs.get(index) else { return };
    let focused = tab.focused;
    let name = self.tab_title(tab);
    let input = cx.new(|cx| InputState::new(window, cx).default_value(name));
    let workspace = cx.weak_entity();
    let initial_focus = Cell::new(true);
    window.open_alert_dialog(cx, move |dialog, window, cx| {
      if initial_focus.replace(false) {
        input.focus_handle(cx).focus(window, cx);
      }
      let input = input.clone();
      let workspace = workspace.clone();
      dialog.confirm().title("Rename tab").child(Input::new(&input)).ok_text("Save").on_ok(move |_, _, cx| {
        let name = input.read(cx).value().trim().to_owned();
        let _ = workspace.update(cx, |this, cx| {
          // The tab may have moved or closed while the dialog was open.
          if let Some(tab) = this.tabs.iter_mut().find(|tab| tab.focused == focused) {
            tab.title = (!name.is_empty()).then(|| name.into());
          }
          cx.notify();
        });
        true
      })
    });
  }

  /// Restarts the shells of a tab, keeping its layout and directories.
  pub(crate) fn reload_tab(&mut self, index: usize, cx: &App) {
    let Some(old) = self.tabs.get(index) else { return };
    let saved = self.saved_tab(old);
    if let Some(mut tab) = self.spawn_tab(&saved, cx) {
      tab.id = self.tabs[index].id;
      let old = std::mem::replace(&mut self.tabs[index], tab);
      for id in old.root.leaves() {
        self.panes.remove(&id);
      }
    }
  }

  /// Opens a copy of a tab, with its layout and directories, right after it.
  pub(crate) fn duplicate_tab(&mut self, index: usize, cx: &App) {
    let Some(old) = self.tabs.get(index) else { return };
    let mut saved = self.saved_tab(old);
    saved.pinned = false;
    saved.title = saved.title.map(|title| format!("{title} copy"));
    if let Some(tab) = self.spawn_tab(&saved, cx) {
      self.tabs.insert(index + 1, tab);
      self.active = index + 1;
    }
  }

  /// Pins or unpins a tab; pinned tabs stay at the front.
  pub(crate) fn toggle_tab_pin(&mut self, index: usize) {
    let Some(tab) = self.tabs.get_mut(index) else { return };
    tab.pinned = !tab.pinned;
    let active = self.tabs.get(self.active).map(|tab| tab.focused);
    self.tabs.sort_by_key(|tab| !tab.pinned);
    if let Some(active) = active {
      self.active = self.tabs.iter().position(|tab| tab.root.leaves().contains(&active)).unwrap_or(0);
    }
  }

  /// Closes every tab but this one, leaving pinned tabs open.
  pub(crate) fn close_other_tabs(&mut self, index: usize, window: &mut Window) {
    let Some(keep) = self.tabs.get(index).map(|tab| tab.focused) else { return };
    let others: Vec<_> = self.tabs.iter().filter(|tab| !tab.pinned && tab.focused != keep).flat_map(|tab| tab.root.leaves()).collect();
    for id in others {
      self.close_pane(id, window);
    }
    if let Some(position) = self.tabs.iter().position(|tab| tab.focused == keep) {
      self.active = position;
    }
  }

  /// What is open in this window, for the next launch to continue from.
  pub fn saved_window(&self) -> SavedWindow {
    let tabs = self.tabs.iter().map(|tab| self.saved_tab(tab)).collect();
    // The window's position is only known to the window itself; the app fills it in.
    SavedWindow { tabs, active: self.active, bounds: None }
  }

  pub(crate) fn split(&mut self, split: Split, cx: &App) {
    let Some(focused) = self.tabs.get(self.active).map(|tab| tab.focused) else { return };
    let directory = self.focused_pane().and_then(|pane| pane.directory.clone());
    if let Some(id) = self.spawn_pane(directory, cx) {
      let tab = &mut self.tabs[self.active];
      if tab.root.split(focused, id, split) {
        tab.focused = id;
      } else {
        self.panes.remove(&id);
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
        self.tabs.insert(index, Tab { root, focused, pinned: tab.pinned, title: tab.title, id: tab.id });
      } else if self.active == index {
        self.active = index.min(self.tabs.len().saturating_sub(1));
      } else if self.active > index {
        self.active -= 1;
      }
    }
    self.active = self.active.min(self.tabs.len().saturating_sub(1));
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
    if index < self.tabs.len() {
      self.active = index;
    }
    self.focus_handle.focus(window, cx);
    cx.notify();
  }

  fn step_tab(&mut self, step: isize, window: &mut Window, cx: &mut Context<Self>) {
    if !self.tabs.is_empty() {
      self.select_tab((self.active as isize + step).rem_euclid(self.tabs.len() as isize) as usize, window, cx);
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

  pub(crate) fn focused_pane_id(&self) -> Option<PaneId> {
    self.tabs.get(self.active).map(|tab| tab.focused)
  }

  pub(crate) fn focused_pane(&self) -> Option<&Pane> {
    self.focused_pane_id().and_then(|id| self.panes.get(&id))
  }

  /// The title of a tab: the name the user gave it, or the focused pane's title.
  pub(crate) fn tab_title(&self, tab: &Tab) -> SharedString {
    tab.title.clone().or_else(|| self.panes.get(&tab.focused).map(|pane| pane.title.clone())).unwrap_or_default()
  }

  /// The title of the selected tab, which names this window in the Window menu.
  pub fn title(&self) -> String {
    if let Some(title) = self.tabs.get(self.active).and_then(|tab| tab.title.clone()) {
      return title.to_string();
    }
    self.focused_pane().map(|pane| pane.title.to_string()).unwrap_or_else(|| "mTerm".into())
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
        let index = if digit == 9 { self.tabs.len().checked_sub(1) } else { Some(digit - 1).filter(|index| *index < self.tabs.len()) };
        if let Some(index) = index {
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
          let mut history = self.history.borrow_mut();
          if history.push(&line) {
            log_failure("save the command history", history.save(&History::default_path()));
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

  /// Clears the recorded commands after confirmation.
  pub(crate) fn confirm_clear_history(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    if self.history.borrow().entries().is_empty() {
      return;
    }
    let answer =
      window.prompt(PromptLevel::Warning, "Clear command history?", Some("This removes the recorded commands."), &["Cancel", "Clear History"], cx);
    cx.spawn(async move |this, cx| {
      if answer.await == Ok(1) {
        let _ = this.update(cx, |this, cx| {
          let mut history = this.history.borrow_mut();
          history.clear();
          log_failure("clear the command history", history.save(&History::default_path()));
          drop(history);
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
    // The dialog closes itself on Esc, so Settings follow whether it is still open.
    if self.settings.is_some() {
      if window.has_active_dialog(cx) {
        self.settings_shown = true;
      } else if self.settings_shown {
        self.settings = None;
        self.settings_shown = false;
        cx.defer_in(window, |this, window, cx| this.focus_handle.focus(window, cx));
      }
    }
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
