use mterm_storage::{FontChoice, Preferences, ThemeChoice};

#[test]
fn preferences_have_sensible_defaults() {
  let dir = std::env::temp_dir().join(format!("mterm-prefs-default-{}", std::process::id()));
  let preferences = Preferences::load(&dir.join("missing.txt"));
  assert!(preferences.check_updates);
  assert_eq!(preferences.theme, ThemeChoice::System);
  assert_eq!((preferences.terminal_font_size, preferences.terminal_line_height), (12, 16));
}

#[test]
fn preferences_round_trip() {
  let dir = std::env::temp_dir().join(format!("mterm-prefs-{}", std::process::id()));
  let path = dir.join("preferences.txt");
  let saved =
    Preferences { check_updates: false, theme: ThemeChoice::SlateDark, font: FontChoice::Geist, terminal_font_size: 18, terminal_line_height: 24 };
  saved.save(&path).unwrap();
  assert_eq!(Preferences::load(&path), saved);
  std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn unknown_or_invalid_values_keep_the_defaults() {
  let dir = std::env::temp_dir().join(format!("mterm-prefs-invalid-{}", std::process::id()));
  let path = dir.join("preferences.txt");
  std::fs::create_dir_all(&dir).unwrap();
  std::fs::write(&path, "check_updates=maybe\ncheck_updates=false\ntheme=neon\nunknown=1\nnot a pair\n").unwrap();
  let preferences = Preferences::load(&path);
  assert!(!preferences.check_updates);
  assert_eq!(preferences.theme, ThemeChoice::System);
  // An older file that still lists the removed sidebar setting loads with the other values intact.
  std::fs::write(&path, "sidebar_open=false\nterminal_font_size=14\n").unwrap();
  let old = Preferences::load(&path);
  assert!(old.check_updates);
  assert_eq!(old.terminal_font_size, 14);
  std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn theme_codes_round_trip() {
  for choice in ThemeChoice::ALL {
    assert_eq!(ThemeChoice::from_code(choice.code()), choice);
  }
}

#[test]
fn legacy_slate_code_uses_slate_dark() {
  assert_eq!(ThemeChoice::from_code("slate"), ThemeChoice::SlateDark);
}

#[test]
fn font_codes_round_trip_and_unknown_fonts_use_default() {
  for choice in FontChoice::ALL {
    assert_eq!(FontChoice::from_code(choice.code()), choice);
  }
  assert_eq!(FontChoice::from_code("unknown"), FontChoice::Default);
}
