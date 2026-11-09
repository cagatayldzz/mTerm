//! Settings, shown inside the window: appearance, updates and about.

use crate::model::{AppModel, AppUpdate, APP_UPDATES_ENABLED, APP_VERSION};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
  component::{
    alert::Alert,
    button::{Button, ButtonVariants},
    description_list::DescriptionList,
    h_flex,
    setting::{SelectIndex, SettingField, SettingGroup, SettingItem, SettingPage, Settings},
    spinner::Spinner,
    tag::Tag,
    v_flex, ActiveTheme, Disableable, IconName, Sizable,
  },
  *,
};
use mterm_storage::{FontChoice, ThemeChoice};
use mterm_updater::Stage;

/// The pages of Settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsPage {
  General,
  Updates,
}

impl SettingsPage {
  fn index(self) -> usize {
    match self {
      Self::General => 0,
      Self::Updates => 1,
    }
  }
}

/// The width of the Settings menu, which the component takes in pixels.
const SIDEBAR_WIDTH: Pixels = px(180.);

/// The Settings panel. `epoch` changes whenever it is opened, so the
/// component starts on `page` again instead of keeping its own selection.
pub(crate) fn render_settings(model: &Entity<AppModel>, page: SettingsPage, epoch: u64, cx: &App) -> impl IntoElement {
  let theme = cx.theme().clone();
  let model = model.clone();

  let (theme_get, theme_set) = (model.clone(), model.clone());
  let (font_get, font_set) = (model.clone(), model.clone());
  let (size_get, size_set) = (model.clone(), model.clone());
  let (height_get, height_set) = (model.clone(), model.clone());
  let general = SettingPage::new("General").icon(IconName::Settings).description("How mTerm looks.").group(
    SettingGroup::new()
      .title("Appearance")
      .item(
        SettingItem::new(
          "Theme",
          SettingField::dropdown(
            theme_options(),
            move |cx| theme_get.read(cx).theme().code().into(),
            move |value, cx| {
              let choice = ThemeChoice::from_code(value.as_ref());
              theme_set.update(cx, |model, cx| model.set_theme(choice, cx));
            },
          ),
        )
        .description("Choose a theme or follow your computer's appearance."),
      )
      .item(
        SettingItem::new(
          "Font",
          SettingField::dropdown(
            FontChoice::ALL
              .map(|choice| {
                (
                  choice.code().into(),
                  match choice {
                    FontChoice::Default => "Default",
                    FontChoice::Geist => "Geist",
                    FontChoice::Inter => "Inter",
                  }
                  .into(),
                )
              })
              .to_vec(),
            move |cx| font_get.read(cx).preferences.font.code().into(),
            move |value, cx| font_set.update(cx, |model, cx| model.set_font(FontChoice::from_code(value.as_ref()), cx)),
          ),
        )
        .description("Choose the interface font. Geist Mono is used in the terminal when Geist is selected."),
      )
      .item(
        SettingItem::new(
          "Terminal font size",
          SettingField::dropdown(
            (9..=24).map(|value| (value.to_string().into(), format!("{value} px").into())).collect(),
            move |cx| size_get.read(cx).preferences.terminal_font_size.to_string().into(),
            move |value, cx| {
              if let Ok(size) = value.parse::<u8>() {
                size_set.update(cx, |model, cx| model.set_terminal_font_size(size, cx));
              }
            },
          ),
        )
        .description("Set the text size in terminal panes."),
      )
      .item(
        SettingItem::new(
          "Terminal line height",
          SettingField::dropdown(
            (12..=36).map(|value| (value.to_string().into(), format!("{value} px").into())).collect(),
            move |cx| height_get.read(cx).preferences.terminal_line_height.to_string().into(),
            move |value, cx| {
              if let Ok(height) = value.parse::<u8>() {
                height_set.update(cx, |model, cx| model.set_terminal_line_height(height, cx));
              }
            },
          ),
        )
        .description("Set the spacing between terminal lines."),
      ),
  );

  let (check_get, check_set, panel_model) = (model.clone(), model.clone(), model.clone());
  let updates = SettingPage::new("Updates")
    .icon(IconName::ArrowDown)
    .description("Keep mTerm up to date. New versions come from GitHub releases.")
    .group(SettingGroup::new().title("mTerm").item(SettingItem::render(move |_, _, cx| update_panel(&panel_model, cx))))
    .group(
      SettingGroup::new().title("Automatic checks").item(
        SettingItem::new(
          "Check for updates on launch",
          SettingField::switch(
            move |cx| check_get.read(cx).preferences.check_updates,
            move |value, cx| check_set.update(cx, |model, cx| model.set_check_updates(value, cx)),
          ),
        )
        .description("mTerm only downloads an update when you ask. Every download is verified against its published checksum and code signature."),
      ),
    );

  let about_foreground = theme.foreground;
  let about = SettingPage::new("About").icon(IconName::Info).description("About this mTerm installation.").group(
    SettingGroup::new().title("mTerm").item(SettingItem::render(move |_, _, _| {
      div().w_full().text_sm().font_weight(FontWeight::NORMAL).text_color(about_foreground).child(
        DescriptionList::new()
          .columns(1)
          .bordered(false)
          .item("Version", APP_VERSION, 1)
          .item("Repository", mterm_updater::repository_slug().unwrap_or("—"), 1)
          .item("Data folder", mterm_storage::data_dir().display().to_string(), 1),
      )
    })),
  );

  // The menu and its entries show the arrow; controls with their own cursor (the search field) keep it.
  div().id("settings").size_full().cursor_default().bg(theme.background).child(
    Settings::new(SharedString::from(format!("settings-{epoch}")))
      .sidebar_width(SIDEBAR_WIDTH)
      .default_selected_index(SelectIndex { page_ix: page.index(), group_ix: None })
      .pages(vec![general, updates, about]),
  )
}

