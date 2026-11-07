use unicode_width::UnicodeWidthChar as _;

pub(crate) fn word_range(text: &str, column: usize) -> Option<(usize, usize)> {
  let mut cells = Vec::new();
  let mut position = 0;
  for character in text.chars() {
    let width = character.width().unwrap_or(0);
    if width > 0 {
      let kind = if character.is_whitespace() {
        0
      } else if character.is_alphanumeric() || character == '_' {
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
