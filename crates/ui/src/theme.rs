use gpui_kit::{
  component::{Theme, ThemeMode},
  *,
};
use mterm_storage::ThemeChoice;

/// Applies the saved appearance to every window.
pub(crate) fn apply_theme(choice: ThemeChoice, window: &mut Window, cx: &mut App) {
  match choice {
    ThemeChoice::System => Theme::sync_system_appearance(Some(window), cx),
    ThemeChoice::Light => Theme::change(ThemeMode::Light, Some(window), cx),
    ThemeChoice::Dark => Theme::change(ThemeMode::Dark, Some(window), cx),
    ThemeChoice::Slate => {
      Theme::change(ThemeMode::Dark, Some(window), cx);
      Theme::update(cx, |theme| {
        let colors = &mut theme.colors;
        colors.background = rgb(0x1e2428).into();
        colors.foreground = rgb(0xe4e8ed).into();
        colors.sidebar = rgb(0x151a1e).into();
        colors.sidebar_foreground = rgb(0xe4e8ed).into();
        colors.sidebar_border = rgb(0x313941).into();
        colors.sidebar_accent = rgb(0x29333f).into();
        colors.sidebar_accent_foreground = rgb(0xe4e8ed).into();
        colors.sidebar_primary = rgb(0x285bc6).into();
        colors.sidebar_primary_foreground = rgb(0xffffff).into();
        colors.border = rgb(0x343c44).into();
        colors.input = rgb(0x3c4650).into();
        colors.popover = rgb(0x242b32).into();
        colors.popover_foreground = rgb(0xe4e8ed).into();
        colors.muted = rgb(0x293139).into();
        colors.muted_foreground = rgb(0x9aa5af).into();
        colors.secondary = rgb(0x2a323a).into();
        colors.secondary_foreground = rgb(0xe4e8ed).into();
        colors.accent = rgb(0x2d3c4e).into();
        colors.accent_foreground = rgb(0xe4e8ed).into();
        colors.primary = rgb(0x2864d7).into();
        colors.primary_foreground = rgb(0xffffff).into();
        colors.ring = rgb(0x488cff).into();
        colors.selection = rgb(0x285bc6).into();
      });
    }
  }
  window.refresh();
}
