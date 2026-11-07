/// A hit in the whole buffer. `line` counts from the top of the live screen, so history lines are negative.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Match {
  pub line: i32,
  /// The first cell of the hit.
  pub start: usize,
  /// One past the last cell of the hit.
  pub end: usize,
}

/// A hit on the visible screen, in viewport cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Highlight {
  pub line: usize,
  pub start: usize,
  pub end: usize,
  /// The hit that find next/previous stopped on.
  pub current: bool,
}

/// A character and the cells it covers.
pub(crate) struct RowCell {
  pub character: char,
  pub column: usize,
  pub width: usize,
}

/// The characters to compare: lowercased unless the search is case-sensitive.
pub(crate) fn fold_query(query: &str, case_sensitive: bool) -> Vec<char> {
  query.chars().map(|character| fold(character, case_sensitive)).collect()
}

fn fold(character: char, case_sensitive: bool) -> char {
  if case_sensitive {
    character
  } else {
    character.to_lowercase().next().unwrap_or(character)
  }
}

/// The non-overlapping hits of `query` in one row, as `(start, end)` cells.
pub(crate) fn find_in_row(row: &[RowCell], query: &[char], case_sensitive: bool) -> Vec<(usize, usize)> {
  let mut hits = Vec::new();
  let mut index = 0;
  while !query.is_empty() && index + query.len() <= row.len() {
    if query.iter().zip(&row[index..]).all(|(wanted, cell)| fold(cell.character, case_sensitive) == *wanted) {
      let last = &row[index + query.len() - 1];
      hits.push((row[index].column, last.column + last.width));
      index += query.len();
    } else {
      index += 1;
    }
  }
  hits
}
