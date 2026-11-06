use crate::{
  workspace::{Workspace, HEADER_HEIGHT, SIDEBAR_WIDTH},
  ToggleSidebar,
};
use gpui_kit::{
  component::{h_flex, v_flex, ActiveTheme, Icon, IconName, Sizable},
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
          .h(HEADER_HEIGHT)
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
          .child(icon_button("clear-history", IconName::Delete, "Clear history", cx.listener(|this, _, _, cx| this.clear_history(cx)))),
      )
      .child(if empty {
        div().px_3().text_size(px(12.)).text_color(theme.muted_foreground).child("Commands you run will appear here.").into_any_element()
      } else {
        v_flex().id("history-list").flex_1().min_h_0().px_2().pb_2().gap(px(1.)).overflow_y_scroll().children(items).into_any_element()
      })
  }
}

/// A clickable icon, larger than the stock button icons so it reads at a glance.
pub(crate) fn icon_button(
  id: impl Into<ElementId>,
  icon: IconName,
  tooltip: impl Into<SharedString>,
  on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
  div()
    .id(id)
    .w(px(24.))
    .h(px(30.))
    .flex()
    .items_center()
    .justify_center()
    .rounded_md()
    .cursor_pointer()
    .hover(|style| style.opacity(0.6))
    .tooltip({
      let tooltip: SharedString = tooltip.into();
      move |window, cx| component::tooltip::Tooltip::new(tooltip.clone()).build(window, cx)
    })
    .on_click(on_click)
    .child(Icon::new(icon).with_size(component::Size::Size(px(20.))))
}

/// The sidebar toggle. It sits in the sidebar header when open and in the page when closed.
pub(crate) fn sidebar_toggle(icon: IconName, tooltip: &'static str) -> impl IntoElement {
  icon_button("toggle-sidebar", icon, tooltip, |_, window, cx| window.dispatch_action(Box::new(ToggleSidebar), cx))
}
