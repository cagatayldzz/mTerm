/// Identifies one terminal pane within a window.
pub type PaneId = u64;

/// How a pane is divided when it is split.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Split {
  /// Side by side, with a vertical divider (iTerm2's "Split Pane Vertically").
  Vertically,
  /// One above the other, with a horizontal divider ("Split Pane Horizontally").
  Horizontally,
}

/// The arrangement of panes in a tab: a binary tree whose leaves are panes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Node {
  Leaf(PaneId),
  Split { split: Split, ratio: u16, first: Box<Node>, second: Box<Node> },
}

impl Node {
  /// Replaces `target` with a split holding it and `new`. Returns whether `target` was found.
  pub fn split(&mut self, target: PaneId, new: PaneId, split: Split) -> bool {
    match self {
      Node::Leaf(id) if *id == target => {
        *self = Node::Split { split, ratio: 5000, first: Box::new(Node::Leaf(target)), second: Box::new(Node::Leaf(new)) };
        true
      }
      Node::Leaf(_) => false,
      Node::Split { first, second, .. } => first.split(target, new, split) || second.split(target, new, split),
    }
  }

  /// Removes `target`; its sibling takes the space. `None` means the tree is now empty.
  pub fn remove(self, target: PaneId) -> Option<Node> {
    match self {
      Node::Leaf(id) if id == target => None,
      Node::Leaf(_) => Some(self),
      Node::Split { split, ratio, first, second } => match (first.remove(target), second.remove(target)) {
        (Some(first), Some(second)) => Some(Node::Split { split, ratio, first: Box::new(first), second: Box::new(second) }),
        (only, None) | (None, only) => only,
      },
    }
  }

  /// Adjusts the split at a path, where false selects the first child.
  pub fn set_ratio(&mut self, path: &[bool], value: u16) -> bool {
    match (self, path.split_first()) {
      (Node::Split { ratio, .. }, None) => {
        *ratio = value.clamp(1000, 9000);
        true
      }
      (Node::Split { first, second, .. }, Some((next, rest))) => {
        if *next {
          second.set_ratio(rest, value)
        } else {
          first.set_ratio(rest, value)
        }
      }
      _ => false,
    }
  }

  /// A compact, pane-independent description of the layout.
  pub fn encode(&self) -> String {
    match self {
      Node::Leaf(_) => "L".into(),
      Node::Split { split, ratio, first, second } => {
        format!("{}{}({},{})", if *split == Split::Vertically { "V" } else { "H" }, ratio, first.encode(), second.encode())
      }
    }
  }

  pub fn decode(text: &str) -> Option<Self> {
    fn parse(chars: &mut std::iter::Peekable<std::str::Chars<'_>>, next: &mut PaneId, depth: usize) -> Option<Node> {
      if depth > 16 {
        return None;
      }
      let kind = chars.next()?;
      if kind == 'L' {
        let id = *next;
        *next += 1;
        return Some(Node::Leaf(id));
      }
      let split = match kind {
        'V' => Split::Vertically,
        'H' => Split::Horizontally,
        _ => return None,
      };
      let mut digits = String::new();
      while chars.peek().is_some_and(|c| c.is_ascii_digit()) {
        digits.push(chars.next()?);
      }
      let ratio: u16 = digits.parse().ok()?;
      if !(1000..=9000).contains(&ratio) || chars.next()? != '(' {
        return None;
      }
      let first = Box::new(parse(chars, next, depth + 1)?);
      if chars.next()? != ',' {
        return None;
      }
      let second = Box::new(parse(chars, next, depth + 1)?);
      if chars.next()? != ')' {
        return None;
      }
      Some(Node::Split { split, ratio, first, second })
    }
    let mut chars = text.trim().chars().peekable();
    let node = parse(&mut chars, &mut 0, 0)?;
    chars.next().is_none().then_some(node)
  }

  pub fn map_leaves(self, ids: &mut impl Iterator<Item = PaneId>) -> Self {
    match self {
      Node::Leaf(_) => Node::Leaf(ids.next().expect("one id per pane")),
      Node::Split { split, ratio, first, second } => {
        Node::Split { split, ratio, first: Box::new(first.map_leaves(ids)), second: Box::new(second.map_leaves(ids)) }
      }
    }
  }

  /// The panes from first to last (left to right, top to bottom).
  pub fn leaves(&self) -> Vec<PaneId> {
    match self {
      Node::Leaf(id) => vec![*id],
      Node::Split { first, second, .. } => {
        let mut leaves = first.leaves();
        leaves.extend(second.leaves());
        leaves
      }
    }
  }
}

#[cfg(test)]
#[path = "../tests/layout.rs"]
mod tests;
