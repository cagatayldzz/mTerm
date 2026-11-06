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
  Split { split: Split, first: Box<Node>, second: Box<Node> },
}

impl Node {
  /// Replaces `target` with a split holding it and `new`. Returns whether `target` was found.
  pub fn split(&mut self, target: PaneId, new: PaneId, split: Split) -> bool {
    match self {
      Node::Leaf(id) if *id == target => {
        *self = Node::Split { split, first: Box::new(Node::Leaf(target)), second: Box::new(Node::Leaf(new)) };
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
      Node::Split { split, first, second } => match (first.remove(target), second.remove(target)) {
        (Some(first), Some(second)) => Some(Node::Split { split, first: Box::new(first), second: Box::new(second) }),
        (only, None) | (None, only) => only,
      },
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
