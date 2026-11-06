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
}

/// Default colors plus the 16 ANSI colors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Palette {
  pub foreground: Rgb,
  pub background: Rgb,
  pub ansi: [Rgb; 16],
}

impl Palette {
  /// Black text on a near-white background, like the macOS Terminal "Basic" profile.
  pub fn light() -> Self {
    Self {
      foreground: Rgb::new(0x00, 0x00, 0x00),
      background: Rgb::new(0xfa, 0xfa, 0xfa),
      ansi: [
        Rgb::new(0x00, 0x00, 0x00),
        Rgb::new(0xc9, 0x1b, 0x00),
        Rgb::new(0x00, 0xa6, 0x00),
        Rgb::new(0x99, 0x99, 0x00),
        Rgb::new(0x00, 0x00, 0xb2),
        Rgb::new(0xb2, 0x00, 0xb2),
        Rgb::new(0x00, 0xa6, 0xb2),
        Rgb::new(0xbf, 0xbf, 0xbf),
        Rgb::new(0x66, 0x66, 0x66),
        Rgb::new(0xe5, 0x00, 0x00),
        Rgb::new(0x00, 0xd9, 0x00),
        Rgb::new(0xe5, 0xe5, 0x00),
        Rgb::new(0x00, 0x00, 0xff),
        Rgb::new(0xe5, 0x00, 0xe5),
        Rgb::new(0x00, 0xe5, 0xe5),
        Rgb::new(0xe5, 0xe5, 0xe5),
      ],
    }
  }

  pub fn dark() -> Self {
    Self {
      foreground: Rgb::new(0xd4, 0xd4, 0xd4),
      background: Rgb::new(0x1c, 0x1c, 0x1e),
      ansi: [
        Rgb::new(0x1c, 0x1c, 0x1e),
        Rgb::new(0xf3, 0x5b, 0x5b),
        Rgb::new(0x6c, 0xc6, 0x6c),
        Rgb::new(0xe5, 0xc0, 0x7b),
        Rgb::new(0x61, 0xaf, 0xef),
        Rgb::new(0xc6, 0x78, 0xdd),
        Rgb::new(0x56, 0xb6, 0xc2),
        Rgb::new(0xd4, 0xd4, 0xd4),
        Rgb::new(0x6b, 0x6b, 0x70),
        Rgb::new(0xff, 0x7b, 0x7b),
        Rgb::new(0x8f, 0xe3, 0x8f),
        Rgb::new(0xff, 0xd7, 0x8a),
        Rgb::new(0x82, 0xc4, 0xff),
        Rgb::new(0xe0, 0x9b, 0xff),
        Rgb::new(0x7c, 0xdc, 0xe8),
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

  fn indexed(&self, index: u8) -> Rgb {
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
