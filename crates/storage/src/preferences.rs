use crate::paths::data_dir;
use std::{
  fs, io,
  path::{Path, PathBuf},
};

/// Interface state that is remembered between launches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Preferences {
  pub sidebar_open: bool,
}

impl Default for Preferences {
  fn default() -> Self {
    Self { sidebar_open: true }
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
      if let Some(("sidebar_open", value)) = line.split_once('=') {
        if let Ok(value) = value.trim().parse() {
          preferences.sidebar_open = value;
        }
      }
    }
    preferences
  }

  pub fn save(&self, path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent() {
      fs::create_dir_all(parent)?;
    }
    fs::write(path, format!("sidebar_open={}\n", self.sidebar_open))
  }
}
