use crate::{
  history::{icon_button, sidebar_toggle},
  workspace::Workspace,
  NewTab,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
  component::{h_flex, ActiveTheme, Icon, IconName, Sizable},
  *,
};

/// Height of the tab strip.
pub(crate) const TAB_BAR_HEIGHT: Pixels = px(36.);

impl Workspace {
  /// The strip of tabs, shown from the second tab on.
  pub(crate) fn render_tab_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
    let theme = cx.theme().clone();
    let tabs: Vec<_> = self
      .tabs
      .iter()
      .enumerate()
      .map(|(index, tab)| {
        let active = index == self.active;
        let title = self.panes.get(&tab.focused).map(|pane| pane.title.clone()).unwrap_or_default();
        h_flex()
          .id(("tab", index))
          .flex_1()
          .min_w_0()
          .h(px(28.))
          .pl_3()
          .pr_1()
          .gap_1()
          .items_center()
          .rounded_md()
          .cursor_pointer()
          .text_size(px(12.))
          .when(active, |this| this.bg(theme.background).text_color(theme.foreground))
          .when(!active, |this| this.text_color(theme.muted_foreground).hover(|style| style.bg(theme.background.opacity(0.5))))
          .on_click(cx.listener(move |this, _, window, cx| this.select_tab(index, window, cx)))
          .child(div().flex_1().min_w_0().overflow_hidden().whitespace_nowrap().text_ellipsis().child(title))
          .child(
            div()
              .id(("tab-close", index))
              .size(px(20.))
              .flex()
              .items_center()
              .justify_center()
              .rounded_sm()
              .hover(|style| style.bg(theme.muted_foreground.opacity(0.2)))
              .on_click(cx.listener(move |this, _, window, cx| {
                this.close_tab(index, window);
                cx.notify();
              }))
              .child(Icon::new(IconName::Close).small()),
          )
      })
      .collect();
    h_flex()
      .w_full()
      .h(TAB_BAR_HEIGHT)
      .flex_shrink_0()
      .px_2()
      .gap_1()
      .items_center()
      .when(!self.sidebar_open, |this| this.child(sidebar_toggle(IconName::PanelLeftOpen, "Show history (⌘B)")))
      .bg(theme.secondary)
      .border_b_1()
      .border_color(theme.border)
      .children(tabs)
      .child(icon_button("new-tab", IconName::Plus, "New tab (⌘T)", |_, window, cx| window.dispatch_action(Box::new(NewTab), cx)))
  }
}
