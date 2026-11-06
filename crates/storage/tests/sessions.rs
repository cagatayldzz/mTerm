use mterm_storage::{History, SavedSession};
use std::{fs, path::PathBuf};

#[test]
fn sessions_keep_metadata_layout_and_history_separate() {
  let root = std::env::temp_dir().join(format!("mterm-sessions-{}", std::process::id()));
  let first = SavedSession::new("Session 1".into(), PathBuf::from("/tmp/first folder"));
  let second = SavedSession::new("Session 2".into(), PathBuf::from("/tmp/second folder"));
  assert_ne!(first.id, second.id);
  first.save(&root).unwrap();
  second.save(&root).unwrap();
  first.save_layout(&root, "V5000(L,L)").unwrap();
  second.remember_selected(&root).unwrap();
  assert_eq!(SavedSession::last_selected(&root), Some(second.id.clone()));
  let mut first_history = History::default();
  first_history.push("echo first");
  first_history.save(&first.history_path(&root)).unwrap();
  let mut second_history = History::default();
  second_history.push("echo second");
  second_history.save(&second.history_path(&root)).unwrap();
  let sessions = SavedSession::list(&root);
  assert!(sessions.contains(&first));
  assert!(sessions.contains(&second));
  assert_eq!(first.history(&root).entries(), &["echo first"]);
  assert_eq!(second.history(&root).entries(), &["echo second"]);
  assert_eq!(first.load_layout(&root).as_deref(), Some("V5000(L,L)\n"));
  assert!(second.load_layout(&root).is_none());
  fs::remove_dir_all(root).unwrap();
}

#[test]
fn invalid_session_ids_cannot_write_outside_the_store() {
  let root = std::env::temp_dir().join(format!("mterm-invalid-session-{}", std::process::id()));
  let mut session = SavedSession::new("Unsafe".into(), PathBuf::from("/tmp"));
  session.id = "../outside".into();
  assert!(session.save(&root).is_err());
  assert!(session.save_layout(&root, "L").is_err());
}
