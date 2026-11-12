use mterm_storage::Language;

#[test]
fn every_language_round_trips_through_its_code() {
  for language in Language::ALL {
    assert_eq!(Language::from_code(language.code()), Some(language));
    assert!(!language.native_name().is_empty());
  }
  assert_eq!(Language::from_code("system"), None);
  assert_eq!(Language::from_code("xx"), None);
}

#[test]
fn macos_language_tags_pick_the_translated_language() {
  for (tag, expected) in [
    ("tr-TR", Some(Language::Turkish)),
    ("tr", Some(Language::Turkish)),
    ("en-GB", Some(Language::English)),
    ("zh-Hans-CN", Some(Language::Chinese)),
    ("zh-CN", Some(Language::Chinese)),
    ("zh-Hant-TW", None),
    ("zh-TW", None),
    ("zh-HK", None),
    ("pt-PT", Some(Language::Portuguese)),
    ("pt_BR", Some(Language::Portuguese)),
    ("ES-mx", Some(Language::Spanish)),
    ("ja-JP", Some(Language::Japanese)),
    ("ar-SA", None),
    ("", None),
  ] {
    assert_eq!(Language::from_locale(tag), expected, "{tag}");
  }
}

#[test]
fn the_first_translated_preferred_language_wins() {
  assert_eq!(Language::from_preferred(["ar-SA", "de-DE", "fr-FR"]), Some(Language::German));
  assert_eq!(Language::from_preferred(["ar-SA", "hi-IN"]), None);
}

#[test]
fn the_output_of_defaults_is_parsed_most_preferred_first() {
  assert_eq!(Language::from_apple_languages("(\n    \"tr-TR\",\n    \"en-TR\"\n)\n"), Some(Language::Turkish));
  assert_eq!(Language::from_apple_languages("(\n    \"ar-SA\",\n    \"zh-Hans-CN\"\n)"), Some(Language::Chinese));
  assert_eq!(Language::from_apple_languages("(\n    \"zh-Hant-TW\",\n    en\n)"), Some(Language::English));
  assert_eq!(Language::from_apple_languages("The domain/default pair does not exist"), None);
  assert_eq!(Language::from_apple_languages(""), None);
}
