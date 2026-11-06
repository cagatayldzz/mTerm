use std::path::PathBuf;

/// The application data directory (`$MTERM_DATA_DIR` overrides it, mainly for tests).
pub fn data_dir() -> PathBuf {
  if let Some(dir) = std::env::var_os("MTERM_DATA_DIR") {
    return PathBuf::from(dir);
  }
  let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
  home.join("Library").join("Application Support").join("mTerm")
}
