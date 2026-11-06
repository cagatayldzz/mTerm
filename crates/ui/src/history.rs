use gpui_kit::{
  component::{Icon, IconName, Sizable},
  *,
};

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
