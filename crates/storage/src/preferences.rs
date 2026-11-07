use crate::paths::data_dir;
use std::{
  fs, io,
  path::{Path, PathBuf},
};

/// Which appearance the interface uses.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ThemeChoice {
  /// Follow the computer's light or dark appearance.
  #[default]
  System,
  Light,
  Dark,
  SlateDark,
  SlateLight,
  GithubDark,
  GithubLight,
}

impl ThemeChoice {
  pub const ALL: [Self; 7] = [Self::System, Self::Light, Self::Dark, Self::SlateDark, Self::SlateLight, Self::GithubDark, Self::GithubLight];

  /// The stored code.
  pub fn code(self) -> &'static str {
    match self {
      Self::System => "system",
      Self::Light => "light",
      Self::Dark => "dark",
      Self::SlateDark => "slate-dark",
      Self::SlateLight => "slate-light",
      Self::GithubDark => "github-dark",
      Self::GithubLight => "github-light",
    }
  }

  /// Unknown codes follow the system appearance.
  pub fn from_code(code: &str) -> Self {
    match code.trim() {
      "light" => Self::Light,
      "dark" => Self::Dark,
      "slate" | "slate-dark" => Self::SlateDark,
      "slate-light" => Self::SlateLight,
      "github-dark" => Self::GithubDark,
      "github-light" => Self::GithubLight,
      _ => Self::System,
    }
  }
}

/// The font used by the application interface.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FontChoice {
  #[default]
  Default,
  Geist,
  Inter,
}

impl FontChoice {
  pub const ALL: [Self; 3] = [Self::Default, Self::Geist, Self::Inter];

  pub fn code(self) -> &'static str {
    match self {
      Self::Default => "default",
      Self::Geist => "geist",
      Self::Inter => "inter",
    }
  }

  pub fn from_code(code: &str) -> Self {
    match code.trim() {
      "geist" => Self::Geist,
      "inter" => Self::Inter,
      _ => Self::Default,
    }
  }
}

/// Settings and interface state that are remembered between launches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Preferences {
  /// Look for a new mTerm release when the app starts.
  pub check_updates: bool,
  pub theme: ThemeChoice,
  pub font: FontChoice,
  pub terminal_font_size: u8,
  pub terminal_line_height: u8,
}

impl Default for Preferences {
  fn default() -> Self {
    Self { check_updates: true, theme: ThemeChoice::System, font: FontChoice::Default, terminal_font_size: 12, terminal_line_height: 16 }
  }
}

impl Preferences {
  pub fn default_path() -> PathBuf {
    data_dir().join("preferences.txt")
  }

  /// Loads the preferences; a missing or unreadable file, or an unknown value, keeps the default.
  pub fn load(path: &Path) -> Self {
    let mut preferences = Self::default();
    for line in fs::read_to_string(path).unwrap_or_default().lines() {
      let Some((key, value)) = line.split_once('=') else { continue };
      match key.trim() {
        "check_updates" => preferences.check_updates = value.trim().parse().unwrap_or(preferences.check_updates),
        "theme" => preferences.theme = ThemeChoice::from_code(value),
        "font" => preferences.font = FontChoice::from_code(value),
        "terminal_font_size" => {
          if let Ok(size @ 9..=24) = value.trim().parse::<u8>() {
            preferences.terminal_font_size = size;
          }
        }
        "terminal_line_height" => {
          if let Ok(height @ 12..=36) = value.trim().parse::<u8>() {
            preferences.terminal_line_height = height;
          }
        }
        _ => {}
      }
    }
    preferences.terminal_line_height = preferences.terminal_line_height.max(preferences.terminal_font_size);
    preferences
  }

  pub fn save(&self, path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent() {
      fs::create_dir_all(parent)?;
    }
    fs::write(
      path,
      format!(
        "check_updates={}\ntheme={}\nfont={}\nterminal_font_size={}\nterminal_line_height={}\n",
        self.check_updates,
        self.theme.code(),
        self.font.code(),
        self.terminal_font_size,
        self.terminal_line_height
      ),
    )
  }
}
