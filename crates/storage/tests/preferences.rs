use mterm_storage::Preferences;

#[test]
fn the_sidebar_is_open_by_default() {
  let dir = std::env::temp_dir().join(format!("mterm-prefs-default-{}", std::process::id()));
  assert!(Preferences::load(&dir.join("missing.txt")).sidebar_open);
}

#[test]
fn the_last_sidebar_state_is_remembered() {
  let dir = std::env::temp_dir().join(format!("mterm-prefs-{}", std::process::id()));
  let path = dir.join("preferences.txt");
  Preferences { sidebar_open: false }.save(&path).unwrap();
  assert!(!Preferences::load(&path).sidebar_open);
  Preferences { sidebar_open: true }.save(&path).unwrap();
  assert!(Preferences::load(&path).sidebar_open);
  std::fs::write(&path, "sidebar_open=maybe\n").unwrap();
  assert!(Preferences::load(&path).sidebar_open);
  std::fs::remove_dir_all(dir).unwrap();
}
