use crate::{data_dir, History};
use std::{
  fs, io,
  path::{Path, PathBuf},
  sync::atomic::{AtomicU64, Ordering},
  time::{SystemTime, UNIX_EPOCH},
};

static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(0);

/// A saved terminal session. Running processes are not persisted.
#[derive(Clone, Debug, PartialEq)]
pub struct SavedSession {
  pub id: String,
  pub name: String,
  pub directory: PathBuf,
  pub pinned: bool,
}

impl SavedSession {
  pub fn new(name: String, directory: PathBuf) -> Self {
    let time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    Self { id: format!("{time}-{}-{}", std::process::id(), NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed)), name, directory, pinned: false }
  }

  pub fn directory_path() -> PathBuf {
    data_dir().join("sessions")
  }

  pub fn list(root: &Path) -> Vec<Self> {
    Self::try_list(root).unwrap_or_default()
  }

  /// Reads the session store without treating an I/O failure as an empty store.
  pub fn try_list(root: &Path) -> io::Result<Vec<Self>> {
    let entries = match fs::read_dir(root) {
      Ok(entries) => entries,
      Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
      Err(error) => return Err(error),
    };
    let mut sessions = Vec::new();
    for entry in entries {
      let entry = entry?;
      let Ok(id) = entry.file_name().into_string() else { continue };
      if !Self::valid_id(&id) {
        continue;
      }
      let contents = match fs::read(entry.path().join("session.json")) {
        Ok(contents) => contents,
        Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
        Err(error) => return Err(error),
      };
      let json: serde_json::Value = serde_json::from_slice(&contents)?;
      let name =
        json.get("name").and_then(|value| value.as_str()).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing session name"))?;
      let directory = json
        .get("directory")
        .and_then(|value| value.as_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing session directory"))?;
      sessions.push(Self {
        id,
        name: name.to_owned(),
        directory: PathBuf::from(directory),
        pinned: json.get("pinned").and_then(|value| value.as_bool()).unwrap_or(false),
      });
    }
    Self::sort(&mut sessions);
    Ok(sessions)
  }

  /// Pinned sessions come first; each group is ordered newest first.
  pub fn sort(sessions: &mut [Self]) {
    fn created(id: &str) -> (u128, u64) {
      let mut parts = id.split('-');
      let first = parts.next().and_then(|part| part.parse().ok()).unwrap_or(0);
      let second = parts.next().and_then(|part| part.parse().ok()).unwrap_or(0);
      let sequence = parts.next().and_then(|part| part.parse().ok()).unwrap_or(0);
      (first.max(second), sequence)
    }
    sessions.sort_by(|a, b| b.pinned.cmp(&a.pinned).then_with(|| created(&b.id).cmp(&created(&a.id))).then_with(|| b.id.cmp(&a.id)));
  }

  pub fn save(&self, root: &Path) -> io::Result<()> {
    if !Self::valid_id(&self.id) {
      return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid session ID"));
    }
    let path = root.join(&self.id);
    fs::create_dir_all(&path)?;
    let json = serde_json::json!({ "name": self.name, "directory": self.directory, "pinned": self.pinned });
    let temporary = path.join("session.json.tmp");
    fs::write(&temporary, serde_json::to_vec(&json)?)?;
    fs::rename(temporary, path.join("session.json"))
  }

  /// Removes only the saved session data, never its working directory.
  pub fn delete(&self, root: &Path) -> io::Result<()> {
    if !Self::valid_id(&self.id) {
      return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid session ID"));
    }
    fs::remove_dir_all(root.join(&self.id))
  }

  pub fn last_selected(root: &Path) -> Option<String> {
    let id = fs::read_to_string(root.join("active.txt")).ok()?;
    let id = id.trim();
    Self::valid_id(id).then(|| id.to_owned())
  }

  pub fn remember_selected(&self, root: &Path) -> io::Result<()> {
    if !Self::valid_id(&self.id) {
      return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid session ID"));
    }
    fs::create_dir_all(root)?;
    fs::write(root.join("active.txt"), &self.id)
  }

  pub fn load_layout(&self, root: &Path) -> Option<String> {
    fs::read_to_string(root.join(&self.id).join("layout.txt")).ok()
  }

  pub fn save_layout(&self, root: &Path, layout: &str) -> io::Result<()> {
    if !Self::valid_id(&self.id) {
      return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid session ID"));
    }
    let path = root.join(&self.id);
    fs::create_dir_all(&path)?;
    fs::write(path.join("layout.txt"), format!("{layout}\n"))
  }

  pub fn history_path(&self, root: &Path) -> PathBuf {
    root.join(&self.id).join("history.txt")
  }

  pub fn history(&self, root: &Path) -> History {
    History::load(&self.history_path(root))
  }

  fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.bytes().all(|byte| byte.is_ascii_digit() || byte == b'-')
  }
}