fn theme_options() -> Vec<(SharedString, SharedString)> {
  ThemeChoice::ALL
    .map(|choice| {
      let label = match choice {
        ThemeChoice::System => "System default",
        ThemeChoice::Light => "Light",
        ThemeChoice::Dark => "Dark",
        ThemeChoice::SlateDark => "Slate Dark",
        ThemeChoice::SlateLight => "Slate Light",
        ThemeChoice::GithubDark => "GitHub Dark",
        ThemeChoice::GithubLight => "GitHub Light",
      };
      (choice.code().into(), label.into())
    })
    .to_vec()
}

fn caption(text: impl Into<SharedString>, cx: &App) -> Div {
  div().text_sm().text_color(cx.theme().muted_foreground).child(text.into())
}

/// The mTerm update status and its actions.
fn update_panel(model: &Entity<AppModel>, cx: &App) -> AnyElement {
  let theme = cx.theme();
  let state = model.read(cx);
  let update = state.update.clone();
  let checked = state.update_checked.map(local_time);
  let action = |id: &'static str, label: &'static str, primary: bool, run: fn(&mut AppModel, &mut Context<AppModel>)| {
    let model = model.clone();
    let button = Button::new(id).label(label).small().on_click(move |_, _, cx| model.update(cx, run));
    if primary {
      button.primary()
    } else {
      button
    }
  };
  let row = || h_flex().gap_3().items_center().flex_wrap().text_sm().text_color(theme.foreground);

  if !APP_UPDATES_ENABLED {
    return caption("Updates are turned off in development builds. Install a release build of mTerm to update it.", cx).into_any_element();
  }
  match update {
    AppUpdate::Idle => row()
      .child(Tag::secondary().small().child("Not checked yet"))
      .child(action("check-update", "Check for Updates", false, AppModel::check_update))
      .into_any_element(),
    AppUpdate::Checking => row().child(Spinner::new().small()).child("Checking…").into_any_element(),
    AppUpdate::UpToDate => row()
      .child(Tag::success().small().child("Up to date"))
      .child(caption(format!("mTerm {APP_VERSION}{}", checked.map(|time| format!(" · checked at {time}")).unwrap_or_default()), cx))
      .child(div().flex_1())
      .child(action("check-update", "Check Again", false, AppModel::check_update))
      .into_any_element(),
    AppUpdate::Available(release) => v_flex()
      .gap_3()
      .child(
        row()
          .child(Tag::warning().small().child(format!("Version {} available", release.version)))
          .child(caption(format!("You have {APP_VERSION}"), cx))
          .child(div().flex_1())
          .when(!release.page_url.is_empty(), |this| {
            let url = release.page_url.clone();
            this.child(Button::new("release-page").ghost().small().label("View on GitHub").on_click(move |_, _, cx| cx.open_url(&url)))
          })
          .child(action("download-update", "Download & Install", true, AppModel::download_update)),
      )
      .when(!release.plain_notes().is_empty(), |this| {
        this.child(
          div()
            .p_3()
            .rounded_lg()
            .bg(theme.foreground.opacity(0.04))
            .text_sm()
            .text_color(theme.muted_foreground)
            .max_h_48()
            .overflow_hidden()
            .child(release.plain_notes()),
        )
      })
      .into_any_element(),
    AppUpdate::Working(release, stage) => row()
      .child(Spinner::new().small())
      .child(match stage {
        Stage::Downloading => format!("Downloading mTerm {}…", release.version),
        Stage::Verifying => "Verifying checksum and signature…".to_string(),
      })
      .into_any_element(),
    AppUpdate::Ready(release) => row()
      .child(Tag::success().small().child(format!("Version {} is ready", release.version)))
      .child(div().flex_1())
      .child(action("restart-update", "Restart to Update", true, AppModel::restart_to_update).disabled(false))
      .into_any_element(),
    AppUpdate::Failed(message) => v_flex()
      .gap_3()
      .child(Alert::error("update-error", message).title("Update failed"))
      .child(row().child(action("check-update", "Try Again", false, AppModel::check_update)))
      .into_any_element(),
  }
}

/// The time of day, e.g. `14:05`.
fn local_time(time: std::time::SystemTime) -> String {
  let seconds = time.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
  let minutes = seconds / 60 % 60;
  let hours = seconds / 3600 % 24;
  format!("{hours:02}:{minutes:02} UTC")
}
