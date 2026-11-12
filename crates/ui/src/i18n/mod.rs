//! Interface translations. English text is the key: `t("Copy")` returns the text of the current
//! language, or the English original when a language has no entry for it.

mod chinese;
mod english;
mod french;
mod german;
mod japanese;
mod korean;
mod portuguese;
mod russian;
mod spanish;
mod turkish;

use mterm_storage::Language;
use std::{
  collections::HashMap,
  process::Command,
  sync::{
    atomic::{AtomicUsize, Ordering},
    OnceLock,
  },
};

/// Every English text that can be translated, in the order it appears in the interface.
pub use english::KEYS;

static CURRENT: AtomicUsize = AtomicUsize::new(0);

/// The entries of `language`: pairs of the English text and its translation.
pub fn entries(language: Language) -> &'static [(&'static str, &'static str)] {
  match language {
    Language::English => &[],
    Language::Chinese => chinese::STRINGS,
    Language::Spanish => spanish::STRINGS,
    Language::German => german::STRINGS,
    Language::French => french::STRINGS,
    Language::Japanese => japanese::STRINGS,
    Language::Portuguese => portuguese::STRINGS,
    Language::Russian => russian::STRINGS,
    Language::Korean => korean::STRINGS,
    Language::Turkish => turkish::STRINGS,
  }
}

fn index(language: Language) -> usize {
  Language::ALL.iter().position(|candidate| *candidate == language).unwrap_or(0)
}

fn table(language: Language) -> &'static HashMap<&'static str, &'static str> {
  static TABLES: [OnceLock<HashMap<&'static str, &'static str>>; 10] = [const { OnceLock::new() }; 10];
  TABLES[index(language)].get_or_init(|| entries(language).iter().copied().collect())
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

/// The text in the current language.
pub fn t(english: &'static str) -> &'static str {
  match language() {
    Language::English => english,
    other => table(other).get(english).copied().unwrap_or(english),
  }
}

/// The text in the current language with each `{name}` replaced by its value.
pub fn tf(english: &'static str, values: &[(&str, &str)]) -> String {
  values.iter().fold(t(english).to_string(), |text, (name, value)| text.replace(&format!("{{{name}}}"), value))
}

#[cfg(test)]
#[path = "../../tests/i18n.rs"]
mod tests;
