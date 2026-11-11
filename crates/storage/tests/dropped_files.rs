use mterm_storage::{open_dropped_paths, prepare_dropped_paths};
use std::{fs, path::PathBuf};

#[test]
fn screenshot_thumbnails_are_copied_and_regular_paths_are_unchanged() {
  let root = std::env::temp_dir().join(format!("mterm-drop-test-{}", std::process::id()));
  let source_dir = root.join("TemporaryItems").join("NSIRD_screencaptureui_test");
  fs::create_dir_all(&source_dir).unwrap();
  let source = source_dir.join("Screenshot.png");
  fs::write(&source, b"test image bytes").unwrap();
  let regular = PathBuf::from("/some/ordinary image.png");

  let prepared = prepare_dropped_paths(&[source.clone(), regular.clone()]).unwrap();
  assert_ne!(prepared[0], source);
  assert_eq!(fs::read(&prepared[0]).unwrap(), b"test image bytes");
  assert_eq!(prepared[1], regular);
  fs::remove_file(&prepared[0]).unwrap();
  fs::remove_file(source).unwrap();
  fs::remove_dir(&source_dir).unwrap();
  let _ = fs::remove_dir(root.join("TemporaryItems"));
  let _ = fs::remove_dir(root);
}

#[test]
fn missing_screenshot_reports_the_read_error() {
  let path = PathBuf::from("/tmp/TemporaryItems/NSIRD_screencaptureui_missing/Screenshot.png");
  assert!(prepare_dropped_paths(&[path]).is_err());
}

#[test]
fn opened_screenshot_survives_source_removal_before_copying() {
  let root = std::env::temp_dir().join(format!("mterm-drop-open-test-{}", std::process::id()));
  let source_dir = root.join("TemporaryItems").join("NSIRD_screencaptureui_test");
  fs::create_dir_all(&source_dir).unwrap();
  let source = source_dir.join("Screenshot.png");
  fs::write(&source, b"image bytes").unwrap();

  let opened = open_dropped_paths(std::slice::from_ref(&source)).unwrap();
  fs::remove_file(&source).unwrap();
  let prepared = opened.prepare().unwrap();
  assert_eq!(fs::read(&prepared[0]).unwrap(), b"image bytes");
  fs::remove_file(&prepared[0]).unwrap();
  fs::remove_dir(&source_dir).unwrap();
  let _ = fs::remove_dir(root.join("TemporaryItems"));
  let _ = fs::remove_dir(root);
}
