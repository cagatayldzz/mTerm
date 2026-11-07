use mterm_terminal::{dropped_paths, encode_key, paste_bytes, Modifiers};

#[test]
fn dropped_paths_are_escaped_as_separate_shell_arguments() {
  use std::path::Path;

  assert_eq!(dropped_paths([Path::new("/tmp/a file.png"), Path::new("/tmp/it's $(here).png")]), "/tmp/a\\ file.png /tmp/it\\'s\\ \\$\\(here\\).png");
}

fn plain() -> Modifiers {
  Modifiers::default()
}

#[test]
fn plain_text_is_sent_as_typed() {
  assert_eq!(encode_key("a", Some("a"), plain(), false), Some(b"a".to_vec()));
  assert_eq!(encode_key("space", Some(" "), plain(), false), Some(b" ".to_vec()));
  assert_eq!(encode_key("ç", Some("ç"), plain(), false), Some("ç".as_bytes().to_vec()));
}

#[test]
fn option_keeps_typed_characters_for_non_us_layouts() {
  let alt = Modifiers { alt: true, ..plain() };
  assert_eq!(encode_key("q", Some("@"), alt, false), Some(b"@".to_vec()));
}

#[test]
fn control_letters_become_control_bytes() {
  let control = Modifiers { control: true, ..plain() };
  assert_eq!(encode_key("c", Some("c"), control, false), Some(vec![3]));
  assert_eq!(encode_key("d", None, control, false), Some(vec![4]));
  assert_eq!(encode_key("[", None, control, false), Some(vec![0x1b]));
}

#[test]
fn special_keys_use_escape_sequences() {
  assert_eq!(encode_key("enter", None, plain(), false), Some(b"\r".to_vec()));
  assert_eq!(encode_key("enter", None, Modifiers { shift: true, ..plain() }, false), Some(b"\x1b[13;2u".to_vec()));
  assert_eq!(encode_key("backspace", None, plain(), false), Some(vec![0x7f]));
  assert_eq!(encode_key("tab", None, Modifiers { shift: true, ..plain() }, false), Some(b"\x1b[Z".to_vec()));
  assert_eq!(encode_key("delete", None, plain(), false), Some(b"\x1b[3~".to_vec()));
  assert_eq!(encode_key("f5", None, plain(), false), Some(b"\x1b[15~".to_vec()));
}

#[test]
fn arrows_follow_the_cursor_mode_and_modifiers() {
  assert_eq!(encode_key("up", None, plain(), false), Some(b"\x1b[A".to_vec()));
  assert_eq!(encode_key("up", None, plain(), true), Some(b"\x1bOA".to_vec()));
  let control = Modifiers { control: true, ..plain() };
  assert_eq!(encode_key("right", None, control, false), Some(b"\x1b[1;5C".to_vec()));
}

#[test]
fn iterm_style_line_and_word_navigation() {
  let alt = Modifiers { alt: true, ..plain() };
  let command = Modifiers { command: true, ..plain() };
  assert_eq!(encode_key("left", None, alt, false), Some(b"\x1bb".to_vec()));
  assert_eq!(encode_key("right", None, alt, false), Some(b"\x1bf".to_vec()));
  assert_eq!(encode_key("left", None, command, false), Some(vec![1]));
  assert_eq!(encode_key("backspace", None, command, false), Some(vec![0x15]));
  assert_eq!(encode_key("c", Some("c"), command, false), None);
}

#[test]
fn paste_normalizes_newlines_and_brackets() {
  assert_eq!(paste_bytes("a\nb", false), b"a\rb".to_vec());
  assert_eq!(paste_bytes("a", true), b"\x1b[200~a\x1b[201~".to_vec());
}
