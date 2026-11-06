use gpui_kit::{
  component::{Theme, ThemeMode},
  *,
};
use mterm_storage::ThemeChoice;

/// Applies the saved appearance to the window: the system's, or a fixed light or dark.
pub(crate) fn apply_theme(choice: ThemeChoice, window: &mut Window, cx: &mut App) {
  match choice {
    ThemeChoice::System => Theme::sync_system_appearance(Some(window), cx),
    ThemeChoice::Light => Theme::change(ThemeMode::Light, Some(window), cx),
    ThemeChoice::Dark => Theme::change(ThemeMode::Dark, Some(window), cx),
  }
  window.refresh();
}
