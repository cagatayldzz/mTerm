use super::*;

#[test]
fn splitting_a_pane_adds_a_sibling() {
  let mut root = Node::Leaf(1);
  assert!(root.split(1, 2, Split::Vertically));
  assert!(root.split(2, 3, Split::Horizontally));
  assert_eq!(root.leaves(), vec![1, 2, 3]);
  assert!(!root.split(9, 4, Split::Vertically));
}

#[test]
fn removing_a_pane_gives_its_space_to_the_sibling() {
  let mut root = Node::Leaf(1);
  root.split(1, 2, Split::Vertically);
  root.split(2, 3, Split::Horizontally);
  let root = root.remove(2).unwrap();
  assert_eq!(root.leaves(), vec![1, 3]);
  let root = root.remove(1).unwrap();
  assert_eq!(root, Node::Leaf(3));
  assert_eq!(root.remove(3), None);
}

#[test]
fn removing_an_unknown_pane_changes_nothing() {
  let mut root = Node::Leaf(1);
  root.split(1, 2, Split::Vertically);
  assert_eq!(root.clone().remove(9), Some(root));
}

#[test]
fn split_ratios_round_trip_and_follow_nested_paths() {
  let mut root = Node::Leaf(0);
  root.split(0, 1, Split::Vertically);
  root.split(1, 2, Split::Horizontally);
  assert!(root.set_ratio(&[], 6200));
  assert!(root.set_ratio(&[true], 3500));
  assert!(!root.set_ratio(&[false], 5000));
  let restored = Node::decode(&root.encode()).unwrap();
  assert_eq!(restored, root);
  assert!(Node::decode("V99999(L,L)").is_none());
  assert!(Node::decode("V5000(L,L)junk").is_none());
}
