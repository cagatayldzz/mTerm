use crate::{
  layout::{Node, PaneId, Split},
  workspace::{Pane, Workspace, TOOLBAR_HEIGHT},
  ClearScreen, ClosePane, Copy, NewTab, NewWindow, Paste, SplitHorizontally, SplitVertically,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
  component::{
    button::{Button, ButtonVariants},
    h_flex,
    menu::{ContextMenuExt as _, DropdownMenu as _},
    ActiveTheme, Icon, IconName, Sizable, WindowExt as _,
  },
  *,
};
use mterm_storage::FontChoice;
use mterm_terminal::{
  encode_key, encode_mouse, CursorShape, Modes, MouseAction, MouseButton as TermButton, MouseEvent as TermMouse, Palette, Rgb, Session, Snapshot,
};
use std::{
  cell::Cell,
  path::{Path, PathBuf},
  rc::Rc,
};
use unicode_width::UnicodeWidthChar as _;

const FONT_FAMILY: &str = "Menlo";
const PADDING: Pixels = px(12.);
const DIVIDER_WIDTH: Pixels = px(1.);
const PANE_HEADER_HEIGHT: Pixels = px(28.);
const SCROLLBAR_INSET: Pixels = px(4.);
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

#[derive(Clone)]
struct ScrollbarDrag {
  pane: PaneId,
  start_y: Rc<Cell<Pixels>>,
  scrolled: usize,
  history_size: usize,
  travel: Pixels,
}

impl Render for ScrollbarDrag {
  fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
    Empty
  }
}

fn color(rgb: Rgb) -> Hsla {
  gpui_kit::rgb(rgb.hex()).into()
}

fn cell_at(bounds: Bounds<Pixels>, position: Point<Pixels>, cell_width: f32, line_height: Pixels) -> Option<(usize, usize)> {
  let x = f32::from(position.x - bounds.origin.x - PADDING);
  let y = f32::from(position.y - bounds.origin.y - PADDING);
  (bounds.contains(&position) && x >= 0. && y >= 0.).then(|| ((y / f32::from(line_height)).floor() as usize, (x / cell_width).floor() as usize))
}

/// The cell under the mouse, held inside the screen, for programs that follow the mouse.
fn clamped_cell(
  bounds: Bounds<Pixels>,
  position: Point<Pixels>,
  cell_width: f32,
  line_height: Pixels,
  columns: usize,
  lines: usize,
) -> (usize, usize) {
  let x = f32::from(position.x - bounds.origin.x - PADDING);
  let y = f32::from(position.y - bounds.origin.y - PADDING);
  let line = ((y / f32::from(line_height)).floor().max(0.) as usize).min(lines.saturating_sub(1));
  let column = ((x / cell_width).floor().max(0.) as usize).min(columns.saturating_sub(1));
  (line, column)
}

/// Sends one mouse event to a program that asked for mouse reporting.
fn report_mouse(session: &Session, modes: Modes, button: TermButton, action: MouseAction, (line, column): (usize, usize), held: &Modifiers) {
  let modifiers = mterm_terminal::Modifiers { control: held.control, alt: held.alt, shift: held.shift, command: false };
  if let Some(bytes) = encode_mouse(TermMouse { button, action, column, line, modifiers }, modes.sgr_mouse) {
    session.report(bytes);
  }
}

/// Sends the release of the button that is held for the program, if any.
fn release_mouse(
  pane: &mut Pane,
  bounds: Option<Bounds<Pixels>>,
  position: Point<Pixels>,
  held: &Modifiers,
  cell_width: f32,
  line_height: Pixels,
  size: (usize, usize),
) {
  pane.selection_dragging = false;
  let (Some(button), Some(bounds)) = (pane.mouse_button.take(), bounds) else { return };
  pane.mouse_cell = None;
  let cell = clamped_cell(bounds, position, cell_width, line_height, size.0, size.1);
  report_mouse(&pane.session, pane.session.modes(), button, MouseAction::Release, cell, held);
}

fn openable_url(target: &str, directory: Option<&Path>) -> Option<String> {
  if target.starts_with("https://") || target.starts_with("http://") {
    return Some(target.to_owned());
  }
  let path = if let Some(rest) = target.strip_prefix("~/") {
    PathBuf::from(std::env::var_os("HOME")?).join(rest)
  } else if Path::new(target).is_absolute() {
    PathBuf::from(target)
  } else {
    directory?.join(target)
  };
  if !path.exists() {
    return None;
  }
  url::Url::from_file_path(path).ok().map(Into::into)
}

