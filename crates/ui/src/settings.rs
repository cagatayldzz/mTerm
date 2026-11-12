//! Settings, shown inside the window: appearance, updates and about.

use crate::model::{AppModel, AppUpdate, APP_UPDATES_ENABLED, APP_VERSION};
use crate::{
  i18n::{t, tf},
  ReportIssue, ShowLogs,
};
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
use mterm_storage::{FontChoice, Language, ThemeChoice};
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
  let (language_get, language_set) = (model.clone(), model.clone());
  let general = SettingPage::new(t("General")).icon(IconName::Settings).description(t("Appearance and language of mTerm.")).group(
    SettingGroup::new()
      .title(t("Appearance"))
      .item(
        SettingItem::new(
          t("Theme"),
          SettingField::dropdown(
            theme_options(),
            move |cx| theme_get.read(cx).theme().code().into(),
            move |value, cx| {
              let choice = ThemeChoice::from_code(value.as_ref());
              theme_set.update(cx, |model, cx| model.set_theme(choice, cx));
            },
          ),
        )
        .description(t("Choose a theme or follow your computer's appearance.")),
      )
      .item(
        SettingItem::new(
          t("Font"),
          SettingField::dropdown(
            FontChoice::ALL
              .map(|choice| {
                (
                  choice.code().into(),
                  match choice {
                    FontChoice::Default => t("Default"),
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
        .description(t("Choose the interface font. Geist Mono is used in the terminal when Geist is selected.")),
      )
      .item(
        SettingItem::new(
          t("Terminal font size"),
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
        .description(t("Set the text size in terminal panes.")),
      )
      .item(
        SettingItem::new(
          t("Terminal line height"),
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
        .description(t("Set the spacing between terminal lines.")),
      )
      .item(
        SettingItem::new(
          t("Language"),
          SettingField::dropdown(
            language_options(),
            move |cx| language_get.read(cx).preferences.language.map_or("system", Language::code).into(),
            move |value, cx| {
              let choice = Language::from_code(value.as_ref());
              language_set.update(cx, |model, cx| model.set_language(choice, cx));
            },
          ),
        )
        .description(t("Choose the language of menus and settings.")),
      ),
  );

  let (check_get, check_set, panel_model) = (model.clone(), model.clone(), model.clone());
  let updates = SettingPage::new(t("Updates"))
    .icon(IconName::ArrowDown)
    .description(t("Keep mTerm up to date. New versions come from GitHub releases."))
    .group(SettingGroup::new().title("mTerm").item(SettingItem::render(move |_, _, cx| update_panel(&panel_model, cx))))
    .group(
      SettingGroup::new().title(t("Automatic checks")).item(
        SettingItem::new(
          t("Check for updates on launch"),
          SettingField::switch(
            move |cx| check_get.read(cx).preferences.check_updates,
            move |value, cx| check_set.update(cx, |model, cx| model.set_check_updates(value, cx)),
          ),
        )
        .description(t("mTerm only downloads an update when you ask. Every download is verified against its published checksum and code signature.")),
      ),
    );

  let about_foreground = theme.foreground;
  let about = SettingPage::new(t("About"))
    .icon(IconName::Info)
    .description(t("About this mTerm installation."))
    .group(SettingGroup::new().title("mTerm").item(SettingItem::render(move |_, _, _| {
      div().w_full().text_sm().font_weight(FontWeight::NORMAL).text_color(about_foreground).child(
        DescriptionList::new()
          .columns(1)
          .bordered(false)
          .item(t("Version"), APP_VERSION, 1)
          .item(t("Repository"), mterm_updater::repository_slug().unwrap_or("—"), 1)
          .item(t("Data folder"), mterm_storage::data_dir().display().to_string(), 1)
          .item(t("Log folder"), mterm_storage::Diagnostics::default().dir().display().to_string(), 1),
      )
    })))
    .group(
      SettingGroup::new()
        .title(t("Support"))
        .description(t("Opens a GitHub issue with the versions and the end of the log filled in. Nothing is sent until you submit it."))
        .item(SettingItem::render(|_, _, _| {
          h_flex()
            .gap_3()
            .child(
              Button::new("report-issue")
                .label(t("Report an Issue"))
                .small()
                .on_click(|_, window, cx| window.dispatch_action(Box::new(ReportIssue), cx)),
            )
            .child(
              Button::new("show-logs")
                .label(t("Show Logs in Finder"))
                .small()
                .on_click(|_, window, cx| window.dispatch_action(Box::new(ShowLogs), cx)),
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

/// The system default first, then every language under its own name.
fn language_options() -> Vec<(SharedString, SharedString)> {
  std::iter::once(("system".into(), t("System default").into()))
    .chain(Language::ALL.map(|language| (language.code().into(), language.native_name().into())))
    .collect()
}

fn theme_options() -> Vec<(SharedString, SharedString)> {
  ThemeChoice::ALL
    .map(|choice| {
      let label = match choice {
        ThemeChoice::System => t("System default"),
        ThemeChoice::Light => t("Light"),
        ThemeChoice::Dark => t("Dark"),
        ThemeChoice::SlateDark => t("Slate Dark"),
        ThemeChoice::SlateLight => t("Slate Light"),
        ThemeChoice::GithubDark => t("GitHub Dark"),
        ThemeChoice::GithubLight => t("GitHub Light"),
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
    return caption(t("Updates are turned off in development builds. Install a release build of mTerm to update it."), cx).into_any_element();
  }
  match update {
    AppUpdate::Idle => row()
      .child(Tag::secondary().small().child(t("Not checked yet")))
      .child(action("check-update", t("Check for Updates"), false, AppModel::check_update))
      .into_any_element(),
    AppUpdate::Checking => row().child(Spinner::new().small()).child(t("Checking…")).into_any_element(),
    AppUpdate::UpToDate => row()
      .child(Tag::success().small().child(t("Up to date")))
      .child(caption(
        match checked {
          Some(time) => tf("mTerm {version} · checked at {time}", &[("version", APP_VERSION), ("time", &time)]),
          None => tf("mTerm {version}", &[("version", APP_VERSION)]),
        },
        cx,
      ))
      .child(div().flex_1())
      .child(action("check-update", t("Check Again"), false, AppModel::check_update))
      .into_any_element(),
    AppUpdate::Available(release) => v_flex()
      .gap_3()
      .child(
        row()
          .child(Tag::warning().small().child(tf("Version {version} available", &[("version", &release.version)])))
          .child(caption(tf("You have {version}", &[("version", APP_VERSION)]), cx))
          .child(div().flex_1())
          .when(!release.page_url.is_empty(), |this| {
            let url = release.page_url.clone();
            this.child(Button::new("release-page").ghost().small().label(t("View on GitHub")).on_click(move |_, _, cx| cx.open_url(&url)))
          })
          .child(action("download-update", t("Download & Install"), true, AppModel::download_update)),
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
        Stage::Downloading => tf("Downloading mTerm {version}…", &[("version", &release.version)]),
        Stage::Verifying => t("Verifying checksum and signature…").to_string(),
      })
      .into_any_element(),
    AppUpdate::Ready(release) => row()
      .child(Tag::success().small().child(tf("Version {version} is ready", &[("version", &release.version)])))
      .child(div().flex_1())
      .child(action("restart-update", t("Restart to Update"), true, AppModel::restart_to_update).disabled(false))
      .into_any_element(),
    AppUpdate::Failed(message) => v_flex()
      .gap_3()
      .child(Alert::error("update-error", message).title(t("Update failed")))
      .child(row().child(action("check-update", t("Try Again"), false, AppModel::check_update)))
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
