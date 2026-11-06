use crate::{
  layout::{Node, PaneId, Split},
  workspace::{Workspace, SIDEBAR_WIDTH, TOOLBAR_HEIGHT},
  ClearScreen, ClosePane, NewTab, NewWindow, Paste, SplitHorizontally, SplitVertically,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
  component::{menu::ContextMenuExt as _, ActiveTheme},
  *,
};
use mterm_terminal::{Palette, Rgb, Snapshot};

const FONT_FAMILY: &str = "Menlo";
const FONT_SIZE: Pixels = px(13.);
const LINE_HEIGHT: Pixels = px(17.);
const PADDING: Pixels = px(12.);
/// How dark the veil over an unfocused split pane is.
const PANE_VEIL: f32 = 0.18;

#[derive(Clone)]
struct DividerDrag {
  path: Vec<bool>,
  split: Split,
  area: Bounds<Pixels>,
  sidebar: Pixels,
}

impl Render for DividerDrag {
  fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
    Empty
  }
}

struct Divider {
  bounds: Bounds<Pixels>,
  drag: DividerDrag,
}

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
    // Panes are placed at explicit rectangles: flex percentages do not resolve reliably for nested splits.
    let sidebar = if self.sidebar_open { SIDEBAR_WIDTH } else { px(0.) };
    let viewport = window.viewport_size();
    let area = Bounds::new(point(px(0.), px(0.)), size(viewport.width - sidebar, viewport.height - TOOLBAR_HEIGHT));
    let mut panes = Vec::new();
    let mut dividers = Vec::new();
    place(&root, area, &mut panes, &mut dividers, &mut Vec::new(), sidebar);
    let border = cx.theme().border;
    let panes: Vec<_> = panes.into_iter().map(|(id, bounds)| self.render_pane(id, bounds, id == focused, split, &metrics, cx)).collect();
    div()
      .flex_1()
      .min_h_0()
      .relative()
      .children(panes)
      .on_drag_move(cx.listener(|this, event: &DragMoveEvent<DividerDrag>, _, cx| {
        let drag = event.drag(cx);
        let (position, origin, extent) = match drag.split {
          Split::Vertically => (event.event.position.x - drag.sidebar, drag.area.origin.x, drag.area.size.width),
          Split::Horizontally => (event.event.position.y - TOOLBAR_HEIGHT, drag.area.origin.y, drag.area.size.height),
        };
        let fraction = f32::from(position - origin) / f32::from(extent - px(1.));
        let ratio = (fraction.clamp(0.1, 0.9) * 10000.) as u16;
        if let Some(tab) = this.tabs.get_mut(this.active) {
          if tab.root.set_ratio(&drag.path, ratio) {
            this.save_layout();
            cx.notify();
          }
        }
      }))
      .children(dividers.into_iter().enumerate().map(move |(index, divider)| {
        let bounds = divider.bounds;
        let vertical = divider.drag.split == Split::Vertically;
        div()
          .id(("divider", index))
          .absolute()
          .left(bounds.origin.x - if vertical { px(4.) } else { px(0.) })
          .top(bounds.origin.y - if vertical { px(0.) } else { px(4.) })
          .w(bounds.size.width + if vertical { px(8.) } else { px(0.) })
          .h(bounds.size.height + if vertical { px(0.) } else { px(8.) })
          .when(vertical, |this| this.cursor_col_resize())
          .when(!vertical, |this| this.cursor_row_resize())
          .on_drag(divider.drag, |drag, _, _, cx| cx.new(|_| drag.clone()))
          .child(
            div()
              .absolute()
              .left(if vertical { px(4.) } else { px(0.) })
              .top(if vertical { px(0.) } else { px(4.) })
              .w(bounds.size.width)
              .h(bounds.size.height)
              .bg(border),
          )
      }))
      .into_any_element()
  }

  fn render_pane(&self, id: PaneId, bounds: Bounds<Pixels>, focused: bool, split: bool, metrics: &Metrics, cx: &mut Context<Self>) -> AnyElement {
    let Some(pane) = self.panes.get(&id) else { return div().into_any_element() };
    let snapshot = pane.session.snapshot(&metrics.palette);
    let session = pane.session.clone();
    let cell_width = f32::from(metrics.cell_width);
    let background = color(metrics.palette.background);
    let action_context = self.focus_handle.clone();

    div()
      .id(("pane", id))
      .absolute()
      .left(bounds.origin.x)
      .top(bounds.origin.y)
      .w(bounds.size.width)
      .h(bounds.size.height)
      .overflow_hidden()
      .bg(background)
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
        // Out of the flow, so a pane's size comes only from the space layout gives it.
        div()
          .absolute()
          .top_0()
          .left_0()
          .size_full()
          .overflow_hidden()
          .p(PADDING)
          .text_size(FONT_SIZE)
          .font_family(FONT_FAMILY)
          .text_color(color(metrics.palette.foreground))
          .child(rows(&snapshot, &metrics.font)),
      )
      // Panes other than the focused one sit under a lighter veil of the same kind.
      .when(split && !focused, |this| this.child(div().absolute().top_0().left_0().size_full().bg(black().opacity(PANE_VEIL))))
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

/// Divides `area` among the panes of `node`, with a one pixel divider between halves.
fn place(
  node: &Node,
  area: Bounds<Pixels>,
  panes: &mut Vec<(PaneId, Bounds<Pixels>)>,
  dividers: &mut Vec<Divider>,
  path: &mut Vec<bool>,
  sidebar: Pixels,
) {
  match node {
    Node::Leaf(id) => panes.push((*id, area)),
    Node::Split { split, ratio, first, second } => {
      let line = px(1.);
      let (origin, extent) = (area.origin, area.size);
      let (first_area, divider, second_area) = if *split == Split::Vertically {
        let width = ((extent.width - line) * (f32::from(*ratio) / 10000.)).floor();
        (
          Bounds::new(origin, size(width, extent.height)),
          Bounds::new(point(origin.x + width, origin.y), size(line, extent.height)),
          Bounds::new(point(origin.x + width + line, origin.y), size(extent.width - width - line, extent.height)),
        )
      } else {
        let height = ((extent.height - line) * (f32::from(*ratio) / 10000.)).floor();
        (
          Bounds::new(origin, size(extent.width, height)),
          Bounds::new(point(origin.x, origin.y + height), size(extent.width, line)),
          Bounds::new(point(origin.x, origin.y + height + line), size(extent.width, extent.height - height - line)),
        )
      };
      dividers.push(Divider { bounds: divider, drag: DividerDrag { path: path.clone(), split: *split, area, sidebar } });
      path.push(false);
      place(first, first_area, panes, dividers, path, sidebar);
      path.pop();
      path.push(true);
      place(second, second_area, panes, dividers, path, sidebar);
      path.pop();
    }
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
