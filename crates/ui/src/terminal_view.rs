use crate::{
  history::sidebar_toggle,
  layout::{Node, PaneId},
  workspace::Workspace,
  ClearScreen, ClosePane, NewTab, NewWindow, Paste, SplitHorizontally, SplitVertically,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
  component::{h_flex, menu::ContextMenuExt as _, v_flex, ActiveTheme, IconName},
  *,
};
use mterm_terminal::{Palette, Rgb, Snapshot};

const FONT_FAMILY: &str = "Menlo";
const FONT_SIZE: Pixels = px(13.);
const LINE_HEIGHT: Pixels = px(17.);
const PADDING: Pixels = px(8.);

fn color(rgb: Rgb) -> Hsla {
  gpui_kit::rgb(rgb.hex()).into()
}

/// What every pane in a frame shares.
struct Metrics {
  font: Font,
  cell_width: Pixels,
  palette: Palette,
}

impl Workspace {
  /// The active tab: its panes, or the reason there are none.
  pub(crate) fn render_tab_content(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
    let palette = Self::palette(cx);
    let Some(tab) = self.tabs.get(self.active) else {
      let message = self.error.clone().unwrap_or_else(|| "No shell is running.".into());
      return div().flex_1().p(PADDING).bg(color(palette.background)).text_color(color(palette.foreground)).child(message).into_any_element();
    };
    let (root, focused) = (tab.root.clone(), tab.focused);
    let font = font(FONT_FAMILY);
    let font_id = window.text_system().resolve_font(&font);
    let cell_width = window.text_system().advance(font_id, FONT_SIZE, 'm').map(|size| size.width).unwrap_or(px(7.8));
    let metrics = Metrics { font, cell_width, palette };
    let split = root.leaves().len() > 1;
    let toggle =
      (!self.sidebar_open).then(|| div().absolute().top(px(6.)).right(px(8.)).child(sidebar_toggle(IconName::PanelLeftOpen, "Show history (⌘B)")));
    div()
      .flex_1()
      .min_h_0()
      .relative()
      .child(self.render_node(&root, focused, split, &metrics, cx))
      // With the sidebar closed, the toggle floats in the corner of the page.
      .children(toggle)
      .into_any_element()
  }

  fn render_node(&self, node: &Node, focused: PaneId, split: bool, metrics: &Metrics, cx: &mut Context<Self>) -> AnyElement {
    match node {
      Node::Leaf(id) => self.render_pane(*id, *id == focused, split, metrics, cx),
      Node::Split { split: direction, first, second } => {
        let divider = cx.theme().border;
        let side_by_side = *direction == crate::layout::Split::Vertically;
        let first = div().flex_1().min_w_0().min_h_0().child(self.render_node(first, focused, split, metrics, cx));
        let second = div().flex_1().min_w_0().min_h_0().child(self.render_node(second, focused, split, metrics, cx));
        let line = if side_by_side { div().w(px(1.)).h_full().bg(divider) } else { div().h(px(1.)).w_full().bg(divider) };
        if side_by_side { h_flex() } else { v_flex() }.size_full().child(first).child(line).child(second).into_any_element()
      }
    }
  }

  fn render_pane(&self, id: PaneId, focused: bool, split: bool, metrics: &Metrics, cx: &mut Context<Self>) -> AnyElement {
    let Some(pane) = self.panes.get(&id) else { return div().into_any_element() };
    let snapshot = pane.session.snapshot(&metrics.palette);
    let session = pane.session.clone();
    let cell_width = f32::from(metrics.cell_width);
    let background = color(metrics.palette.background);
    let action_context = self.focus_handle.clone();

    div()
      .id(("pane", id))
      .relative()
      .size_full()
      .overflow_hidden()
      .bg(background)
      .when(split && !focused, |this| this.opacity(0.7))
      .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, window, cx| this.focus_pane(id, window, cx)))
      .on_mouse_down(MouseButton::Right, cx.listener(move |this, _, window, cx| this.focus_pane(id, window, cx)))
      .on_scroll(cx.listener(move |this, event: &ScrollEvent, _, cx| {
        let Some(pane) = this.panes.get_mut(&id) else { return };
        pane.scroll_remainder += f32::from(event.delta.pixel_delta(LINE_HEIGHT).y) / f32::from(LINE_HEIGHT);
        let lines = pane.scroll_remainder.trunc();
        if lines != 0. {
          pane.scroll_remainder -= lines;
          pane.session.scroll(lines as i32);
          cx.notify();
        }
      }))
      // Fits the PTY to the space this pane was given.
      .child(
        canvas(
          move |bounds, window, _| {
            let columns = ((f32::from(bounds.size.width) - 2. * f32::from(PADDING)) / cell_width).floor().max(2.) as usize;
            let lines = ((f32::from(bounds.size.height) - 2. * f32::from(PADDING)) / f32::from(LINE_HEIGHT)).floor().max(1.) as usize;
            if session.resize(columns, lines) {
              window.refresh();
            }
          },
          |_, _, _, _| {},
        )
        .absolute()
        .size_full(),
      )
      .child(
        div()
          .size_full()
          .p(PADDING)
          .text_size(FONT_SIZE)
          .font_family(FONT_FAMILY)
          .text_color(color(metrics.palette.foreground))
          .child(rows(&snapshot, &metrics.font)),
      )
      .context_menu(move |menu, _, _| {
        menu
          .action_context(action_context.clone())
          .menu("New Window", Box::new(NewWindow))
          .menu("New Tab", Box::new(NewTab))
          .separator()
          .menu("Split Pane Vertically", Box::new(SplitVertically))
          .menu("Split Pane Horizontally", Box::new(SplitHorizontally))
          .separator()
          .menu("Paste", Box::new(Paste))
          .menu("Clear Buffer", Box::new(ClearScreen))
          .separator()
          .menu("Close", Box::new(ClosePane))
      })
      .into_any_element()
  }
}

fn rows(snapshot: &Snapshot, base: &Font) -> impl IntoElement {
  let bold = Font { weight: FontWeight::BOLD, ..base.clone() };
  let italic = Font { style: FontStyle::Italic, ..base.clone() };
  let bold_italic = Font { weight: FontWeight::BOLD, style: FontStyle::Italic, ..base.clone() };
  let lines = snapshot.lines.iter().enumerate().map(|(index, spans)| {
    let mut text = String::new();
    let mut runs = Vec::with_capacity(spans.len());
    for span in spans {
      text.push_str(&span.text);
      let foreground = color(span.foreground);
      runs.push(TextRun {
        len: span.text.len(),
        font: match (span.bold, span.italic) {
          (true, true) => bold_italic.clone(),
          (true, false) => bold.clone(),
          (false, true) => italic.clone(),
          (false, false) => base.clone(),
        },
        color: foreground,
        background_color: span.background.map(color),
        underline: span.underline.then(|| UnderlineStyle { thickness: px(1.), color: Some(foreground), wavy: false }),
        strikethrough: span.strikeout.then(|| StrikethroughStyle { thickness: px(1.), color: Some(foreground) }),
      });
    }
    div()
      .id(("line", index))
      .h(LINE_HEIGHT)
      .w_full()
      .flex_shrink_0()
      .overflow_hidden()
      .whitespace_nowrap()
      .child(StyledText::new(text).with_runs(runs))
  });
  div().w_full().children(lines)
}
