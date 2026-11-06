use super::*;

#[test]
fn icons_are_thinned_and_other_assets_pass_through() {
  let icon = Assets.load("icons/panel-left-close.svg").unwrap().expect("the default icon exists");
  let svg = std::str::from_utf8(&icon).unwrap();
  assert!(svg.contains("stroke-width=\"1\""));
  assert!(!svg.contains("stroke-width=\"2\""));
  // A missing asset is an error or nothing, never a made-up icon.
  assert!(Assets.load("icons/does-not-exist.svg").map_or(true, |asset| asset.is_none()));
}

#[test]
fn only_the_default_stroke_width_is_replaced() {
  assert_eq!(with_stroke_width("<svg stroke-width=\"2\"/>"), "<svg stroke-width=\"1\"/>");
  assert_eq!(with_stroke_width("<svg stroke-width=\"3\"/>"), "<svg stroke-width=\"3\"/>");
}
