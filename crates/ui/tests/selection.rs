use super::*;

#[test]
fn double_click_selects_a_word_in_cell_coordinates() {
  assert_eq!(word_range("hello 世界 test", 7), Some((6, 10)));
  assert_eq!(word_range("hello 世界 test", 12), Some((11, 15)));
  assert_eq!(word_range("hello", 7), None);
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
