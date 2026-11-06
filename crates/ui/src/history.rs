use crate::{
  workspace::{Workspace, SIDEBAR_WIDTH},
  ToggleSidebar,
};
use gpui_kit::{
  component::{
    button::{Button, ButtonVariants},
    h_flex, v_flex, ActiveTheme, IconName, Sizable,
  },
  *,
};

impl Workspace {
  /// The sidebar: commands entered so far, newest first. Clicking one types it at the prompt.
  pub(crate) fn render_history(&self, cx: &mut Context<Self>) -> impl IntoElement {
    let theme = cx.theme().clone();
    let items: Vec<_> = self
      .history
      .entries()
      .iter()
      .enumerate()
      .rev()
      .map(|(index, command)| {
        let text = command.clone();
        div()
          .id(("history", index))
          .px_2()
          .py(px(3.))
          .rounded_md()
          .cursor_pointer()
          .font_family("Menlo")
          .text_size(px(12.))
          .text_color(theme.sidebar_foreground)
          .overflow_hidden()
          .whitespace_nowrap()
          .text_ellipsis()
          .hover(|style| style.bg(theme.sidebar_accent))
          .tooltip({
            let text = command.clone();
            move |window, cx| component::tooltip::Tooltip::new(text.clone()).build(window, cx)
          })
          .child(command.clone())
          .on_click(cx.listener(move |this, _, window, cx| this.insert_text(&text, window, cx)))
      })
      .collect();
    let empty = items.is_empty();

    v_flex()
      .w(SIDEBAR_WIDTH)
      .flex_shrink_0()
      .h_full()
      .bg(theme.sidebar)
      .border_r_1()
      .border_color(theme.sidebar_border)
      .child(
        h_flex()
          .h(px(32.))
          .px_3()
          .items_center()
          .justify_between()
          .child(
            h_flex()
              .gap_2()
              .items_center()
              .child(sidebar_toggle(IconName::PanelLeftClose, "Hide history (⌘B)"))
              .child(div().text_size(px(12.)).font_weight(FontWeight::SEMIBOLD).text_color(theme.foreground).child("History")),
          )
          .child(
            Button::new("clear-history")
              .ghost()
              .xsmall()
              .icon(IconName::Delete)
              .tooltip("Clear history")
              .on_click(cx.listener(|this, _, _, cx| this.clear_history(cx))),
          ),
      )
      .child(if empty {
        div().px_3().text_size(px(12.)).text_color(theme.muted_foreground).child("Commands you run will appear here.").into_any_element()
      } else {
        v_flex().id("history-list").flex_1().min_h_0().px_2().pb_2().gap(px(1.)).overflow_y_scroll().children(items).into_any_element()
      })
  }
}

/// The sidebar toggle. It sits in the sidebar header when open and in the page when closed.
pub(crate) fn sidebar_toggle(icon: IconName, tooltip: &'static str) -> impl IntoElement {
  Button::new("toggle-sidebar")
    .ghost()
    .small()
    .icon(icon)
    .tooltip(tooltip)
    .on_click(|_, window, cx| window.dispatch_action(Box::new(ToggleSidebar), cx))
}
