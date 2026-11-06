use crate::paths::data_dir;
use std::{
  fs, io,
  path::{Path, PathBuf},
};

/// The last window size, in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowSize {
  pub width: f32,
  pub height: f32,
}

impl WindowSize {
  pub const DEFAULT: Self = Self { width: 960., height: 600. };
  pub const MIN: Self = Self { width: 480., height: 300. };

  pub fn default_path() -> PathBuf {
    data_dir().join("window-size.txt")
  }

  /// Loads the saved size; a missing or invalid file yields the default.
  pub fn load(path: &Path) -> Self {
    let Ok(text) = fs::read_to_string(path) else { return Self::DEFAULT };
    let mut parts = text.split_whitespace().map(|part| part.parse::<f32>());
    match (parts.next(), parts.next()) {
      (Some(Ok(width)), Some(Ok(height))) if width >= Self::MIN.width && height >= Self::MIN.height => Self { width, height },
      _ => Self::DEFAULT,
    }
  }

  pub fn save(&self, path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent() {
      fs::create_dir_all(parent)?;
    }
    fs::write(path, format!("{} {}\n", self.width, self.height))
  }
}
