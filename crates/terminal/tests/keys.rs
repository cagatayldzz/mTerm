use mterm_terminal::{dropped_paths, encode_key, paste_bytes, Modifiers};

#[test]
fn dropped_paths_are_escaped_as_separate_shell_arguments() {
  use std::path::Path;

  assert_eq!(dropped_paths([Path::new("/tmp/a file.png"), Path::new("/tmp/it's $(here).png")]), "/tmp/a\\ file.png /tmp/it\\'s\\ \\$\\(here\\).png ");
}

#[test]
fn dropped_paths_end_with_a_space_so_the_next_drop_does_not_glue_to_them() {
  use std::path::Path;

  // Dropping two screenshots one after the other must leave two separate paths on the line.
  let first = dropped_paths([Path::new("/tmp/a.png")]);
  let second = dropped_paths([Path::new("/tmp/b.png")]);
  assert_eq!(format!("{first}{second}"), "/tmp/a.png /tmp/b.png ");
}

#[test]
fn dropped_paths_keep_non_ascii_characters_unescaped() {
  use std::path::Path;

  // `\ş` or `\<U+202F>` would reach programs that read the pasted path themselves as part of the
  // file name, and the file would not be found.
  assert_eq!(dropped_paths([Path::new("/Users/me/görsel şeması.png")]), "/Users/me/görsel\\ şeması.png ");
  assert_eq!(dropped_paths([Path::new("/Users/me/Screenshot 9.20.15\u{202f}PM.png")]), "/Users/me/Screenshot\\ 9.20.15\u{202f}PM.png ");
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

#[test]
fn control_reaches_keys_beyond_letters() {
  let control = Modifiers { control: true, ..plain() };
  assert_eq!(encode_key("space", Some(" "), control, false), Some(vec![0]));
  assert_eq!(encode_key("2", Some("2"), control, false), Some(vec![0]));
  assert_eq!(encode_key("6", Some("6"), control, false), Some(vec![0x1e]));
  assert_eq!(encode_key("-", Some("-"), control, false), Some(vec![0x1f]));
  let control_option = Modifiers { control: true, alt: true, ..plain() };
  assert_eq!(encode_key("b", None, control_option, false), Some(vec![0x1b, 2]));
}

#[test]
fn bracketed_paste_cannot_end_itself_early() {
  assert_eq!(paste_bytes("a\x1b[201~rm -rf ~", true), b"\x1b[200~a[201~rm -rf ~\x1b[201~".to_vec());
}
