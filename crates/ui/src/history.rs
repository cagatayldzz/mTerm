use crate::{
  tabs::band_line,
  workspace::{Workspace, SIDEBAR_WIDTH, TOOLBAR_HEIGHT},
  ToggleSidebar,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
  component::{
    h_flex,
    menu::{ContextMenuExt as _, PopupMenuItem},
    v_flex, ActiveTheme, Icon, IconName, Sizable, TitleBar,
  },
  *,
};

impl Workspace {
  /// Lists saved sessions and the active session's recorded commands.
  pub(crate) fn render_history(&self, cx: &mut Context<Self>) -> impl IntoElement {
    let theme = cx.theme().clone();
    let active = self.tabs.get(self.active);
    let active_id = active.map(|tab| tab.saved.id.as_str());
    let sessions: Vec<_> = self
      .saved_sessions
      .iter()
      .map(|saved| {
        let id = saved.id.clone();
        let menu_id = id.clone();
        let workspace = cx.weak_entity();
        let pinned = saved.pinned;
        let selected = active_id == Some(saved.id.as_str());
        let directory = saved.directory.display().to_string();
        v_flex()
          .id(SharedString::from(format!("session-{id}")))
          .px_2()
          .py_1()
          .rounded_md()
          .cursor_pointer()
          .min_w_0()
          .when(selected, |this| this.bg(theme.sidebar_accent))
          .hover(|style| style.bg(theme.sidebar_accent))
          .text_color(theme.sidebar_foreground)
          .child(
            h_flex()
              .items_center()
              .gap_1()
              .child(
                div()
                  .flex_1()
                  .min_w_0()
                  .text_size(px(12.))
                  .font_weight(FontWeight::SEMIBOLD)
                  .overflow_hidden()
                  .whitespace_nowrap()
                  .text_ellipsis()
                  .child(saved.name.clone()),
              )
              .when(pinned, |this| this.child(Icon::new(gpui_kit_assets::IconName::Pin).with_size(component::Size::Size(px(12.))))),
          )
          .child(div().text_size(px(11.)).text_color(theme.muted_foreground).overflow_hidden().whitespace_nowrap().text_ellipsis().child(directory))
          .on_click(cx.listener(move |this, _, window, cx| this.select_session(&id, window, cx)))
          .context_menu(move |menu, _, _| {
            let pin_id = menu_id.clone();
            let delete_id = menu_id.clone();
            let pin_workspace = workspace.clone();
            let delete_workspace = workspace.clone();
            menu
              .item(
                PopupMenuItem::new(if pinned { "Unpin session" } else { "Pin session" }).icon(Icon::new(gpui_kit_assets::IconName::Pin)).on_click(
                  move |_, window, cx| {
                    let _ = pin_workspace.update(cx, |this, cx| this.toggle_session_pin(&pin_id, window, cx));
                  },
                ),
              )
              .separator()
              .item(PopupMenuItem::new("Delete session").icon(Icon::new(gpui_kit_assets::IconName::Trash)).on_click(move |_, window, cx| {
                let id = delete_id.clone();
                let _ = delete_workspace.update(cx, |this, cx| this.confirm_delete_session(id, window, cx));
              }))
          })
          .into_any_element()
      })
      .collect();
    let commands: Vec<_> = active
      .into_iter()
      .flat_map(|tab| tab.history.entries().iter().rev())
      .enumerate()
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
          .child(command.clone())
          .on_click(cx.listener(move |this, _, window, cx| this.insert_text(&text, window, cx)))
          .into_any_element()
      })
      .collect();
    let has_commands = !commands.is_empty();
    v_flex()
      .w(SIDEBAR_WIDTH)
      .flex_shrink_0()
      .h_full()
      .bg(theme.sidebar)
      .border_r_1()
      .border_color(theme.sidebar_border)
      .child(TitleBar::new().h(TOOLBAR_HEIGHT).bg(transparent_black()).border_b_1().border_color(band_line(&theme)))
      .child(
        h_flex()
          .h(px(32.))
          .px_3()
          .items_center()
          .justify_between()
          .child(div().text_size(px(11.)).font_weight(FontWeight::SEMIBOLD).text_color(theme.muted_foreground).child("SESSIONS"))
          .child(icon_button(
            "new-session",
            IconName::Plus,
            "New session",
            cx.listener(|this, _, window, cx| {
              this.new_tab(cx);
              this.focus_handle.focus(window, cx);
              cx.notify();
            }),
          )),
      )
      .child(
        v_flex()
          .id("session-list")
          .flex_1()
          .min_h_0()
          .px_2()
          .pb_2()
          .gap(px(1.))
          .overflow_y_scroll()
          .children(sessions)
          .child(
            h_flex()
              .h(px(32.))
              .px_1()
              .items_center()
              .justify_between()
              .child(div().text_size(px(11.)).font_weight(FontWeight::SEMIBOLD).text_color(theme.muted_foreground).child("COMMAND HISTORY"))
              .when(has_commands, |this| {
                this.child(icon_button(
                  "clear-history",
                  IconName::Delete,
                  "Clear session history",
                  cx.listener(|this, _, window, cx| this.confirm_clear_history(window, cx)),
                ))
              }),
          )
          .children(commands),
      )
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

/// The sidebar toggle, always at the left of the header row.
pub(crate) fn sidebar_toggle(icon: IconName, tooltip: &'static str) -> impl IntoElement {
  icon_button("toggle-sidebar", icon, tooltip, |_, window, cx| window.dispatch_action(Box::new(ToggleSidebar), cx))
}
