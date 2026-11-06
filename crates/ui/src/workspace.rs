use crate::{ClearHistory, ClearScreen, Paste, ToggleSidebar};
use futures::StreamExt as _;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
  component::{
    button::{Button, ButtonVariants},
    h_flex, v_flex, ActiveTheme, IconName, Sizable, Theme, TitleBar,
  },
  *,
};
use mterm_storage::History;
use mterm_terminal::{encode_key, paste_bytes, Modifiers, Palette, Session, SessionEvent, SessionOptions};

/// Height of the band that holds the traffic lights, the sidebar button and the title.
pub const TOOLBAR_HEIGHT: Pixels = px(44.);

pub(crate) const SIDEBAR_WIDTH: Pixels = px(240.);
const CONTEXT: &str = "Workspace";

pub struct Workspace {
  pub(crate) session: Result<Session, String>,
  pub(crate) history: History,
  /// What has been typed on the current command line, when it can be tracked reliably.
  line: String,
  /// Set when the line was edited in a way that cannot be followed (history recall, cursor moves).
  line_unreliable: bool,
  pub(crate) sidebar_open: bool,
  pub(crate) title: String,
  /// Fractional lines left over from smooth scrolling.
  pub(crate) scroll_remainder: f32,
  pub(crate) focus_handle: FocusHandle,
  _subscriptions: Vec<Subscription>,
}

impl Workspace {
  pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
    cx.bind_keys([
      KeyBinding::new("cmd-b", ToggleSidebar, Some(CONTEXT)),
      KeyBinding::new("cmd-v", Paste, Some(CONTEXT)),
      KeyBinding::new("cmd-k", ClearScreen, Some(CONTEXT)),
    ]);
    Theme::sync_system_appearance(Some(window), cx);
    let subscriptions = vec![cx.observe_window_appearance(window, |_, window, cx| {
      Theme::sync_system_appearance(Some(window), cx);
      window.refresh();
    })];

    let (sender, mut receiver) = futures::channel::mpsc::unbounded();
    let options = SessionOptions { program: None, working_directory: std::env::var_os("HOME").map(Into::into) };
    let session = Session::spawn(options, 80, 24, move |event| {
      let _ = sender.unbounded_send(event);
    })
    .map_err(|error| format!("Could not start the shell: {error}"));
    cx.spawn(async move |this, cx| {
      while let Some(first) = receiver.next().await {
        // Output arrives in bursts: handle everything queued in one update.
        let mut events = vec![first];
        while let Ok(event) = receiver.try_recv() {
          events.push(event);
        }
        if this.update(cx, |this, cx| this.handle_events(events, cx)).is_err() {
          break;
        }
      }
    })
    .detach();

    let focus_handle = cx.focus_handle();
    focus_handle.focus(window, cx);
    let shell = std::env::var("SHELL").ok().and_then(|path| path.rsplit('/').next().map(|name| format!("-{name}"))).unwrap_or_else(|| "mTerm".into());
    Self {
      session,
      history: History::load(&History::default_path()),
      line: String::new(),
      line_unreliable: false,
      sidebar_open: true,
      title: shell,
      scroll_remainder: 0.,
      focus_handle,
      _subscriptions: subscriptions,
    }
  }

  fn handle_events(&mut self, events: Vec<SessionEvent>, cx: &mut Context<Self>) {
    for event in events {
      match event {
        SessionEvent::Wakeup => {}
        SessionEvent::Title(title) => {
          if !title.is_empty() {
            self.title = title;
          }
        }
        // Like iTerm2 with a single window, leaving the shell leaves the app.
        SessionEvent::Exit => {
          cx.quit();
          return;
        }
      }
    }
    cx.notify();
  }

  pub(crate) fn write(&self, bytes: Vec<u8>) {
    if let Ok(session) = &self.session {
      session.write(bytes);
    }
  }

  fn on_key_down(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
    let Ok(session) = &self.session else { return };
    let keystroke = &event.keystroke;
    let modifiers = Modifiers {
      control: keystroke.modifiers.control,
      alt: keystroke.modifiers.alt,
      shift: keystroke.modifiers.shift,
      command: keystroke.modifiers.platform,
    };
    let text = keystroke.key_char.as_deref();
    let Some(bytes) = encode_key(&keystroke.key, text, modifiers, session.modes().app_cursor) else { return };
    session.write(bytes);
    self.track_input(&keystroke.key, text, modifiers);
    cx.stop_propagation();
    cx.notify();
  }

  /// Follows what is typed so a finished line can be added to the history.
  fn track_input(&mut self, key: &str, text: Option<&str>, modifiers: Modifiers) {
    match key {
      "enter" => {
        let line = std::mem::take(&mut self.line);
        if !self.line_unreliable && self.history.push(&line) {
          self.save_history();
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

  /// Types a history entry at the prompt without running it.
  pub(crate) fn insert_text(&mut self, text: &str, window: &mut Window, cx: &mut Context<Self>) {
    if let Ok(session) = &self.session {
      session.write(paste_bytes(text, session.modes().bracketed_paste));
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

  fn toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
    let theme = cx.theme().clone();
    let icon = if self.sidebar_open { IconName::PanelLeftClose } else { IconName::PanelLeftOpen };
    TitleBar::new().h(TOOLBAR_HEIGHT).bg(theme.background).child(
      h_flex()
        .w_full()
        .h_full()
        .items_center()
        .pr_3()
        .gap_2()
        .child(
          Button::new("toggle-sidebar")
            .ghost()
            .small()
            .icon(icon)
            .tooltip("Toggle history (⌘B)")
            .on_click(|_, window, cx| window.dispatch_action(Box::new(ToggleSidebar), cx)),
        )
        .child(
          div()
            .flex_1()
            .min_w_0()
            .overflow_hidden()
            .whitespace_nowrap()
            .text_ellipsis()
            .text_center()
            .text_size(px(13.))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(theme.foreground.opacity(0.7))
            .child(self.title.clone()),
        )
        // Balances the toggle button so the title stays centered.
        .child(div().w(px(28.))),
    )
  }
}

impl Render for Workspace {
  fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let theme = cx.theme().clone();
    let terminal = self.render_terminal(window, cx);
    v_flex()
      .id("workspace")
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
      .size_full()
      .bg(theme.background)
      .text_color(theme.foreground)
      .child(self.toolbar(cx))
      .child(h_flex().flex_1().min_h_0().w_full().when(self.sidebar_open, |this| this.child(self.render_history(cx))).child(terminal))
  }
}
