use super::*;
use std::collections::{BTreeSet, HashSet};

/// The ui and app sources that call `t(…)` and `tf(…)`.
const SOURCES: [&str; 9] = [
  include_str!("../src/components.rs"),
  include_str!("../src/find.rs"),
  include_str!("../src/settings.rs"),
  include_str!("../src/tabs.rs"),
  include_str!("../src/terminal_view.rs"),
  include_str!("../src/workspace.rs"),
  include_str!("../../app/src/menus.rs"),
  include_str!("../../app/src/diagnostics.rs"),
  include_str!("../../app/src/windows.rs"),
];

fn placeholders(text: &str) -> BTreeSet<String> {
  let mut found = BTreeSet::new();
  let mut rest = text;
  while let Some(start) = rest.find('{') {
    let Some(end) = rest[start..].find('}') else { break };
    found.insert(rest[start..start + end + 1].to_string());
    rest = &rest[start + end + 1..];
  }
  found
}

/// Every string literal passed straight to `t(` or `tf(` in the sources.
fn used_keys() -> HashSet<String> {
  let mut keys = HashSet::new();
  for source in SOURCES {
    for marker in ["t(\"", "tf(\""] {
      for (position, _) in source.match_indices(marker) {
        let before = source[..position].chars().next_back();
        if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
          continue;
        }
        let rest = &source[position + marker.len()..];
        let mut key = String::new();
        let mut chars = rest.chars();
        while let Some(c) = chars.next() {
          match c {
            '"' => break,
            '\\' => key.push(chars.next().unwrap_or('\\')),
            c => key.push(c),
          }
        }
        keys.insert(key);
      }
    }
  }
  keys
}

fn load(language: Language) -> Table {
  entries(language).unwrap_or_else(|error| panic!("{language:?}: locales file is not a flat key: text object: {error}"))
}

fn english() -> Table {
  load(Language::English)
}

fn others() -> impl Iterator<Item = Language> {
  Language::ALL.into_iter().filter(|language| *language != Language::English)
}

#[test]
fn every_locale_file_has_unique_keys() {
  for language in Language::ALL {
    // One entry per line, so a repeated key shows up as fewer map entries than lines.
    let lines = source(language).lines().filter(|line| line.starts_with("  \"")).count();
    assert_eq!(load(language).len(), lines, "{language:?} has a duplicate key");
  }
}

#[test]
fn every_language_has_exactly_the_english_keys() {
  let english: HashSet<_> = english().into_keys().collect();
  for language in others() {
    let keys: HashSet<_> = load(language).into_keys().collect();
    let missing: Vec<_> = english.difference(&keys).collect();
    let extra: Vec<_> = keys.difference(&english).collect();
    assert!(missing.is_empty() && extra.is_empty(), "{language:?}: missing {missing:?}, unknown {extra:?}");
  }
}

#[test]
fn translations_keep_the_placeholders_and_are_not_empty() {
  let english = english();
  for language in others() {
    for (key, text) in load(language) {
      assert!(!text.trim().is_empty(), "{language:?}: {key:?} is empty");
      assert_eq!(placeholders(&english[&key]), placeholders(&text), "{language:?}: {key:?} -> {text:?}");
    }
  }
}

#[test]
fn translations_are_translations_not_copies_of_the_english() {
  let english = english();
  for language in others() {
    let same = load(language).iter().filter(|(key, text)| english[*key] == **text).count();
    // Names such as "Zoom" or "mTerm {version}" can stay the same in some languages.
    assert!(same * 4 < english.len(), "{language:?} leaves {same} of {} texts in English", english.len());
  }
}

#[test]
fn every_key_the_interface_uses_exists_and_none_is_unused() {
  let used = used_keys();
  let known: HashSet<String> = english().into_keys().collect();
  let mut unknown: Vec<_> = used.difference(&known).collect();
  unknown.sort();
  assert!(unknown.is_empty(), "t(…) is called with keys that no locale file has: {unknown:?}");
  let mut unused: Vec<_> = known.difference(&used).collect();
  unused.sort();
  assert!(unused.is_empty(), "keys the interface never uses: {unused:?}");
}

/// One test owns the process-wide language, so parallel tests cannot see it change.
#[test]
fn the_current_language_decides_the_text() {
  set_language(Language::Turkish);
  assert_eq!(language(), Language::Turkish);
  assert_eq!(t("menu.copy"), "Kopyala");
  assert_eq!(tf("find.position", &[("index", "2"), ("count", "9")]), "2 / 9");
  assert_eq!(tf("error.shell_start", &[("error", "boom")]), "Kabuk başlatılamadı: boom");

  set_language(Language::Chinese);
  assert_eq!(t("menu.copy"), "拷贝");
  assert_eq!(t("no.such.key"), "no.such.key", "an unknown key shows the key");

  set_language(Language::English);
  assert_eq!(t("menu.copy"), "Copy");
  assert_eq!(tf("tab.copy_title", &[("title", "build")]), "build copy");
}

#[test]
fn a_chosen_language_wins_over_the_system_language() {
  assert_eq!(resolve_language(Some(Language::Korean)), Language::Korean);
}
