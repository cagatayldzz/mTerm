use crate::{
  components::icon_button,
  i18n::{t, tf},
  layout::PaneId,
  workspace::{Workspace, TOOLBAR_HEIGHT},
};
use gpui_kit::{
  component::{
    button::{Button, ButtonVariants},
    h_flex,
    input::{Input, InputEvent, InputState},
    ActiveTheme, IconName, Selectable as _, Sizable,
  },
  *,
};
use mterm_terminal::{Highlight, Match};

/// The find bar and what it found in the focused pane.
pub(crate) struct Find {
  input: Entity<InputState>,
  case_sensitive: bool,
  matches: Vec<Match>,
  current: Option<usize>,
  _subscription: Subscription,
}

impl Workspace {
  /// Shows the find bar, or selects its text when it is already open.
  pub(crate) fn open_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    self.settings = None;
    if let Some(find) = &self.find {
      let input = find.input.clone();
      input.update(cx, |input, cx| input.select_all(window, cx));
      input.read(cx).focus_handle(cx).focus(window, cx);
      cx.notify();
      return;
    }
    let input = cx.new(|cx| InputState::new(window, cx).placeholder(t("Find")));
    let subscription = cx.subscribe_in(&input, window, |this, _, event: &InputEvent, window, cx| match event {
      InputEvent::Change => this.refresh_find(window, cx),
      // Return goes toward older output, ⇧Return back toward newer.
      InputEvent::PressEnter { shift, .. } => this.step_find(!*shift, cx),
      _ => {}
    });
    input.read(cx).focus_handle(cx).focus(window, cx);
    self.find = Some(Find { input, case_sensitive: false, matches: Vec::new(), current: None, _subscription: subscription });
    cx.notify();
  }

  pub(crate) fn close_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    if self.find.take().is_some() {
      self.focus_handle.focus(window, cx);
      cx.notify();
    }
  }

  /// Searches again after the text or the options changed, and goes to the newest hit.
  fn refresh_find(&mut self, _: &mut Window, cx: &mut Context<Self>) {
    let Some((query, case_sensitive)) = self.find.as_ref().map(|find| (find.input.read(cx).value().to_string(), find.case_sensitive)) else {
      return;
    };
    let matches = self.focused_pane().map(|pane| pane.session.find(&query, case_sensitive)).unwrap_or_default();
    let current = matches.len().checked_sub(1);
    if let (Some(pane), Some(hit)) = (self.focused_pane(), current.map(|index| matches[index])) {
      pane.session.reveal(hit);
    }
    if let Some(find) = &mut self.find {
      find.matches = matches;
      find.current = current;
    }
    cx.notify();
  }

  /// Moves to the next hit toward older output (`older`) or newer output, wrapping around at the ends.
  pub(crate) fn step_find(&mut self, older: bool, cx: &mut Context<Self>) {
    let Some((query, case_sensitive, previous)) = self
      .find
      .as_ref()
      .map(|find| (find.input.read(cx).value().to_string(), find.case_sensitive, find.current.and_then(|index| find.matches.get(index)).copied()))
    else {
      return;
    };
    // The output moved on since the last search: look again before stepping.
    let matches = self.focused_pane().map(|pane| pane.session.find(&query, case_sensitive)).unwrap_or_default();
    let count = matches.len();
    let at = previous.and_then(|hit| matches.iter().position(|other| *other == hit));
    let current = match (at, count) {
      (_, 0) => None,
      (Some(index), _) if older => Some((index + count - 1) % count),
      (Some(index), _) => Some((index + 1) % count),
      (None, _) if older => Some(count - 1),
      (None, _) => Some(0),
    };
    if let (Some(pane), Some(hit)) = (self.focused_pane(), current.map(|index| matches[index])) {
      pane.session.reveal(hit);
    }
    if let Some(find) = &mut self.find {
      find.matches = matches;
      find.current = current;
    }
    cx.notify();
  }

  /// The hits to paint on the focused pane's screen.
  pub(crate) fn find_highlights(&self, pane: PaneId, cx: &App) -> Vec<Highlight> {
    let Some(find) = self.find.as_ref().filter(|_| self.focused_pane_id() == Some(pane)) else { return Vec::new() };
    let query = find.input.read(cx).value();
    if query.is_empty() {
      return Vec::new();
    }
    let current = find.current.and_then(|index| find.matches.get(index)).copied();
    self.panes.get(&pane).map(|pane| pane.session.highlights(&query, find.case_sensitive, current)).unwrap_or_default()
  }

  pub(crate) fn render_find_bar(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
    let find = self.find.as_ref()?;
    let theme = cx.theme().clone();
    let has_query = !find.input.read(cx).value().is_empty();
    let status = match (find.current, find.matches.len()) {
      _ if !has_query => String::new(),
      (Some(index), count) => tf("{index} of {count}", &[("index", &(index + 1).to_string()), ("count", &count.to_string())]),
      (None, _) => t("No results").into(),
    };
    Some(
      h_flex()
        .absolute()
        .top(TOOLBAR_HEIGHT)
        .mt_2()
        .right_5()
        .gap_1()
        .p_1()
        .items_center()
        .rounded_lg()
        .border_1()
        .border_color(theme.border)
        .bg(theme.popover)
        .shadow_md()
        .child(div().w_48().child(Input::new(&find.input)))
        .child(div().w_16().text_xs().text_color(theme.muted_foreground).child(status))
        .child(Button::new("find-case").label("Aa").ghost().xsmall().selected(find.case_sensitive).tooltip(t("Match case")).on_click(cx.listener(
          |this, _, window, cx| {
            if let Some(find) = &mut this.find {
              find.case_sensitive = !find.case_sensitive;
            }
            this.refresh_find(window, cx);
          },
        )))
        .child(
          Button::new("find-older")
            .label("↑")
            .ghost()
            .xsmall()
            .tooltip(t("Older match (⌘G)"))
            .on_click(cx.listener(|this, _, _, cx| this.step_find(true, cx))),
        )
        .child(
          Button::new("find-newer")
            .label("↓")
            .ghost()
            .xsmall()
            .tooltip(t("Newer match (⇧⌘G)"))
            .on_click(cx.listener(|this, _, _, cx| this.step_find(false, cx))),
        )
        .child(icon_button("find-close", IconName::Close, "Close (Esc)", cx.listener(|this, _, window, cx| this.close_find(window, cx)))),
    )
  }
}
