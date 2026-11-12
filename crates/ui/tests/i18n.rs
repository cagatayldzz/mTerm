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

#[test]
fn english_keys_are_unique() {
  let unique: HashSet<_> = KEYS.iter().collect();
  assert_eq!(unique.len(), KEYS.len());
}

#[test]
fn every_language_translates_exactly_the_english_keys() {
  let english: HashSet<_> = KEYS.iter().copied().collect();
  for language in Language::ALL.into_iter().filter(|language| *language != Language::English) {
    let entries = entries(language);
    let keys: HashSet<_> = entries.iter().map(|(key, _)| *key).collect();
    assert_eq!(keys.len(), entries.len(), "{language:?} has a duplicate key");
    let missing: Vec<_> = english.difference(&keys).collect();
    let extra: Vec<_> = keys.difference(&english).collect();
    assert!(missing.is_empty() && extra.is_empty(), "{language:?}: missing {missing:?}, unknown {extra:?}");
  }
}

#[test]
fn translations_keep_the_placeholders_and_are_not_empty() {
  for language in Language::ALL.into_iter().filter(|language| *language != Language::English) {
    for (key, translation) in entries(language) {
      assert!(!translation.trim().is_empty(), "{language:?}: {key:?} is empty");
      assert_eq!(placeholders(key), placeholders(translation), "{language:?}: {key:?} -> {translation:?}");
    }
  }
}

#[test]
fn translations_are_translations_not_copies_of_the_english() {
  for language in Language::ALL.into_iter().filter(|language| *language != Language::English) {
    let same = entries(language).iter().filter(|(key, translation)| key == translation).count();
    // Names such as "Zoom" or "mTerm {version}" can stay the same in some languages.
    assert!(same * 4 < KEYS.len(), "{language:?} leaves {same} of {} texts in English", KEYS.len());
  }
}

#[test]
fn every_text_the_interface_translates_has_a_translation_and_none_is_unused() {
  let used = used_keys();
  let known: HashSet<String> = KEYS.iter().map(|key| key.to_string()).collect();
  let mut unknown: Vec<_> = used.difference(&known).collect();
  unknown.sort();
  assert!(unknown.is_empty(), "t(…) is called with texts that have no translations: {unknown:?}");
  let mut unused: Vec<_> = known.difference(&used).collect();
  unused.sort();
  assert!(unused.is_empty(), "translated texts the interface never uses: {unused:?}");
}

/// One test owns the process-wide language, so parallel tests cannot see it change.
#[test]
fn the_current_language_decides_the_text() {
  set_language(Language::Turkish);
  assert_eq!(language(), Language::Turkish);
  assert_eq!(t("Copy"), "Kopyala");
  assert_eq!(tf("{index} of {count}", &[("index", "2"), ("count", "9")]), "2 / 9");
  assert_eq!(tf("Could not start the shell: {error}", &[("error", "boom")]), "Kabuk başlatılamadı: boom");

  set_language(Language::Chinese);
  assert_eq!(t("Copy"), "拷贝");
  assert_eq!(t("This text has no translation"), "This text has no translation", "unknown text falls back to English");

  set_language(Language::English);
  assert_eq!(t("Copy"), "Copy");
  assert_eq!(tf("{title} copy", &[("title", "build")]), "build copy");
}

#[test]
fn a_chosen_language_wins_over_the_system_language() {
  assert_eq!(resolve_language(Some(Language::Korean)), Language::Korean);
}
