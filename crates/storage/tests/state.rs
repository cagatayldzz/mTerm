use mterm_storage::{SavedBounds, SavedState, SavedTab, SavedWindow};
use std::{fs, path::PathBuf};

fn temporary_file(name: &str) -> PathBuf {
  std::env::temp_dir().join(format!("mterm-state-{}-{name}", std::process::id())).join("state.json")
}

#[test]
fn state_round_trips_windows_tabs_and_directories() {
  let path = temporary_file("round-trip");
  let state = SavedState {
    windows: vec![
      SavedWindow {
        tabs: vec![
          SavedTab {
            layout: "V5000(L,L)".into(),
            directories: vec![PathBuf::from("/tmp/a b"), PathBuf::from("/tmp")],
            focused: 1,
            pinned: true,
            pinned_panes: vec![true, false],
            title: Some("Build".into()),
          },
          SavedTab { layout: "L".into(), directories: vec![PathBuf::from("/")], focused: 0, pinned: false, pinned_panes: vec![false], title: None },
        ],
        active: 1,
        bounds: Some(SavedBounds { x: 40., y: 60.5, width: 960., height: 600. }),
      },
      SavedWindow {
        tabs: vec![SavedTab {
          layout: "L".into(),
          directories: vec![PathBuf::from("/var")],
          focused: 0,
          pinned: false,
          pinned_panes: vec![false],
          title: None,
        }],
        active: 0,
        bounds: None,
      },
    ],
  };
  state.save(&path).unwrap();
  assert_eq!(SavedState::load(&path), state);
  fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

#[test]
fn missing_or_broken_state_is_empty() {
  let path = temporary_file("broken");
  assert_eq!(SavedState::load(&path), SavedState::default());
  fs::create_dir_all(path.parent().unwrap()).unwrap();
  fs::write(&path, "not json").unwrap();
  assert_eq!(SavedState::load(&path), SavedState::default());
  fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

#[test]
fn windows_without_tabs_are_dropped_and_the_active_tab_is_clamped() {
  let path = temporary_file("clamp");
  fs::create_dir_all(path.parent().unwrap()).unwrap();
  fs::write(&path, r#"{"windows":[{"tabs":[],"active":0},{"tabs":[{"layout":"L","directories":["/tmp"],"focused":0}],"active":7}]}"#).unwrap();
  let state = SavedState::load(&path);
  assert_eq!(state.windows.len(), 1);
  assert_eq!(state.windows[0].active, 0);
  fs::remove_dir_all(path.parent().unwrap()).unwrap();
}
