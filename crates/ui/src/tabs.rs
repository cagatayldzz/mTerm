use crate::{
  components::{close_button, icon_button, truncated},
  workspace::{Workspace, TOOLBAR_HEIGHT},
  NewTab, OpenSettings,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
  component::{
    h_flex,
    menu::{ContextMenuExt as _, PopupMenuItem},
    tab::{Tab, TabBar},
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

    let middle = if self.tabs.len() > 1 {
      self.render_tab_bar(cx).into_any_element()
    } else {
      truncated()
        .text_center()
        .text_sm()
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
        .child(icon_button("open-settings", IconName::Settings, "Settings (⌘,)", |_, window, cx| {
          window.dispatch_action(Box::new(OpenSettings), cx)
        })),
    )
  }

  /// The tabs of this window as a GPUI Kit tab bar, with the button that opens another.
  fn render_tab_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
    let tabs: Vec<Tab> = self
      .tabs
      .iter()
      .enumerate()
      .map(|(index, tab)| {
        let pinned = tab.pinned;
        let workspace = cx.weak_entity();
        // Everything sits inside the tab's own surface, so the selected tab's highlight holds the
        // icon, the title and the button together.
        let content = h_flex()
          .gap_1()
          .items_center()
          .when(pinned, |this| this.child(Icon::new(gpui_kit_assets::IconName::Pin).small()))
          .child(div().w_40().min_w_0().overflow_hidden().whitespace_nowrap().text_ellipsis().child(self.tab_title(tab)))
          .when(!pinned, |this| {
            this.child(close_button(
              ("tab-close", index),
              cx.listener(move |this, _, window, cx| {
                this.close_tab(index, window);
                cx.notify();
              }),
            ))
          });
        let item = Tab::new();
        // The menu opens from anywhere on the tab except its button, which sits above this layer.
        item
          .child(div().id(("tab-menu", index)).absolute().size_full().context_menu(move |menu, _, _| {
            let item = |label: &'static str, action: fn(&mut Workspace, usize, &mut Window, &mut Context<Workspace>)| {
              let workspace = workspace.clone();
              PopupMenuItem::new(label).on_click(move |_, window, cx| {
                let _ = workspace.update(cx, |this, cx| {
                  action(this, index, window, cx);
                  this.focus_handle.focus(window, cx);
                  cx.notify();
                });
              })
            };
            menu
              .item(item("New Tab to the Right", |this, index, _, cx| this.new_tab_after(index, cx)))
              .separator()
              .item(item("Rename", |this, index, window, cx| this.rename_tab(index, window, cx)))
              .item(item("Reload", |this, index, _, cx| this.reload_tab(index, cx)))
              .item(item("Duplicate", |this, index, _, cx| this.duplicate_tab(index, cx)))
              .item(item(if pinned { "Unpin" } else { "Pin" }, |this, index, _, _| this.toggle_tab_pin(index)))
              .separator()
              .item(item("Close", |this, index, window, _| this.close_tab(index, window)))
              .item(item("Close Other Tabs", |this, index, window, _| this.close_other_tabs(index, window)))
          }))
          .child(content)
      })
      .collect();
    let workspace = cx.weak_entity();
    h_flex()
      .flex_1()
      .min_w_0()
      .gap_1()
      .items_center()
      .child(
        TabBar::new("tabs")
          .segmented()
          .min_w_0()
          .selected_index(self.active)
          .on_click(move |index, window, cx| {
            let _ = workspace.update(cx, |this, cx| this.select_tab(*index, window, cx));
          })
          .children(tabs),
      )
      .child(icon_button("new-tab", IconName::Plus, "New tab (⌘T)", |_, window, cx| window.dispatch_action(Box::new(NewTab), cx)))
  }
}

/// The hairline under the top band: the text color at low strength, so it shows on any background.
pub(crate) fn band_line(theme: &ThemeColor) -> Hsla {
  theme.foreground.opacity(0.14)
}
