use mterm_storage::{open_dropped_paths, prepare_dropped_paths};
use std::{fs, path::PathBuf};

#[test]
fn screenshot_thumbnails_are_copied_and_regular_paths_are_unchanged() {
  let root = std::env::temp_dir().join(format!("mterm-drop-test-{}", std::process::id()));
  let source_dir = root.join("TemporaryItems").join("NSIRD_screencaptureui_test");
  fs::create_dir_all(&source_dir).unwrap();
  let source = source_dir.join("Screenshot.png");
  fs::write(&source, b"test image bytes").unwrap();
  let regular = root.join("ordinary image.png");
  fs::write(&regular, b"ordinary").unwrap();

  let prepared = prepare_dropped_paths(&[source.clone(), regular.clone()]).unwrap();
  assert_ne!(prepared[0], source);
  assert_eq!(fs::read(&prepared[0]).unwrap(), b"test image bytes");
  assert_eq!(prepared[1], regular);
  fs::remove_file(&regular).unwrap();
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

fn unique_root(name: &str) -> PathBuf {
  let root = std::env::temp_dir().join(format!("mterm-{name}-{}", std::process::id()));
  let _ = fs::remove_dir_all(&root);
  fs::create_dir_all(&root).unwrap();
  root
}

#[test]
fn screenshot_with_a_narrow_no_break_space_in_its_name_is_copied_to_a_plain_name() {
  // macOS writes "Screenshot 2026-11-12 at 21.20.15.png" with U+202F before AM/PM.
  let root = unique_root("drop-nnbsp");
  let directory = root.join("TemporaryItems").join("NSIRD_screencaptureui_nnbsp");
  fs::create_dir_all(&directory).unwrap();
  let source = directory.join("Screenshot 2026-11-12 at 9.20.15\u{202f}PM.png");
  fs::write(&source, b"png").unwrap();

  let prepared = prepare_dropped_paths(std::slice::from_ref(&source)).unwrap();
  let name = prepared[0].file_name().unwrap().to_str().unwrap();
  assert!(name.is_ascii(), "the copy keeps a name every program can read back: {name}");
  assert_eq!(prepared[0].extension().unwrap(), "png");
  fs::remove_file(&prepared[0]).unwrap();
  let _ = fs::remove_dir_all(root);
}

#[test]
fn files_from_any_temporary_items_folder_are_copied_before_macos_removes_them() {
  // Other apps (Preview, Photos, Safari) drag transient files from sibling folders of the
  // screenshot one; they vanish when the drag ends just like screenshot thumbnails.
  let root = unique_root("drop-transient");
  let directory = root.join("TemporaryItems").join("NSIRD_Preview_abc123");
  fs::create_dir_all(&directory).unwrap();
  let source = directory.join("image.png");
  fs::write(&source, b"transient").unwrap();

  let opened = open_dropped_paths(std::slice::from_ref(&source)).unwrap();
  fs::remove_file(&source).unwrap();
  let prepared = opened.prepare().unwrap();
  assert_ne!(prepared[0], source);
  assert_eq!(fs::read(&prepared[0]).unwrap(), b"transient");
  fs::remove_file(&prepared[0]).unwrap();
  let _ = fs::remove_dir_all(root);
}

#[test]
fn a_dropped_path_that_no_longer_exists_is_reported_with_its_name() {
  let path = PathBuf::from("/definitely/not/here/picture.png");
  let error = match open_dropped_paths(std::slice::from_ref(&path)) {
    Ok(opened) => opened.prepare().unwrap_err(),
    Err(error) => error,
  };
  assert!(error.to_string().contains("picture.png"), "the notification says which file failed: {error}");
}

#[test]
fn an_empty_screenshot_is_an_error_instead_of_an_empty_image() {
  // The thumbnail can be handed over before macOS finished writing it.
  let root = unique_root("drop-empty");
  let directory = root.join("TemporaryItems").join("NSIRD_screencaptureui_empty");
  fs::create_dir_all(&directory).unwrap();
  let source = directory.join("Screenshot.png");
  fs::write(&source, b"").unwrap();

  assert!(prepare_dropped_paths(&[source]).is_err());
  let _ = fs::remove_dir_all(root);
}

#[test]
fn files_already_in_the_drop_folder_are_not_copied_again() {
  let root = unique_root("drop-twice");
  let directory = root.join("TemporaryItems").join("NSIRD_screencaptureui_twice");
  fs::create_dir_all(&directory).unwrap();
  let source = directory.join("Screenshot.png");
  fs::write(&source, b"once").unwrap();

  let first = prepare_dropped_paths(&[source]).unwrap();
  let second = prepare_dropped_paths(&first).unwrap();
  assert_eq!(first, second);
  fs::remove_file(&first[0]).unwrap();
  let _ = fs::remove_dir_all(root);
}
