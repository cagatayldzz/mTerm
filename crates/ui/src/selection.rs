use unicode_width::UnicodeWidthChar as _;

pub(crate) fn word_range(text: &str, column: usize) -> Option<(usize, usize)> {
  let mut cells = Vec::new();
  let mut position = 0;
  for character in text.chars() {
    let width = character.width().unwrap_or(0);
    if width > 0 {
      let kind = if character.is_whitespace() {
        0
      } else if character.is_alphanumeric() || "_-./+~\\".contains(character) {
        // Path and flag characters stay in the word, so a double click takes a whole path.
        1
      } else {
        2
      };
      cells.push((position, position + width, kind));
      position += width;
    }
  }
  let index = cells.iter().position(|(start, end, _)| *start <= column && column < *end)?;
  let kind = cells[index].2;
  let mut first = index;
  let mut last = index;
  while first > 0 && cells[first - 1].2 == kind {
    first -= 1;
  }
  while last + 1 < cells.len() && cells[last + 1].2 == kind {
    last += 1;
  }
  Some((cells[first].0, cells[last].1))
}

/// Extends `selection` to `position`, like Shift+click in a browser. After a word or line was
/// selected the far edge snaps to the clicked word; after a plain click it follows the pointer.
/// `text` is the clicked line.
pub(crate) fn extend_selection(
  text: &str,
  selection: ((usize, usize), (usize, usize)),
  position: (usize, usize),
) -> ((usize, usize), (usize, usize)) {
  let (start, end) = if selection.0 <= selection.1 { selection } else { (selection.1, selection.0) };
  let by_word = start != end;
  let word = if by_word { word_range(text, position.1) } else { None };
  if position < start {
    let focus = word.map_or(position, |(first, _)| (position.0, first));
    (end, focus)
  } else {
    let focus = word.map_or(position, |(_, last)| (position.0, last));
    (start, focus)
  }
}

pub(crate) fn move_endpoint(lines: &[String], (line, column): (usize, usize), key: &str) -> (usize, usize) {
  if lines.is_empty() {
    return (0, 0);
  }
  let line = line.min(lines.len() - 1);
  let boundaries = |line: usize| {
    let mut positions = vec![0];
    let mut position = 0;
    for character in lines[line].chars() {
      position += character.width().unwrap_or(0);
      if positions.last().copied() != Some(position) {
        positions.push(position);
      }
    }
    positions
  };
  match key {
    "up" | "down" => {
      let target = if key == "up" { line.saturating_sub(1) } else { (line + 1).min(lines.len() - 1) };
      let positions = boundaries(target);
      (target, positions.iter().copied().take_while(|&position| position <= column).last().unwrap_or(0))
    }
    "left" => boundaries(line).into_iter().take_while(|&position| position < column).last().map(|position| (line, position)).unwrap_or_else(|| {
      if line == 0 {
        (0, 0)
      } else {
        (line - 1, *boundaries(line - 1).last().unwrap_or(&0))
      }
    }),
    "right" => boundaries(line).into_iter().find(|&position| position > column).map(|position| (line, position)).unwrap_or_else(|| {
      if line + 1 < lines.len() {
        (line + 1, 0)
      } else {
        (line, *boundaries(line).last().unwrap_or(&0))
      }
    }),
    _ => (line, column),
  }
}

#[cfg(test)]
#[path = "../tests/selection.rs"]
mod tests;
