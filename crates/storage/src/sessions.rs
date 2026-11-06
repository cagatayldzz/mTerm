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
}

impl SavedSession {
  pub fn new(name: String, directory: PathBuf) -> Self {
    let time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    Self { id: format!("{time}-{}-{}", std::process::id(), NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed)), name, directory }
  }

  pub fn directory_path() -> PathBuf {
    data_dir().join("sessions")
  }

  pub fn list(root: &Path) -> Vec<Self> {
    let mut sessions: Vec<Self> = fs::read_dir(root)
      .into_iter()
      .flatten()
      .flatten()
      .filter_map(|entry| {
        let id = entry.file_name().into_string().ok()?;
        if !Self::valid_id(&id) {
          return None;
        }
        let json: serde_json::Value = serde_json::from_slice(&fs::read(entry.path().join("session.json")).ok()?).ok()?;
        Some(Self { id, name: json.get("name")?.as_str()?.to_owned(), directory: PathBuf::from(json.get("directory")?.as_str()?) })
      })
      .collect();
    sessions.sort_by(|a, b| a.id.cmp(&b.id));
    sessions
  }

  pub fn save(&self, root: &Path) -> io::Result<()> {
    if !Self::valid_id(&self.id) {
      return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid session ID"));
    }
    let path = root.join(&self.id);
    fs::create_dir_all(&path)?;
    let json = serde_json::json!({ "name": self.name, "directory": self.directory });
    let temporary = path.join("session.json.tmp");
    fs::write(&temporary, serde_json::to_vec(&json)?)?;
    fs::rename(temporary, path.join("session.json"))
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
