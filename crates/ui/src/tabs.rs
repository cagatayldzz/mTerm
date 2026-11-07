use crate::{
  history::icon_button,
  workspace::{Workspace, TOOLBAR_HEIGHT},
  NewTab, OpenSettings,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
  component::{
    button::{Button, ButtonVariants},
    h_flex,
    menu::{DropdownMenu as _, PopupMenuItem},
    ActiveTheme, Icon, IconName, Sizable, ThemeColor, TitleBar,
  },
  *,
};

/// Keep the band content clear of the traffic lights.
const BAND_INSET_CLEARING_LIGHTS: Pixels = px(88.);

impl Workspace {
  /// The top band holds the title, tabs and window controls.
  pub(crate) fn render_band(&self, cx: &mut Context<Self>) -> impl IntoElement {
    let theme = cx.theme().clone();
    let background = rgb(Self::palette(cx).background.hex());
    let sessions = self.saved_sessions.clone();
    let current = self.tabs.get(self.active).map(|tab| tab.saved.clone());
    let workspace = cx.weak_entity();

    let middle = if self.active_tab_indices().len() > 1 {
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

    TitleBar::new().h(TOOLBAR_HEIGHT).pl(BAND_INSET_CLEARING_LIGHTS).pr_3().bg(background).border_b_1().border_color(band_line(&theme)).child(
      h_flex()
        .id("title-bar-content")
        .w_full()
        .h_full()
        .on_mouse_down(MouseButton::Left, |event: &MouseDownEvent, window, cx| {
          if event.click_count == 2 {
            window.zoom_window();
            cx.stop_propagation();
          }
        })
        .items_center()
        .gap_2()
        .child(middle)
        .child(
          Button::new("sessions-menu")
            .text()
            .w(px(24.))
            .h(px(30.))
            .text_color(theme.foreground)
            .tooltip("Sessions")
            .child(
              div()
                .id("sessions-icon")
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .hover(|style| style.opacity(0.6))
                .child(Icon::new(gpui_kit_assets::IconName::Layers).with_size(gpui_kit::component::Size::Size(px(20.)))),
            )
            .dropdown_menu(move |mut menu, _, _| {
              let new_workspace = workspace.clone();
              menu = menu
                .item(PopupMenuItem::new("New session").on_click(move |_, window, cx| {
                  let _ = new_workspace.update(cx, |this, cx| {
                    this.new_session(cx);
                    this.focus_handle.focus(window, cx);
                    cx.notify();
                  });
                }))
                .separator();
              for saved in &sessions {
                let id = saved.id.clone();
                let target = workspace.clone();
                let label = if current.as_ref().is_some_and(|active| active.id == id) { format!("✓ {}", saved.name) } else { saved.name.clone() };
                menu = menu.item(PopupMenuItem::new(label).on_click(move |_, window, cx| {
                  let _ = target.update(cx, |this, cx| this.select_session(&id, window, cx));
                }));
              }
              if let Some(saved) = &current {
                let rename_id = saved.id.clone();
                let delete_id = saved.id.clone();
                let pin_id = saved.id.clone();
                let rename = workspace.clone();
                let delete = workspace.clone();
                let pin = workspace.clone();
                menu = menu
                  .separator()
                  .item(PopupMenuItem::new("Rename session").on_click(move |_, window, cx| {
                    let _ = rename.update(cx, |this, cx| this.rename_session(rename_id.clone(), window, cx));
                  }))
                  .item(PopupMenuItem::new(if saved.pinned { "Unpin session" } else { "Pin session" }).on_click(move |_, window, cx| {
                    let _ = pin.update(cx, |this, cx| this.toggle_session_pin(&pin_id, window, cx));
                  }))
                  .item(PopupMenuItem::new("Delete session").on_click(move |_, window, cx| {
                    let _ = delete.update(cx, |this, cx| this.confirm_delete_session(delete_id.clone(), window, cx));
                  }));
              }
              menu
            }),
        )
        .child(icon_button("open-settings", IconName::Settings, "Settings (⌘,)", |_, window, cx| {
          window.dispatch_action(Box::new(OpenSettings), cx)
        })),
    )
  }

  fn render_tabs(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
    let theme = cx.theme().clone();
    let indices = self.active_tab_indices();
    self
      .tabs
      .iter()
      .enumerate()
      .filter(|(index, _)| indices.contains(index))
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

/// The hairline under the top band: the text color at low strength, so it shows on any background.
pub(crate) fn band_line(theme: &ThemeColor) -> Hsla {
  theme.foreground.opacity(0.14)
}
