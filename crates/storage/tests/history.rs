use mterm_storage::{History, WindowSize};

#[test]
fn history_ignores_blank_and_repeated_commands() {
  let mut history = History::default();
  assert!(history.push("ls"));
  assert!(!history.push("  "));
  assert!(!history.push("ls"));
  assert!(history.push("pwd"));
  assert_eq!(history.entries(), ["ls", "pwd"]);
}

#[test]
fn history_round_trips_and_is_limited() {
  let dir = std::env::temp_dir().join(format!("mterm-history-{}", std::process::id()));
  let path = dir.join("history.txt");
  let mut history = History::default();
  for index in 0..History::LIMIT + 5 {
    history.push(&format!("echo {index}"));
  }
  assert_eq!(history.entries().len(), History::LIMIT);
  history.save(&path).unwrap();
  assert_eq!(History::load(&path), history);
  std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn window_size_falls_back_to_default() {
  let dir = std::env::temp_dir().join(format!("mterm-size-{}", std::process::id()));
  let path = dir.join("size.txt");
  assert_eq!(WindowSize::load(&path), WindowSize::DEFAULT);
  WindowSize { width: 800., height: 500. }.save(&path).unwrap();
  assert_eq!(WindowSize::load(&path), WindowSize { width: 800., height: 500. });
  std::fs::write(&path, "10 10").unwrap();
  assert_eq!(WindowSize::load(&path), WindowSize::DEFAULT);
  std::fs::remove_dir_all(dir).unwrap();
}
