use crate::{
  history::sidebar_toggle,
  workspace::{Workspace, SIDEBAR_WIDTH, TOOLBAR_HEIGHT},
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
  component::{v_flex, ActiveTheme, IconName},
  *,
};
use mterm_terminal::{Rgb, Snapshot};

const FONT_FAMILY: &str = "Menlo";
const FONT_SIZE: Pixels = px(13.);
const LINE_HEIGHT: Pixels = px(17.);
const PADDING: Pixels = px(8.);

fn color(rgb: Rgb) -> Hsla {
  gpui_kit::rgb(rgb.hex()).into()
}

impl Workspace {
  /// The terminal screen. It also fits the PTY to the space the window leaves for it.
  pub(crate) fn render_terminal(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let palette = Self::palette(cx);
    let base = font(FONT_FAMILY);
    let font_id = window.text_system().resolve_font(&base);
    let cell_width = window.text_system().advance(font_id, FONT_SIZE, 'm').map(|size| size.width).unwrap_or(px(7.8));

    let viewport = window.viewport_size();
    let sidebar = if self.sidebar_open { SIDEBAR_WIDTH } else { px(0.) };
    let width = f32::from(viewport.width - sidebar - PADDING * 2.);
    let height = f32::from(viewport.height - TOOLBAR_HEIGHT - PADDING * 2.);
    let columns = (width / f32::from(cell_width)).floor().max(2.) as usize;
    let lines = (height / f32::from(LINE_HEIGHT)).floor().max(1.) as usize;

    let content = match &self.session {
      Ok(session) => {
        session.resize(columns, lines);
        let snapshot = session.snapshot(&palette);
        rows(&snapshot, &base).into_any_element()
      }
      Err(message) => div().text_color(color(palette.foreground)).child(message.clone()).into_any_element(),
    };

    v_flex()
      .id("terminal")
      .relative()
      .flex_1()
      .min_w_0()
      .h_full()
      .overflow_hidden()
      .p(PADDING)
      .bg(color(palette.background))
      .text_size(FONT_SIZE)
      .font_family(FONT_FAMILY)
      .text_color(color(palette.foreground))
      .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| this.focus_handle.focus(window, cx)))
      .on_scroll(cx.listener(|this, event: &ScrollEvent, _, cx| {
        let Ok(session) = &this.session else { return };
        this.scroll_remainder += f32::from(event.delta.pixel_delta(LINE_HEIGHT).y) / f32::from(LINE_HEIGHT);
        let lines = this.scroll_remainder.trunc();
        if lines != 0. {
          this.scroll_remainder -= lines;
          session.scroll(lines as i32);
          cx.notify();
        }
      }))
      .child(content)
      // With the sidebar closed, the toggle floats in the corner of the page.
      .when(!self.sidebar_open, |this| {
        this.child(div().absolute().top(px(6.)).right(px(8.)).child(sidebar_toggle(IconName::PanelLeftOpen, "Show history (⌘B)")))
      })
      .border_color(cx.theme().border)
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
