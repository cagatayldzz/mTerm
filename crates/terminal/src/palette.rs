use alacritty_terminal::vte::ansi::{Color, NamedColor};

/// An sRGB color.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rgb {
  pub r: u8,
  pub g: u8,
  pub b: u8,
}

impl Rgb {
  pub const fn new(r: u8, g: u8, b: u8) -> Self {
    Self { r, g, b }
  }

  /// Packs the color as `0xRRGGBB`.
  pub const fn hex(self) -> u32 {
    ((self.r as u32) << 16) | ((self.g as u32) << 8) | self.b as u32
  }

  fn dimmed(self) -> Self {
    let dim = |channel: u8| (channel as u16 * 2 / 3) as u8;
    Self::new(dim(self.r), dim(self.g), dim(self.b))
  }

  /// Blends faint text toward the cell background for both light and dark themes.
  pub(crate) fn faint_on(self, background: Self) -> Self {
    let blend = |foreground: u8, background: u8| ((foreground as u16 + background as u16) / 2) as u8;
    Self::new(blend(self.r, background.r), blend(self.g, background.g), blend(self.b, background.b))
  }
}

/// Default colors plus the 16 ANSI colors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Palette {
  pub foreground: Rgb,
  pub background: Rgb,
  pub ansi: [Rgb; 16],
}

impl Palette {
  /// A muted, high-contrast palette for light terminal backgrounds.
  pub fn light() -> Self {
    Self {
      foreground: Rgb::new(0x00, 0x00, 0x00),
      background: Rgb::new(0xfa, 0xfa, 0xfa),
      ansi: [
        Rgb::new(0x00, 0x00, 0x00),
        Rgb::new(0xb4, 0x23, 0x32),
        Rgb::new(0x1a, 0x7f, 0x37),
        Rgb::new(0x8a, 0x61, 0x00),
        Rgb::new(0x09, 0x69, 0xda),
        Rgb::new(0x82, 0x50, 0xdf),
        Rgb::new(0x1b, 0x7c, 0x83),
        Rgb::new(0x57, 0x60, 0x6a),
        Rgb::new(0x6e, 0x77, 0x81),
        Rgb::new(0xcf, 0x22, 0x2e),
        Rgb::new(0x11, 0x6d, 0x2d),
        Rgb::new(0x7d, 0x4e, 0x00),
        Rgb::new(0x05, 0x50, 0xae),
        Rgb::new(0x66, 0x39, 0xba),
        Rgb::new(0x0a, 0x69, 0x70),
        Rgb::new(0x24, 0x29, 0x2f),
      ],
    }
  }

  pub fn dark() -> Self {
    Self {
      foreground: Rgb::new(0xd4, 0xd4, 0xd4),
      background: Rgb::new(0x1c, 0x1c, 0x1e),
      ansi: [
        Rgb::new(0x1c, 0x1c, 0x1e),
        Rgb::new(0xf4, 0x70, 0x67),
        Rgb::new(0x7e, 0xd6, 0x93),
        Rgb::new(0xe8, 0xc0, 0x7d),
        Rgb::new(0x79, 0xb8, 0xff),
        Rgb::new(0xd2, 0xa8, 0xff),
        Rgb::new(0x76, 0xd6, 0xd1),
        Rgb::new(0xd4, 0xd4, 0xd4),
        Rgb::new(0x6b, 0x6b, 0x70),
        Rgb::new(0xff, 0x91, 0x89),
        Rgb::new(0x9e, 0xe5, 0xa9),
        Rgb::new(0xf2, 0xd0, 0x99),
        Rgb::new(0xa5, 0xd0, 0xff),
        Rgb::new(0xe2, 0xc5, 0xff),
        Rgb::new(0x9b, 0xe5, 0xe2),
        Rgb::new(0xff, 0xff, 0xff),
      ],
    }
  }

  /// Resolves a terminal color to RGB.
  pub(crate) fn resolve(&self, color: Color) -> Rgb {
    match color {
      Color::Spec(rgb) => Rgb::new(rgb.r, rgb.g, rgb.b),
      Color::Indexed(index) => self.indexed(index),
      Color::Named(name) => match name {
        NamedColor::Foreground | NamedColor::Cursor | NamedColor::BrightForeground => self.foreground,
        NamedColor::DimForeground => self.foreground.dimmed(),
        NamedColor::Background => self.background,
        NamedColor::DimBlack => self.ansi[0].dimmed(),
        NamedColor::DimRed => self.ansi[1].dimmed(),
        NamedColor::DimGreen => self.ansi[2].dimmed(),
        NamedColor::DimYellow => self.ansi[3].dimmed(),
        NamedColor::DimBlue => self.ansi[4].dimmed(),
        NamedColor::DimMagenta => self.ansi[5].dimmed(),
        NamedColor::DimCyan => self.ansi[6].dimmed(),
        NamedColor::DimWhite => self.ansi[7].dimmed(),
        // Black..BrightWhite are numbered 0..=15.
        other => self.indexed(other as u8),
      },
    }
  }

  pub(crate) fn indexed(&self, index: u8) -> Rgb {
    match index {
      0..=15 => self.ansi[index as usize],
      16..=231 => {
        let cube = index - 16;
        let level = |value: u8| if value == 0 { 0 } else { 55 + value * 40 };
        Rgb::new(level(cube / 36), level(cube / 6 % 6), level(cube % 6))
      }
      232..=255 => {
        let gray = 8 + (index - 232) * 10;
        Rgb::new(gray, gray, gray)
      }
    }
  }
}
