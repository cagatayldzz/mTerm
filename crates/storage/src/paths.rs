use std::path::PathBuf;

/// The application data directory (`$MTERM_DATA_DIR` overrides it, mainly for tests).
pub fn data_dir() -> PathBuf {
  if let Some(dir) = std::env::var_os("MTERM_DATA_DIR") {
    return PathBuf::from(dir);
  }
  let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
  home.join("Library").join("Application Support").join("mTerm")
}

/// Turns a `file://` URL (or a plain path) into a directory: a file becomes its parent folder.
/// Returns `None` when it does not point at an existing location.
pub fn directory_from_url(url: &str) -> Option<PathBuf> {
  let path = match url.strip_prefix("file://") {
    Some(rest) => PathBuf::from(percent_decode(rest.strip_prefix("localhost").unwrap_or(rest))),
    None => PathBuf::from(url),
  };
  if path.is_dir() {
    Some(path)
  } else {
    path.parent().filter(|parent| parent.is_dir()).map(PathBuf::from)
  }
}

fn percent_decode(text: &str) -> String {
  let bytes = text.as_bytes();
  let mut decoded = Vec::with_capacity(bytes.len());
  let mut index = 0;
  while index < bytes.len() {
    let escaped = (bytes[index] == b'%' && index + 2 < bytes.len())
      .then(|| text.get(index + 1..index + 3).and_then(|hex| u8::from_str_radix(hex, 16).ok()))
      .flatten();
    match escaped {
      Some(byte) => {
        decoded.push(byte);
        index += 3;
      }
      None => {
        decoded.push(bytes[index]);
        index += 1;
      }
    }
  }
  String::from_utf8_lossy(&decoded).into_owned()
}
