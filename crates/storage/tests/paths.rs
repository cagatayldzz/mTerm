use mterm_storage::directory_from_url;
use std::fs;

#[test]
fn file_urls_are_decoded_into_directories() {
  let dir = std::env::temp_dir().join(format!("mterm paths {}", std::process::id()));
  fs::create_dir_all(&dir).unwrap();
  let url = format!("file://{}/", dir.to_str().unwrap().replace(' ', "%20"));
  assert_eq!(directory_from_url(&url).as_deref(), Some(dir.as_path()));
  assert_eq!(directory_from_url(dir.to_str().unwrap()).as_deref(), Some(dir.as_path()));
  fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_file_resolves_to_its_folder_and_a_missing_path_to_nothing() {
  let dir = std::env::temp_dir().join(format!("mterm file {}", std::process::id()));
  fs::create_dir_all(&dir).unwrap();
  let file = dir.join("note.txt");
  fs::write(&file, "x").unwrap();
  assert_eq!(directory_from_url(&format!("file://{}", file.display())).as_deref(), Some(dir.as_path()));
  assert_eq!(directory_from_url("file:///definitely/not/here/file.txt"), None);
  fs::remove_dir_all(dir).unwrap();
}
