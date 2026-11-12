//! Interface translations. Each language is one flat `key: text` file in `locales/` (`ja.json`);
//! `t("menu.copy")` returns the text of the current language, or the English text when that
//! language has no entry for the key.

use mterm_storage::Language;
use std::{
  collections::HashMap,
  process::Command,
  sync::{
    atomic::{AtomicUsize, Ordering},
    OnceLock,
  },
};

static CURRENT: AtomicUsize = AtomicUsize::new(0);

type Table = HashMap<String, String>;

/// The raw `locales/<code>.json` file of `language`.
pub fn source(language: Language) -> &'static str {
  match language {
    Language::English => include_str!("../../locales/en.json"),
    Language::Chinese => include_str!("../../locales/zh.json"),
    Language::Spanish => include_str!("../../locales/es.json"),
    Language::German => include_str!("../../locales/de.json"),
    Language::French => include_str!("../../locales/fr.json"),
    Language::Japanese => include_str!("../../locales/ja.json"),
    Language::Portuguese => include_str!("../../locales/pt.json"),
    Language::Russian => include_str!("../../locales/ru.json"),
    Language::Korean => include_str!("../../locales/ko.json"),
    Language::Turkish => include_str!("../../locales/tr.json"),
  }
}

/// The keys and texts of `language`.
pub fn entries(language: Language) -> Result<Table, serde_json::Error> {
  serde_json::from_str(source(language))
}

fn index(language: Language) -> usize {
  Language::ALL.iter().position(|candidate| *candidate == language).unwrap_or(0)
}

fn table(language: Language) -> &'static Table {
  static TABLES: [OnceLock<Table>; 10] = [const { OnceLock::new() }; 10];
  // The files are checked by the tests, so a broken one only costs its language.
  TABLES[index(language)].get_or_init(|| entries(language).unwrap_or_default())
}

/// The language the interface is shown in.
pub fn language() -> Language {
  Language::ALL[CURRENT.load(Ordering::Relaxed).min(Language::ALL.len() - 1)]
}

pub fn set_language(language: Language) {
  CURRENT.store(index(language), Ordering::Relaxed);
}

/// The chosen language, or the computer's first preferred language that is translated, or English.
pub fn resolve_language(choice: Option<Language>) -> Language {
  choice.unwrap_or_else(system_language)
}

fn system_language() -> Language {
  Command::new("defaults")
    .args(["read", "-g", "AppleLanguages"])
    .output()
    .ok()
    .and_then(|output| String::from_utf8(output.stdout).ok())
    .and_then(|text| Language::from_apple_languages(&text))
    .unwrap_or_default()
}

/// The text for `key` in the current language, else in English, else the key itself.
pub fn t(key: &'static str) -> &'static str {
  table(language()).get(key).or_else(|| table(Language::English).get(key)).map(String::as_str).unwrap_or(key)
}

/// The text for `key` with each `{name}` replaced by its value.
pub fn tf(key: &'static str, values: &[(&str, &str)]) -> String {
  values.iter().fold(t(key).to_string(), |text, (name, value)| text.replace(&format!("{{{name}}}"), value))
}

#[cfg(test)]
#[path = "../../tests/i18n.rs"]
mod tests;
