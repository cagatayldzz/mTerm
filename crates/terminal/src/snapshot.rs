use crate::palette::{Palette, Rgb};

/// A run of cells that share one style.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
  pub text: String,
  pub foreground: Rgb,
  /// `None` is the default background.
  pub background: Option<Rgb>,
  pub bold: bool,
  pub italic: bool,
  pub underline: bool,
  pub strikeout: bool,
}

/// Where the cursor is, in viewport cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cursor {
  pub line: usize,
  pub column: usize,
}

/// The visible screen, one `Vec<Span>` per line.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Snapshot {
  pub lines: Vec<Vec<Span>>,
  pub cursor: Option<Cursor>,
  pub columns: usize,
  /// Lines scrolled back from the bottom; `0` follows the live output.
  pub scrolled: usize,
}

impl Snapshot {
  /// The text of one line without styling, with trailing blanks removed.
  pub fn line_text(&self, line: usize) -> String {
    let text: String = self.lines.get(line).map(|spans| spans.iter().map(|span| span.text.as_str()).collect()).unwrap_or_default();
    text.trim_end().to_owned()
  }

  /// The whole screen as text.
  pub fn text(&self) -> String {
    (0..self.lines.len()).map(|line| self.line_text(line)).collect::<Vec<_>>().join("\n")
  }
}

/// A cell reduced to what rendering needs.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Glyph {
  pub character: char,
  pub foreground: Rgb,
  pub background: Option<Rgb>,
  pub bold: bool,
  pub italic: bool,
  pub underline: bool,
  pub strikeout: bool,
}

impl Glyph {
  pub fn blank() -> Self {
    Self { character: ' ', foreground: Rgb::default(), background: None, bold: false, italic: false, underline: false, strikeout: false }
  }

  /// Swaps foreground and background, as for inverse video and the cursor.
  pub fn invert(&mut self, palette: &Palette) {
    let background = self.background.unwrap_or(palette.background);
    self.background = Some(self.foreground);
    self.foreground = background;
  }

  fn same_style(&self, other: &Glyph) -> bool {
    self.foreground == other.foreground
      && self.background == other.background
      && self.bold == other.bold
      && self.italic == other.italic
      && self.underline == other.underline
      && self.strikeout == other.strikeout
  }
}

/// Groups a row of glyphs into spans of equal style.
pub(crate) fn spans(row: &[Glyph]) -> Vec<Span> {
  let mut spans: Vec<Span> = Vec::new();
  let mut previous: Option<&Glyph> = None;
  for glyph in row {
    match (spans.last_mut(), previous) {
      (Some(span), Some(last)) if last.same_style(glyph) => span.text.push(glyph.character),
      _ => spans.push(Span {
        text: glyph.character.to_string(),
        foreground: glyph.foreground,
        background: glyph.background,
        bold: glyph.bold,
        italic: glyph.italic,
        underline: glyph.underline,
        strikeout: glyph.strikeout,
      }),
    }
    previous = Some(glyph);
  }
  spans
}