/// What every pane in a frame shares.
struct Metrics {
  font: Font,
  font_size: Pixels,
  line_height: Pixels,
  cell_width: Pixels,
  fixed_cells: bool,
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
    let choice = self.model.read(cx).preferences.font;
    let font_size = px(self.model.read(cx).preferences.terminal_font_size as f32);
    let line_height = px(self.model.read(cx).preferences.terminal_line_height as f32);
    let family = match choice {
      FontChoice::Default => FONT_FAMILY,
      FontChoice::Geist => "Geist Mono",
      FontChoice::Inter => "Inter",
    };
    let font = font(family);
    let fixed_cells = choice == FontChoice::Inter;
    let text_system = window.text_system();
    let font_id = text_system.resolve_font(&font);
    let mut cell_width = text_system.advance(font_id, font_size, 'm').map(|size| size.width).unwrap_or(font_size * 0.6);
    if fixed_cells {
      let bold_id = text_system.resolve_font(&Font { weight: FontWeight::BOLD, ..font.clone() });
      for id in [font_id, bold_id] {
        for character in ['m', 'W'] {
          if let Ok(advance) = text_system.advance(id, font_size, character) {
            cell_width = cell_width.max(advance.width);
          }
        }
      }
    }
    let metrics = Metrics { font, font_size, line_height, cell_width, fixed_cells, palette };
    let split = root.leaves().len() > 1;
    // Panes are placed at explicit rectangles: flex percentages do not resolve reliably for nested splits.
    let sidebar = px(0.);
    let viewport = window.viewport_size();
    let area = Bounds::new(point(px(0.), px(0.)), size(viewport.width - sidebar, viewport.height - TOOLBAR_HEIGHT));
    let mut panes = Vec::new();
    let mut dividers = Vec::new();
    place(&root, area, &mut panes, &mut dividers, &mut Vec::new(), sidebar);
    let border = cx.theme().sidebar_border;
    let panes: Vec<_> = panes.into_iter().map(|(id, bounds)| self.render_pane(id, bounds, id == focused, split, &metrics, window, cx)).collect();
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
        let fraction = f32::from(position - origin) / f32::from(extent - DIVIDER_WIDTH);
        let ratio = (fraction.clamp(0.1, 0.9) * 10000.) as u16;
        if let Some(tab) = this.tabs.get_mut(this.active) {
          if tab.root.set_ratio(&drag.path, ratio) {
            cx.notify();
          }
        }
      }))
      .on_drag_move(cx.listener(|this, event: &DragMoveEvent<ScrollbarDrag>, _, cx| {
        let drag = event.drag(cx);
        let delta = f32::from(event.event.position.y - drag.start_y.get()) / f32::from(drag.travel);
        let target = (drag.scrolled as f32 - delta * drag.history_size as f32).round().clamp(0., drag.history_size as f32) as usize;
        if let Some(pane) = this.panes.get_mut(&drag.pane) {
          if pane.session.scroll_to(target) {
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

  fn render_pane(
    &self,
    id: PaneId,
    bounds: Bounds<Pixels>,
    focused: bool,
    split: bool,
    metrics: &Metrics,
    window: &Window,
    cx: &mut Context<Self>,
  ) -> AnyElement {
    let Some(pane) = self.panes.get(&id) else { return div().into_any_element() };
    pane.session.set_palette(metrics.palette.clone());
    let snapshot = pane.session.snapshot(&metrics.palette);
    let session = pane.session.clone();
    let cell_width = f32::from(metrics.cell_width);
    let line_height = metrics.line_height;
    let background = color(metrics.palette.background);
    let action_context = self.focus_handle.clone();
    let header_height = if split { PANE_HEADER_HEIGHT } else { px(0.) };
    let title = pane.title.clone();
    let content_bounds = pane.content_bounds.clone();
    let click_bounds = content_bounds.clone();
    let click_palette = metrics.palette.clone();
    let move_bounds = content_bounds.clone();
    let up_bounds = content_bounds.clone();
    let out_bounds = content_bounds.clone();
    let scroll_bounds = content_bounds.clone();
    let middle_bounds = content_bounds.clone();
    let middle_up_bounds = content_bounds.clone();
    let selection = pane.selection;
    let columns = snapshot.columns;
    let line_count = snapshot.lines.len();
    let track_height = (bounds.size.height - header_height - SCROLLBAR_INSET * 2.).max(px(0.));
    let thumb_height = (track_height * (line_count as f32 / (line_count + snapshot.history_size).max(1) as f32)).max(px(28.)).min(track_height);
    let travel = track_height - thumb_height;
    let thumb_top = if snapshot.history_size > 0 { travel * (1. - snapshot.scrolled as f32 / snapshot.history_size as f32) } else { px(0.) };
    let has_scrollbar = snapshot.history_size > 0 && travel > px(0.);
    let scrollbar_start = Rc::new(Cell::new(px(0.)));
    let reporting = pane.session.modes().mouse_reporting();
    let highlights = self.find_highlights(id, cx);
    let hovered = if window.modifiers().platform {
      content_bounds.get().and_then(|bounds| cell_at(bounds, window.mouse_position(), cell_width, line_height)).and_then(|(line, column)| {
        let (target, start, end) = snapshot.link_range_at(line, column)?;
        let url = openable_url(&target, pane.directory.as_deref())?;
        Some((line, start, end, url))
      })
    } else {
      None
    };

    div()
      .id(("pane", id))
      .absolute()
      .left(bounds.origin.x)
      .top(bounds.origin.y)
      .w(bounds.size.width)
      .h(bounds.size.height)
      .overflow_hidden()
      .bg(background)
      .on_mouse_move(cx.listener(move |this, event: &MouseMoveEvent, window, cx| {
        if event.pressed_button == Some(MouseButton::Left) && !cx.has_active_drag() {
          if let (Some(bounds), Some(pane)) = (move_bounds.get(), this.panes.get_mut(&id)) {
            if let Some((anchor, _)) = pane.selection.filter(|_| pane.selection_dragging) {
              let x = f32::from(event.position.x - bounds.origin.x - PADDING);
              let y = f32::from(event.position.y - bounds.origin.y - PADDING);
              let line = (y / f32::from(line_height)).floor().max(0.) as usize;
              let column = (x / cell_width).round().max(0.) as usize;
              pane.selection = Some((anchor, (line.min(line_count.saturating_sub(1)), column.min(columns))));
              cx.notify();
            }
          }
        }
        if event.pressed_button != Some(MouseButton::Left) {
          if let Some(pane) = this.panes.get_mut(&id) {
            pane.selection_dragging = false;
          }
        }
        if reporting && !event.modifiers.shift {
          if let (Some(bounds), Some(pane)) = (move_bounds.get(), this.panes.get_mut(&id)) {
            let modes = pane.session.modes();
            let wanted = if pane.mouse_button.is_some() { modes.mouse_drag || modes.mouse_motion } else { modes.mouse_motion };
            let cell = clamped_cell(bounds, event.position, cell_width, line_height, columns, line_count);
            if wanted && pane.mouse_cell != Some(cell) {
              pane.mouse_cell = Some(cell);
              let (button, action) = match pane.mouse_button {
                Some(button) => (button, MouseAction::Drag),
                None => (TermButton::None, MouseAction::Move),
              };
              report_mouse(&pane.session, modes, button, action, cell, &event.modifiers);
            }
          }
        }
        if window.modifiers().platform {
          if let Some(pane) = this.panes.get_mut(&id) {
            let cell = move_bounds.get().and_then(|bounds| cell_at(bounds, event.position, cell_width, line_height));
            if pane.hover_cell != cell {
              pane.hover_cell = cell;
              cx.notify();
            }
          }
        }
      }))
      .on_modifiers_changed(cx.listener(|_, _, _, cx| cx.notify()))
      .on_mouse_down(
        MouseButton::Left,
        cx.listener(move |this, event: &MouseDownEvent, window, cx| {
          if cx.has_active_drag() {
            return;
          }
          this.focus_pane(id, window, cx);
          if let Some(pane) = this.panes.get_mut(&id) {
            pane.selection_dragging = false;
          }
          if has_scrollbar {
            if let Some(content) = click_bounds.get() {
              let top = content.origin.y + SCROLLBAR_INSET + thumb_top;
              if event.position.x >= content.origin.x + content.size.width - px(14.)
                && event.position.y >= top
                && event.position.y <= top + thumb_height
              {
                return;
              }
            }
          }
          let modes = this.panes.get(&id).map(|pane| pane.session.modes()).unwrap_or_default();
          if modes.mouse_reporting() && !event.modifiers.shift && !event.modifiers.platform {
            // The program follows the mouse itself; holding Shift selects text instead.
            if let (Some(bounds), Some(pane)) = (click_bounds.get(), this.panes.get_mut(&id)) {
              let cell = clamped_cell(bounds, event.position, cell_width, line_height, columns, line_count);
              pane.selection = None;
              pane.mouse_button = Some(TermButton::Left);
              pane.mouse_cell = Some(cell);
              report_mouse(&pane.session, modes, TermButton::Left, MouseAction::Press, cell, &event.modifiers);
            }
            return;
          }
          if !event.modifiers.platform {
            if let Some(bounds) = click_bounds.get() {
              if let Some((line, column)) = cell_at(bounds, event.position, cell_width, line_height) {
                if let Some(pane) = this.panes.get_mut(&id) {
                  let position = (line.min(line_count.saturating_sub(1)), column.min(columns));
                  pane.selection = if event.click_count < 2 {
                    Some((position, position))
                  } else {
                    let text = pane.session.snapshot(&click_palette).line_text(position.0);
                    if event.click_count >= 3 {
                      let width = text.chars().map(|character| character.width().unwrap_or(0)).sum();
                      Some(((position.0, 0), (position.0, width)))
                    } else {
                      crate::selection::word_range(&text, position.1).map(|(start, end)| ((position.0, start), (position.0, end)))
                    }
                  };
                  pane.selection_dragging = pane.selection.is_some();
                  cx.notify();
                }
              }
            }
            return;
          }
          let Some(bounds) = click_bounds.get() else { return };
          let Some((line, column)) = cell_at(bounds, event.position, cell_width, line_height) else { return };
          if let Some(url) = this.panes.get(&id).and_then(|pane| {
            let link = pane.session.snapshot(&click_palette).link_at(line, column)?;
            openable_url(&link, pane.directory.as_deref())
          }) {
            cx.open_url(&url);
          }
        }),
      )
      .on_mouse_up(
        MouseButton::Left,
        cx.listener(move |this, event: &MouseUpEvent, _, _| {
          if let Some(pane) = this.panes.get_mut(&id) {
            release_mouse(pane, up_bounds.get(), event.position, &event.modifiers, cell_width, line_height, (columns, line_count));
          }
        }),
      )
      .on_mouse_up_out(
        MouseButton::Left,
        cx.listener(move |this, event: &MouseUpEvent, _, _| {
          if let Some(pane) = this.panes.get_mut(&id) {
            release_mouse(pane, out_bounds.get(), event.position, &event.modifiers, cell_width, line_height, (columns, line_count));
          }
        }),
      )
      // The middle button pastes, or goes to a program that follows the mouse.
      .on_mouse_down(
        MouseButton::Middle,
        cx.listener(move |this, event: &MouseDownEvent, window, cx| {
          this.focus_pane(id, window, cx);
          let modes = this.panes.get(&id).map(|pane| pane.session.modes()).unwrap_or_default();
          if !modes.mouse_reporting() || event.modifiers.shift {
            this.paste(window, cx);
          } else if let (Some(bounds), Some(pane)) = (middle_bounds.get(), this.panes.get_mut(&id)) {
            let cell = clamped_cell(bounds, event.position, cell_width, line_height, columns, line_count);
            pane.mouse_button = Some(TermButton::Middle);
            report_mouse(&pane.session, modes, TermButton::Middle, MouseAction::Press, cell, &event.modifiers);
          }
        }),
      )
      .on_mouse_up(
        MouseButton::Middle,
        cx.listener(move |this, event: &MouseUpEvent, _, _| {
          if let Some(pane) = this.panes.get_mut(&id).filter(|pane| pane.mouse_button == Some(TermButton::Middle)) {
            release_mouse(pane, middle_up_bounds.get(), event.position, &event.modifiers, cell_width, line_height, (columns, line_count));
          }
        }),
      )
      .on_mouse_down(MouseButton::Right, cx.listener(move |this, _, window, cx| this.focus_pane(id, window, cx)))
      .on_drop(cx.listener(move |this, paths: &ExternalPaths, window, cx| {
        if let Some(pane) = this.panes.get_mut(&id) {
          pane.selection_dragging = false;
        }
        let paths = match mterm_storage::prepare_dropped_paths(paths.paths()) {
          Ok(paths) => paths,
          Err(error) => {
            window.push_notification(component::notification::Notification::error(format!("Could not prepare dropped file: {error}")), cx);
            return;
          }
        };
        let text = mterm_terminal::dropped_paths(paths.iter().map(|path| path.as_path()));
        if !text.is_empty() {
          this.focus_pane(id, window, cx);
          this.insert_text(&text, window, cx);
        }
      }))
      .on_scroll(cx.listener(move |this, event: &ScrollEvent, _, cx| {
        let Some(pane) = this.panes.get_mut(&id) else { return };
        pane.scroll_remainder += f32::from(event.delta.pixel_delta(line_height).y) / f32::from(line_height);
        let lines = pane.scroll_remainder.trunc();
        if lines == 0. {
          return;
        }
        pane.scroll_remainder -= lines;
        let modes = pane.session.modes();
        let (up, count) = (lines > 0., (lines.abs() as usize).min(20));
        if modes.mouse_reporting() && !event.modifiers.shift {
          // A program that follows the mouse scrolls itself.
          if let Some(bounds) = scroll_bounds.get() {
            let cell = clamped_cell(bounds, event.position, cell_width, line_height, columns, line_count);
            let button = if up { TermButton::WheelUp } else { TermButton::WheelDown };
            for _ in 0..count {
              report_mouse(&pane.session, modes, button, MouseAction::Press, cell, &event.modifiers);
            }
          }
        } else if modes.alt_screen && modes.alternate_scroll {
          // Pagers and editors have no history; the wheel moves their cursor like the arrow keys.
          if let Some(bytes) = encode_key(if up { "up" } else { "down" }, None, Default::default(), modes.app_cursor) {
            for _ in 0..count {
              pane.session.report(bytes.clone());
            }
          }
        } else if pane.session.scroll(lines as i32) {
          // The selection is tied to screen positions, which just moved.
          pane.selection = None;
          cx.notify();
        }
      }))
      // Fits the PTY to the space this pane was given.
      .child(
        canvas(
          move |bounds, window, _| {
            content_bounds.set(Some(bounds));
            let columns = ((f32::from(bounds.size.width) - 2. * f32::from(PADDING)) / cell_width).floor().max(2.) as usize;
            let lines = ((f32::from(bounds.size.height) - 2. * f32::from(PADDING)) / f32::from(line_height)).floor().max(1.) as usize;
            if session.resize(columns, lines) {
              window.refresh();
            }
          },
          |_, _, _, _| {},
        )
        .absolute()
        .top(header_height)
        .left_0()
        .right_0()
        .bottom_0(),
      )
      .when(!reporting, |this| this.cursor_text())
      .when_some(hovered, |this, (line, start, end, url)| {
        this
          .cursor_pointer()
          .child(
            div()
              .absolute()
              .left(PADDING + metrics.cell_width * start as f32)
              .top(header_height + PADDING + line_height * (line + 1) as f32 - px(2.))
              .w(metrics.cell_width * (end - start) as f32)
              .h(px(1.))
              .bg(gpui_kit::rgb(0x3b9fff)),
          )
          .child(
            div()
              .absolute()
              .left(px(4.))
              .bottom(px(4.))
              .px_2()
              .py_1()
              .bg(cx.theme().popover)
              .text_color(cx.theme().popover_foreground)
              .text_size(px(11.))
              .child(url),
          )
      })
      .child(
        // Out of the flow, so a pane's size comes only from the space layout gives it.
        div()
          .absolute()
          .top(header_height)
          .left_0()
          .right_0()
          .bottom_0()
          .overflow_hidden()
          .p(PADDING)
          .text_size(metrics.font_size)
          .font_family(metrics.font.family.clone())
          .text_color(color(metrics.palette.foreground))
          .child(if metrics.fixed_cells {
            fixed_cell_rows(&snapshot, &metrics.font, metrics.cell_width, line_height).into_any_element()
          } else {
            rows(&snapshot, &metrics.font, line_height).into_any_element()
          }),
      )
      .children(highlights.into_iter().map(|hit| {
        div()
          .absolute()
          .left(PADDING + metrics.cell_width * hit.start as f32)
          .top(header_height + PADDING + line_height * hit.line as f32)
          .w(metrics.cell_width * (hit.end - hit.start) as f32)
          .h(line_height)
          .bg(if hit.current { gpui_kit::rgb(0xff9f0a).opacity(0.7) } else { gpui_kit::rgb(0xffd60a).opacity(0.4) })
      }))
      .when_some(selection, |this, (anchor, end)| {
        let (start, end) = if anchor < end { (anchor, end) } else { (end, anchor) };
        this.children((start.0..=end.0).filter_map(|line| {
          let text_width: usize = snapshot.line_text(line).chars().map(|character| character.width().unwrap_or(0)).sum();
          let from = if line == start.0 { start.1 } else { 0 };
          let to = (if line == end.0 { end.1 } else { columns }).min(text_width);
          (to > from).then(|| {
            div()
              .absolute()
              .left(PADDING + metrics.cell_width * from as f32)
              .top(header_height + PADDING + line_height * line as f32)
              .w(metrics.cell_width * (to - from) as f32)
              .h(line_height)
              .bg(gpui_kit::rgb(0x69a9ff).opacity(0.4))
          })
        }))
      })
      .when_some(snapshot.cursor.filter(|cursor| focused && cursor.shape != CursorShape::Block), |this, cursor| {
        let (width, height, offset) = match cursor.shape {
          CursorShape::Beam => (px(2.), line_height, px(0.)),
          _ => (metrics.cell_width, px(2.), line_height - px(2.)),
        };
        this.child(
          div()
            .absolute()
            .left(PADDING + metrics.cell_width * cursor.column as f32)
            .top(header_height + PADDING + line_height * cursor.line as f32 + offset)
            .w(width)
            .h(height)
            .bg(color(metrics.palette.foreground)),
        )
      })
      // Panes other than the focused one sit under a lighter veil of the same kind.
      .when(split && !focused, |this| this.child(div().absolute().top(header_height).left_0().right_0().bottom_0().bg(black().opacity(PANE_VEIL))))
      .when(has_scrollbar, |this| {
        let drag = ScrollbarDrag { pane: id, start_y: scrollbar_start, scrolled: snapshot.scrolled, history_size: snapshot.history_size, travel };
        this.child(
          div()
            .id(("scrollbar", id))
            .absolute()
            .right_0()
            .top(header_height + SCROLLBAR_INSET + thumb_top)
            .w(px(14.))
            .h(thumb_height)
            .group("terminal-scrollbar")
            .cursor_pointer()
            .on_drag(drag, |drag, _, window, cx| {
              drag.start_y.set(window.mouse_position().y);
              cx.new(|_| drag.clone())
            })
            .child(
              div()
                .absolute()
                .right(SCROLLBAR_INSET)
                .w(px(6.))
                .h_full()
                .rounded_full()
                .bg(color(metrics.palette.foreground).opacity(0.35))
                .group_hover("terminal-scrollbar", |style| style.bg(color(metrics.palette.foreground).opacity(0.65))),
            ),
        )
      })
      .when(split, |this| this.child(self.render_pane_header(id, title, focused, cx)))
      .context_menu(move |menu, _, _| {
        menu
          .action_context(action_context.clone())
          .menu("New Window", Box::new(NewWindow))
          .menu("New Tab", Box::new(NewTab))
          .separator()
          .menu("Split Pane Vertically", Box::new(SplitVertically))
          .menu("Split Pane Horizontally", Box::new(SplitHorizontally))
          .separator()
          .menu("Copy", Box::new(Copy))
          .menu("Paste", Box::new(Paste))
          .menu("Clear Buffer", Box::new(ClearScreen))
          .separator()
          .menu("Close", Box::new(ClosePane))
      })
      .into_any_element()
  }

  fn render_pane_header(&self, id: PaneId, title: String, focused: bool, cx: &mut Context<Self>) -> impl IntoElement {
    let theme = cx.theme().clone();
    let action_context = self.focus_handle.clone();
    let workspace = cx.weak_entity();
    h_flex()
      .absolute()
      .top_0()
      .left_0()
      .right_0()
      .h(PANE_HEADER_HEIGHT)
      .items_center()
      .gap_1()
      .px_1()
      .bg(if focused { theme.secondary } else { theme.muted })
      .border_b_1()
      .border_color(theme.foreground.opacity(0.25))
      .text_color(if focused { theme.foreground } else { theme.muted_foreground })
      .child(Button::new(("pane-close", id)).icon(IconName::Close).ghost().xsmall().on_click(cx.listener(move |this, _, window, cx| {
        cx.stop_propagation();
        this.close_pane(id, window);
        cx.notify();
      })))
      .child(div().flex_1().min_w_0().overflow_hidden().whitespace_nowrap().text_ellipsis().text_size(px(12.)).child(title))
      .child(
        Button::new(("pane-menu", id))
          .icon(Icon::new(gpui_kit_assets::IconName::CircleEllipsis))
          .ghost()
          .xsmall()
          .dropdown_menu(move |menu, _, _| {
            menu
              .action_context(action_context.clone())
              .menu("New Window", Box::new(NewWindow))
              .menu("New Tab", Box::new(NewTab))
              .separator()
              .menu("Split Pane Vertically", Box::new(SplitVertically))
              .menu("Split Pane Horizontally", Box::new(SplitHorizontally))
              .separator()
              .menu("Copy", Box::new(Copy))
              .menu("Paste", Box::new(Paste))
              .menu("Clear Buffer", Box::new(ClearScreen))
              .separator()
              .menu("Close", Box::new(ClosePane))
          })
          .on_open_change(move |open, window, cx| {
            if *open {
              let _ = workspace.update(cx, |workspace, cx| workspace.focus_pane(id, window, cx));
            }
          }),
      )
  }
}

/// Divides `area` among the panes of `node`, leaving room for the divider.
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
      let line = DIVIDER_WIDTH;
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

fn rows(snapshot: &Snapshot, base: &Font, line_height: Pixels) -> impl IntoElement {
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
      .h(line_height)
      .w_full()
      .flex_shrink_0()
      .overflow_hidden()
      .whitespace_nowrap()
      .child(StyledText::new(text).with_runs(runs))
  });
  div().w_full().children(lines)
}

/// Draws proportional fonts in terminal cells so columns and cursor positions stay aligned.
fn fixed_cell_rows(snapshot: &Snapshot, base: &Font, cell_width: Pixels, line_height: Pixels) -> impl IntoElement {
  let bold = Font { weight: FontWeight::BOLD, ..base.clone() };
  let italic = Font { style: FontStyle::Italic, ..base.clone() };
  let bold_italic = Font { weight: FontWeight::BOLD, style: FontStyle::Italic, ..base.clone() };
  let lines = snapshot.lines.iter().enumerate().map(|(index, spans)| {
    let mut cells = Vec::new();
    for span in spans {
      let foreground = color(span.foreground);
      let font = match (span.bold, span.italic) {
        (true, true) => &bold_italic,
        (true, false) => &bold,
        (false, true) => &italic,
        (false, false) => base,
      };
      let mut characters = span.text.chars().peekable();
      while let Some(character) = characters.next() {
        if character == ' ' && !span.underline && !span.strikeout {
          let mut count = 1;
          while characters.peek() == Some(&' ') {
            characters.next();
            count += 1;
          }
          cells.push(
            div().w(cell_width * count as f32).h_full().flex_shrink_0().when_some(span.background, |this, background| this.bg(color(background))),
          );
          continue;
        }
        let width = character.width().unwrap_or(1).clamp(1, 2);
        let text = character.to_string();
        let run = TextRun {
          len: text.len(),
          font: font.clone(),
          color: foreground,
          background_color: None,
          underline: span.underline.then(|| UnderlineStyle { thickness: px(1.), color: Some(foreground), wavy: false }),
          strikethrough: span.strikeout.then(|| StrikethroughStyle { thickness: px(1.), color: Some(foreground) }),
        };
        cells.push(
          div()
            .w(cell_width * width as f32)
            .h_full()
            .flex_shrink_0()
            .overflow_hidden()
            .whitespace_nowrap()
            .when_some(span.background, |this, background| this.bg(color(background)))
            .child(StyledText::new(text).with_runs(vec![run])),
        );
      }
    }
    div().id(("line", index)).h(line_height).w_full().flex().flex_shrink_0().overflow_hidden().children(cells)
  });
  div().w_full().children(lines)
}
