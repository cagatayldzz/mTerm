use mterm_storage::{History, SavedSession};
use std::{fs, path::PathBuf};

#[test]
fn sessions_keep_metadata_layout_and_history_separate() {
  let root = std::env::temp_dir().join(format!("mterm-sessions-{}", std::process::id()));
  let first = SavedSession::new("Session 1".into(), PathBuf::from("/tmp/first folder"));
  let mut second = SavedSession::new("Session 2".into(), PathBuf::from("/tmp/second folder"));
  second.pinned = true;
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
  assert_eq!(sessions[0], second);
  assert_eq!(first.history(&root).entries(), &["echo first"]);
  assert_eq!(second.history(&root).entries(), &["echo second"]);
  assert_eq!(first.load_layout(&root).as_deref(), Some("V5000(L,L)\n"));
  assert!(second.load_layout(&root).is_none());
  second.delete(&root).unwrap();
  assert_eq!(SavedSession::list(&root), vec![first]);
  fs::remove_dir_all(root).unwrap();
}

#[test]
fn invalid_session_ids_cannot_write_outside_the_store() {
  let root = std::env::temp_dir().join(format!("mterm-invalid-session-{}", std::process::id()));
  let mut session = SavedSession::new("Unsafe".into(), PathBuf::from("/tmp"));
  session.id = "../outside".into();
  assert!(session.save(&root).is_err());
  assert!(session.delete(&root).is_err());
  assert!(session.save_layout(&root, "L").is_err());
}

#[test]
fn older_session_records_are_not_pinned() {
  let root = std::env::temp_dir().join(format!("mterm-old-session-{}", std::process::id()));
  let session = SavedSession::new("Old".into(), PathBuf::from("/tmp"));
  fs::create_dir_all(root.join(&session.id)).unwrap();
  fs::write(root.join(&session.id).join("session.json"), r#"{"name":"Old","directory":"/tmp"}"#).unwrap();
  assert_eq!(SavedSession::list(&root), vec![session]);
  fs::remove_dir_all(root).unwrap();
}

#[test]
fn deleting_a_session_keeps_its_working_directory() {
  let root = std::env::temp_dir().join(format!("mterm-delete-session-{}", std::process::id()));
  let working = root.join("working-directory");
  fs::create_dir_all(&working).unwrap();
  fs::write(working.join("keep.txt"), "important").unwrap();
  let session = SavedSession::new("Session".into(), working.clone());
  session.save(&root).unwrap();
  session.delete(&root).unwrap();
  assert_eq!(fs::read_to_string(working.join("keep.txt")).unwrap(), "important");
  fs::remove_dir_all(root).unwrap();
}

#[test]
fn session_read_errors_are_not_treated_as_an_empty_list() {
  let root = std::env::temp_dir().join(format!("mterm-bad-session-{}", std::process::id()));
  let session = SavedSession::new("Session".into(), PathBuf::from("/tmp"));
  session.save(&root).unwrap();
  fs::write(root.join(&session.id).join("session.json"), "invalid").unwrap();
  assert!(SavedSession::try_list(&root).is_err());
  fs::remove_dir_all(root).unwrap();
}

#[test]
fn pinned_sessions_come_first_and_newest_sessions_lead_each_group() {
  let root = std::env::temp_dir().join(format!("mterm-session-order-{}", std::process::id()));
  let mut oldest = SavedSession::new("Old".into(), PathBuf::from("/tmp"));
  let mut newer = SavedSession::new("New".into(), PathBuf::from("/tmp"));
  let mut pinned = SavedSession::new("Pinned".into(), PathBuf::from("/tmp"));
  oldest.id = "123-1700000000000000000".into();
  newer.id = "1800000000000000000-123-0".into();
  pinned.id = "1600000000000000000-123-0".into();
  pinned.pinned = true;
  for session in [&oldest, &newer, &pinned] {
    session.save(&root).unwrap();
  }
  assert_eq!(SavedSession::list(&root), vec![pinned, newer, oldest]);
  fs::remove_dir_all(root).unwrap();
}
