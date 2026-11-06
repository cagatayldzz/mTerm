use crate::{
  history::{icon_button, sidebar_toggle},
  workspace::{Workspace, TOOLBAR_HEIGHT},
  NewTab,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
  component::{h_flex, ActiveTheme, Icon, IconName, Sizable, TitleBar},
  *,
};

/// Where the band's content starts when the sidebar is open (the sidebar already covers the traffic lights).
const BAND_INSET: Pixels = px(12.);
/// Where it starts when the sidebar is closed: past the traffic lights.
const BAND_INSET_CLEARING_LIGHTS: Pixels = px(88.);

impl Workspace {
  /// The single top band over the page, level with the traffic lights. It holds the sidebar toggle,
  /// the title (or the tabs, once there is more than one) and shares the terminal's background so
  /// the two read as one surface.
  pub(crate) fn render_band(&self, cx: &mut Context<Self>) -> impl IntoElement {
    let theme = cx.theme().clone();
    let background = rgb(Self::palette(cx).background.hex());
    let open = self.sidebar_open;
    let icon = if open { IconName::PanelLeftClose } else { IconName::PanelLeftOpen };
    let tooltip = if open { "Hide history (⌘B)" } else { "Show history (⌘B)" };

    let middle = if self.tabs.len() > 1 {
      h_flex()
        .flex_1()
        .min_w_0()
        .items_center()
        .gap_1()
        .children(self.render_tabs(cx))
        .child(icon_button("new-tab", IconName::Plus, "New tab (⌘T)", |_, window, cx| window.dispatch_action(Box::new(NewTab), cx)))
        .into_any_element()
    } else {
      div()
        .flex_1()
        .min_w_0()
        .overflow_hidden()
        .whitespace_nowrap()
        .text_ellipsis()
        .text_center()
        .text_size(px(13.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(theme.foreground.opacity(0.6))
        .child(self.title())
        .into_any_element()
    };

    TitleBar::new().h(TOOLBAR_HEIGHT).pl(if open { BAND_INSET } else { BAND_INSET_CLEARING_LIGHTS }).pr_3().bg(background).border_b_0().child(
      h_flex()
        .w_full()
        .h_full()
        .items_center()
        .gap_2()
        .child(sidebar_toggle(icon, tooltip))
        .child(middle)
        // Balances the toggle so a lone title stays centered.
        .when(self.tabs.len() <= 1, |this| this.child(div().w(px(24.)))),
    )
  }

  fn render_tabs(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
    let theme = cx.theme().clone();
    self
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
          .max_w(px(200.))
          .h(px(28.))
          .pl_3()
          .pr_1()
          .gap_1()
          .items_center()
          .rounded_md()
          .cursor_pointer()
          .text_size(px(12.))
          .when(active, |this| this.bg(theme.secondary).text_color(theme.foreground))
          .when(!active, |this| this.text_color(theme.muted_foreground).hover(|style| style.bg(theme.secondary.opacity(0.5))))
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
          .into_any_element()
      })
      .collect()
  }
}
