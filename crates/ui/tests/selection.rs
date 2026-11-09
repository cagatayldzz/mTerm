use super::*;

#[test]
fn double_click_selects_a_word_in_cell_coordinates() {
  assert_eq!(word_range("hello 世界 test", 7), Some((6, 10)));
  assert_eq!(word_range("hello 世界 test", 12), Some((11, 15)));
  assert_eq!(word_range("hello", 7), None);
}

#[test]
fn double_click_keeps_paths_and_flags_together() {
  assert_eq!(word_range("cd ~/dev/my-app.rs now", 6), Some((3, 18)));
  assert_eq!(word_range("run --flag=1", 6), Some((4, 10)));
}

#[test]
fn arrow_selection_moves_across_lines_and_wide_characters() {
  let lines = vec!["hello".to_owned(), "a界z".to_owned(), "end".to_owned()];
  assert_eq!(move_endpoint(&lines, (0, 5), "down"), (1, 4));
  assert_eq!(move_endpoint(&lines, (1, 3), "left"), (1, 1));
  assert_eq!(move_endpoint(&lines, (1, 1), "right"), (1, 3));
  assert_eq!(move_endpoint(&lines, (1, 0), "left"), (0, 5));
  assert_eq!(move_endpoint(&lines, (1, 4), "right"), (2, 0));
}

#[test]
fn shift_click_after_a_word_extends_to_the_edges_of_the_clicked_word() {
  let text = "one two three";
  // "two" is selected; Shift+click inside "three" extends to its end.
  assert_eq!(extend_selection(text, ((0, 4), (0, 7)), (0, 10)), ((0, 4), (0, 13)));
  // Shift+click inside "one" extends backwards to its start, keeping the far edge as the anchor.
  assert_eq!(extend_selection(text, ((0, 4), (0, 7)), (0, 1)), ((0, 7), (0, 0)));
}

#[test]
fn shift_click_after_a_plain_click_extends_to_the_pointer() {
  assert_eq!(extend_selection("one two three", ((0, 4), (0, 4)), (0, 10)), ((0, 4), (0, 10)));
}
