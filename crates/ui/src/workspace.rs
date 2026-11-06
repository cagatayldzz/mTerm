use crate::{
  layout::{Node, PaneId, Split},
  ClearHistory, ClearScreen, ClosePane, NewTab, NextTab, Paste, PreviousTab, SplitHorizontally, SplitVertically, ToggleSidebar,
};
use futures::StreamExt as _;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
  component::{h_flex, v_flex, ActiveTheme, Theme},
  *,
};
use mterm_storage::History;
use mterm_terminal::{encode_key, paste_bytes, Modifiers, Palette, Session, SessionEvent, SessionOptions};
use std::{collections::HashMap, path::PathBuf, rc::Rc};

/// Height of the top band: the traffic lights, the toggle and the title or tabs.
pub const TOOLBAR_HEIGHT: Pixels = px(44.);

pub(crate) const SIDEBAR_WIDTH: Pixels = px(240.);

/// How dark the veil over an inactive window is (0 is clear, 1 is black).
pub(crate) const INACTIVE_VEIL: f32 = 0.35;

/// The key context key bindings use to reach a window's workspace.
pub const CONTEXT: &str = "Workspace";

/// One shell and its screen.
pub(crate) struct Pane {
  pub session: Rc<Session>,
  pub title: String,
  /// Fractional lines left over from smooth scrolling.
  pub scroll_remainder: f32,
}

/// A tab: an arrangement of panes and the one that has the keyboard.
pub(crate) struct Tab {
  pub root: Node,
  pub focused: PaneId,
}

pub struct Workspace {
  pub(crate) panes: HashMap<PaneId, Pane>,
  pub(crate) tabs: Vec<Tab>,
  pub(crate) active: usize,
  next_pane: PaneId,
  directory: Option<PathBuf>,
  events: futures::channel::mpsc::UnboundedSender<(PaneId, SessionEvent)>,
  pub(crate) error: Option<String>,
  pub(crate) history: History,
  /// What has been typed on the current command line, when it can be tracked reliably.
  line: String,
  /// Set when the line was edited in a way that cannot be followed (history recall, cursor moves).
  line_unreliable: bool,
  pub(crate) sidebar_open: bool,
  pub(crate) focus_handle: FocusHandle,
  _subscriptions: Vec<Subscription>,
}

impl Workspace {
  /// `directory` is where shells start; `None` is the home folder.
  pub fn new(directory: Option<PathBuf>, window: &mut Window, cx: &mut Context<Self>) -> Self {
    Theme::sync_system_appearance(Some(window), cx);
    let subscriptions = vec![
      cx.observe_window_appearance(window, |_, window, cx| {
        Theme::sync_system_appearance(Some(window), cx);
        window.refresh();
      }),
      // The dimming of inactive windows follows activation.
      cx.observe_window_activation(window, |_, _, cx| cx.notify()),
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
      directory,
      events,
      error: None,
      history: History::load(&History::default_path()),
      line: String::new(),
      line_unreliable: false,
      sidebar_open: true,
      focus_handle,
      _subscriptions: subscriptions,
    };
    workspace.new_tab();
    workspace
  }

  fn handle_events(&mut self, events: Vec<(PaneId, SessionEvent)>, window: &mut Window, cx: &mut Context<Self>) {
    for (id, event) in events {
      match event {
        SessionEvent::Wakeup => {}
        SessionEvent::Title(title) => {
          if let (Some(pane), false) = (self.panes.get_mut(&id), title.is_empty()) {
            pane.title = title;
          }
        }
        // Leaving the shell closes its pane, like iTerm2.
        SessionEvent::Exit => self.close_pane(id, window),
      }
    }
    cx.notify();
  }

  // Panes, tabs and splits

  fn spawn_pane(&mut self) -> Option<PaneId> {
    let id = self.next_pane;
    let sender = self.events.clone();
    let options = SessionOptions { program: None, working_directory: self.directory.clone().or_else(|| std::env::var_os("HOME").map(Into::into)) };
    match Session::spawn(options, 80, 24, move |event| {
      let _ = sender.unbounded_send((id, event));
    }) {
      Ok(session) => {
        self.next_pane += 1;
        let title =
          std::env::var("SHELL").ok().and_then(|path| path.rsplit('/').next().map(|name| format!("-{name}"))).unwrap_or_else(|| "mTerm".into());
        self.panes.insert(id, Pane { session: Rc::new(session), title, scroll_remainder: 0. });
        Some(id)
      }
      Err(error) => {
        self.error = Some(format!("Could not start the shell: {error}"));
        None
      }
    }
  }

  pub(crate) fn new_tab(&mut self) {
    if let Some(id) = self.spawn_pane() {
      self.tabs.push(Tab { root: Node::Leaf(id), focused: id });
      self.active = self.tabs.len() - 1;
    }
  }

