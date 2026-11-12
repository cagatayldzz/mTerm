use mterm_storage::{issue_url, scrub_home, Diagnostics};
use std::{fs, path::Path, path::PathBuf};

fn diagnostics(name: &str) -> Diagnostics {
  let dir: PathBuf = std::env::temp_dir().join(format!("mterm-diagnostics-{name}-{}", std::process::id()));
  let _ = fs::remove_dir_all(&dir);
  Diagnostics::new(dir)
}

#[test]
fn log_lines_are_appended_and_the_tail_returns_the_newest() {
  let diagnostics = diagnostics("tail");
  for number in 0..5 {
    diagnostics.log(&format!("line {number}")).unwrap();
  }
  let tail = diagnostics.log_tail(2);
  assert_eq!(tail.lines().count(), 2);
  assert!(tail.lines().next().unwrap().ends_with("line 3"));
  assert!(tail.lines().last().unwrap().ends_with("line 4"));
  let _ = fs::remove_dir_all(diagnostics.dir());
}

#[test]
fn a_multi_line_message_stays_one_log_line() {
  let diagnostics = diagnostics("multiline");
  diagnostics.log("first\nsecond").unwrap();
  assert_eq!(diagnostics.log_tail(10).lines().count(), 1);
  let _ = fs::remove_dir_all(diagnostics.dir());
}

#[test]
fn a_large_log_is_moved_aside_instead_of_growing() {
  let diagnostics = diagnostics("rotate");
  let line = "x".repeat(1000);
  for _ in 0..600 {
    diagnostics.log(&line).unwrap();
  }
  assert!(diagnostics.dir().join("mterm.log.1").exists());
  assert!(fs::metadata(diagnostics.dir().join("mterm.log")).unwrap().len() < 512 * 1024);
  let _ = fs::remove_dir_all(diagnostics.dir());
}

#[test]
fn reading_a_log_that_does_not_exist_is_empty() {
  assert_eq!(diagnostics("missing").log_tail(10), "");
}

#[cfg(unix)]
#[test]
fn log_and_crash_files_are_readable_only_by_the_owner() {
  use std::os::unix::fs::PermissionsExt as _;

  let diagnostics = diagnostics("private");
  diagnostics.log("hello").unwrap();
  let crash = diagnostics.write_crash("boom").unwrap();
  for path in [diagnostics.dir().join("mterm.log"), crash] {
    assert_eq!(fs::metadata(path).unwrap().permissions().mode() & 0o777, 0o600);
  }
  let _ = fs::remove_dir_all(diagnostics.dir());
}

#[test]
fn a_crash_is_announced_once_and_stays_available_for_reporting() {
  let diagnostics = diagnostics("announce");
  assert_eq!(diagnostics.take_unannounced_crash(), None);
  diagnostics.write_crash("first crash").unwrap();
  diagnostics.write_crash("second crash").unwrap();

  assert_eq!(diagnostics.take_unannounced_crash().as_deref(), Some("second crash"));
  assert_eq!(diagnostics.take_unannounced_crash(), None, "the next launch does not announce it again");
  assert_eq!(diagnostics.latest_crash().as_deref(), Some("second crash"));

  diagnostics.write_crash("third crash").unwrap();
  assert_eq!(diagnostics.take_unannounced_crash().as_deref(), Some("third crash"));
  let _ = fs::remove_dir_all(diagnostics.dir());
}

#[test]
fn only_the_newest_crash_reports_are_kept() {
  let diagnostics = diagnostics("prune");
  for number in 0..15 {
    diagnostics.write_crash(&format!("crash {number}")).unwrap();
  }
  let files = fs::read_dir(diagnostics.dir()).unwrap().count();
  assert_eq!(files, 10);
  assert_eq!(diagnostics.latest_crash().as_deref(), Some("crash 14"));
  let _ = fs::remove_dir_all(diagnostics.dir());
}

#[test]
fn the_home_folder_is_replaced_so_reports_do_not_carry_the_user_name() {
  let text = "panicked at /Users/ada/Desktop/web/mTerm/src/a.rs\nopened /Users/ada/notes.txt";
  assert_eq!(scrub_home(text, Path::new("/Users/ada")), "panicked at ~/Desktop/web/mTerm/src/a.rs\nopened ~/notes.txt");
  assert_eq!(scrub_home(text, Path::new("/")), text, "a root home would mangle every path");
}

#[test]
fn issue_urls_encode_title_and_body() {
  let url = issue_url("https://github.com/o/r/", "Bug: a b", "line 1\n`code` ü");
  assert_eq!(url, "https://github.com/o/r/issues/new?title=Bug%3A%20a%20b&body=line%201%0A%60code%60%20%C3%BC");
}

#[test]
fn a_long_issue_body_is_cut_to_a_url_that_still_opens() {
  let body = "ü".repeat(10_000);
  let url = issue_url("https://github.com/o/r", "t", &body);
  assert!(url.len() <= 7000, "{} bytes", url.len());
  assert!(url.ends_with("%E2%80%A6"));
  let encoded = url.split("&body=").nth(1).unwrap().strip_suffix("%E2%80%A6").unwrap();
  assert!(encoded.len().is_multiple_of(6) && encoded.as_bytes().chunks(6).all(|chunk| chunk == b"%C3%BC"), "no character is cut in the middle");
}
