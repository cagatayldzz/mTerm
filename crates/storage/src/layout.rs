use crate::data_dir;
use std::{fs, io, path::PathBuf};

/// The last terminal split layout, without shell state.
pub struct SavedLayout;

impl SavedLayout {
  pub fn path() -> PathBuf {
    data_dir().join("layout.txt")
  }

  pub fn load() -> Option<String> {
    fs::read_to_string(Self::path()).ok()
  }

  pub fn save(layout: &str) -> io::Result<()> {
    let path = Self::path();
    if let Some(parent) = path.parent() {
      fs::create_dir_all(parent)?;
    }
    fs::write(path, format!("{layout}\n"))
  }
}
