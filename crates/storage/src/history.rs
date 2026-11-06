use crate::paths::data_dir;
use std::{
  fs, io,
  path::{Path, PathBuf},
};

/// Commands entered in the terminal, newest last, persisted one per line.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct History {
  entries: Vec<String>,
}

impl History {
  /// The most commands kept.
  pub const LIMIT: usize = 500;

  pub fn default_path() -> PathBuf {
    data_dir().join("history.txt")
  }

  pub fn load(path: &Path) -> Self {
    let entries = fs::read_to_string(path).map(|text| text.lines().map(str::to_owned).filter(|line| !line.is_empty()).collect()).unwrap_or_default();
    let mut history = Self { entries };
    history.trim();
    history
  }

  pub fn save(&self, path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent() {
      fs::create_dir_all(parent)?;
    }
    let mut text = self.entries.join("\n");
    text.push('\n');
    fs::write(path, text)
  }

  /// Adds a command. Blank commands and an immediate repeat are ignored.
  pub fn push(&mut self, command: &str) -> bool {
    let command = command.trim();
    if command.is_empty() || self.entries.last().is_some_and(|last| last == command) {
      return false;
    }
    self.entries.push(command.to_owned());
    self.trim();
    true
  }

  pub fn clear(&mut self) {
    self.entries.clear();
  }

  /// Commands in chronological order.
  pub fn entries(&self) -> &[String] {
    &self.entries
  }

  fn trim(&mut self) {
    if self.entries.len() > Self::LIMIT {
      self.entries.drain(..self.entries.len() - Self::LIMIT);
    }
  }
}