  pub(crate) fn split(&mut self, split: Split) {
    let Some(focused) = self.tabs.get(self.active).map(|tab| tab.focused) else { return };
    if let Some(id) = self.spawn_pane() {
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
        self.tabs.insert(index, Tab { root, focused });
      } else if self.active >= index {
        self.active = self.active.saturating_sub(1);
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
    if let Some(tab) = self.tabs.get_mut(self.active) {
      if tab.root.leaves().contains(&id) {
        tab.focused = id;
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
    let count = self.tabs.len() as isize;
    if count > 0 {
      self.select_tab((self.active as isize + step).rem_euclid(count) as usize, window, cx);
    }
  }

  pub(crate) fn focused_pane_id(&self) -> Option<PaneId> {
    self.tabs.get(self.active).map(|tab| tab.focused)
  }

  pub(crate) fn focused_pane(&self) -> Option<&Pane> {
    self.focused_pane_id().and_then(|id| self.panes.get(&id))
  }

  pub(crate) fn title(&self) -> String {
    self.focused_pane().map(|pane| pane.title.clone()).unwrap_or_else(|| "mTerm".into())
  }

  // Input and history

  fn write(&self, bytes: Vec<u8>) {
    if let Some(pane) = self.focused_pane() {
      pane.session.write(bytes);
    }
  }

  fn on_key_down(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
    let Some(pane) = self.focused_pane() else { return };
    let keystroke = &event.keystroke;
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
    cx.stop_propagation();
    cx.notify();
  }

  /// Follows what is typed so a finished line can be added to the history.
  fn track_input(&mut self, key: &str, text: Option<&str>, modifiers: Modifiers) {
    match key {
      "enter" => {
        let line = std::mem::take(&mut self.line);
        if !self.line_unreliable && !line.trim().is_empty() {
          // Other windows share the file: merge with what they saved.
          self.history = History::load(&History::default_path());
          if self.history.push(&line) {
            self.save_history();
          }
        }
        self.line_unreliable = false;
      }
      "backspace" if modifiers.alt || modifiers.command => self.line_unreliable = true,
      "backspace" => {
        self.line.pop();
      }
      "up" | "down" | "left" | "right" | "home" | "end" | "tab" | "delete" | "escape" => self.line_unreliable = true,
      "c" | "u" | "g" if modifiers.control => {
        self.line.clear();
        self.line_unreliable = false;
      }
      _ if modifiers.control || modifiers.command => self.line_unreliable = true,
      _ => match text {
        Some(text) => self.line.push_str(text),
        None if key == "space" => self.line.push(' '),
        None => {}
      },
    }
  }

  fn save_history(&self) {
    if let Err(error) = self.history.save(&History::default_path()) {
      eprintln!("Could not save the history: {error}");
    }
  }

  /// Types text at the prompt without running it.
  pub(crate) fn insert_text(&mut self, text: &str, window: &mut Window, cx: &mut Context<Self>) {
    if let Some(pane) = self.focused_pane() {
      pane.session.write(paste_bytes(text, pane.session.modes().bracketed_paste));
    }
    self.line.push_str(text);
    self.focus_handle.focus(window, cx);
    cx.notify();
  }

  pub(crate) fn clear_history(&mut self, cx: &mut Context<Self>) {
    self.history.clear();
    self.save_history();
    cx.notify();
  }

  fn paste(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
      if text.contains('\n') {
        self.line_unreliable = true;
      }
      self.insert_text(&text, window, cx);
    }
  }

  pub(crate) fn palette(cx: &App) -> Palette {
    if cx.theme().is_dark() {
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
    let inactive = !window.is_window_active();
    h_flex()
      .id("workspace")
      .relative()
      .key_context(CONTEXT)
      .track_focus(&self.focus_handle)
      .on_key_down(cx.listener(Self::on_key_down))
      .on_action(cx.listener(|this, _: &ToggleSidebar, window, cx| {
        this.sidebar_open = !this.sidebar_open;
        this.focus_handle.focus(window, cx);
        cx.notify();
      }))
      .on_action(cx.listener(|this, _: &Paste, window, cx| this.paste(window, cx)))
      .on_action(cx.listener(|this, _: &ClearScreen, _, _| this.write(vec![0x0c])))
      .on_action(cx.listener(|this, _: &ClearHistory, _, cx| this.clear_history(cx)))
      .on_action(cx.listener(|this, _: &NewTab, window, cx| {
        this.new_tab();
        this.focus_handle.focus(window, cx);
        cx.notify();
      }))
      .on_action(cx.listener(|this, _: &SplitVertically, _, cx| {
        this.split(Split::Vertically);
        cx.notify();
      }))
      .on_action(cx.listener(|this, _: &SplitHorizontally, _, cx| {
        this.split(Split::Horizontally);
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
      .when(self.sidebar_open, |this| this.child(self.render_history(cx)))
      .child(page)
      // A soft black veil over every window that is not the one in use.
      .when(inactive, |this| this.child(div().absolute().top_0().left_0().size_full().bg(black().opacity(INACTIVE_VEIL))))
  }
}
