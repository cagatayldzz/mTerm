/// A language the interface is translated into.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Language {
  #[default]
  English,
  Chinese,
  Spanish,
  German,
  French,
  Japanese,
  Portuguese,
  Russian,
  Korean,
  Turkish,
}

impl Language {
  pub const ALL: [Self; 10] = [
    Self::English,
    Self::Chinese,
    Self::Spanish,
    Self::German,
    Self::French,
    Self::Japanese,
    Self::Portuguese,
    Self::Russian,
    Self::Korean,
    Self::Turkish,
  ];

  /// The stored code, which is also the language tag macOS uses.
  pub fn code(self) -> &'static str {
    match self {
      Self::English => "en",
      Self::Chinese => "zh-Hans",
      Self::Spanish => "es",
      Self::German => "de",
      Self::French => "fr",
      Self::Japanese => "ja",
      Self::Portuguese => "pt-BR",
      Self::Russian => "ru",
      Self::Korean => "ko",
      Self::Turkish => "tr",
    }
  }

  /// The language's own name, as shown in the language list so it can be found in any language.
  pub fn native_name(self) -> &'static str {
    match self {
      Self::English => "English",
      Self::Chinese => "简体中文",
      Self::Spanish => "Español",
      Self::German => "Deutsch",
      Self::French => "Français",
      Self::Japanese => "日本語",
      Self::Portuguese => "Português (Brasil)",
      Self::Russian => "Русский",
      Self::Korean => "한국어",
      Self::Turkish => "Türkçe",
    }
  }

  /// The exact stored code; anything else is `None`.
  pub fn from_code(code: &str) -> Option<Self> {
    Self::ALL.into_iter().find(|language| language.code() == code.trim())
  }

  /// The language for a macOS language tag such as `tr-TR`, `zh-Hans-CN` or `pt-PT`.
  /// Traditional Chinese is not translated, so it falls back to `None` like any other language.
  pub fn from_locale(tag: &str) -> Option<Self> {
    let tag = tag.trim().replace('_', "-").to_ascii_lowercase();
    let mut parts = tag.split('-');
    match parts.next()? {
      "en" => Some(Self::English),
      "zh" => {
        let rest: Vec<_> = parts.collect();
        (!rest.iter().any(|part| matches!(*part, "hant" | "tw" | "hk" | "mo"))).then_some(Self::Chinese)
      }
      "es" => Some(Self::Spanish),
      "de" => Some(Self::German),
      "fr" => Some(Self::French),
      "ja" => Some(Self::Japanese),
      "pt" => Some(Self::Portuguese),
      "ru" => Some(Self::Russian),
      "ko" => Some(Self::Korean),
      "tr" => Some(Self::Turkish),
      _ => None,
    }
  }

  /// The first of the user's preferred languages (most preferred first) that is translated.
  pub fn from_preferred<'a>(tags: impl IntoIterator<Item = &'a str>) -> Option<Self> {
    tags.into_iter().find_map(Self::from_locale)
  }
}

impl Language {
  /// The language for the output of `defaults read -g AppleLanguages`, a list such as
  /// `( "tr-TR", "en-TR" )`, most preferred first.
  pub fn from_apple_languages(output: &str) -> Option<Self> {
    Self::from_preferred(output.split(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_')).filter(|tag| !tag.is_empty()))
  }
}
