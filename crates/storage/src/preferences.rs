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
}

impl ThemeChoice {
  pub const ALL: [Self; 3] = [Self::System, Self::Light, Self::Dark];

  /// The stored code.
  pub fn code(self) -> &'static str {
    match self {
      Self::System => "system",
      Self::Light => "light",
      Self::Dark => "dark",
    }
  }

  /// Unknown codes follow the system appearance.
  pub fn from_code(code: &str) -> Self {
    match code.trim() {
      "light" => Self::Light,
      "dark" => Self::Dark,
      _ => Self::System,
    }
  }
}

/// Settings and interface state that are remembered between launches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Preferences {
  pub sidebar_open: bool,
  /// Look for a new mTerm release when the app starts.
  pub check_updates: bool,
  pub theme: ThemeChoice,
}

impl Default for Preferences {
  fn default() -> Self {
    Self { sidebar_open: true, check_updates: true, theme: ThemeChoice::System }
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
        "sidebar_open" => preferences.sidebar_open = value.trim().parse().unwrap_or(preferences.sidebar_open),
        "check_updates" => preferences.check_updates = value.trim().parse().unwrap_or(preferences.check_updates),
        "theme" => preferences.theme = ThemeChoice::from_code(value),
        _ => {}
      }
    }
    preferences
  }

  pub fn save(&self, path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent() {
      fs::create_dir_all(parent)?;
    }
    fs::write(path, format!("sidebar_open={}\ncheck_updates={}\ntheme={}\n", self.sidebar_open, self.check_updates, self.theme.code()))
  }
}
