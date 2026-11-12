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
  let general = SettingPage::new(t("settings.general")).icon(IconName::Settings).description(t("settings.general_description")).group(
    SettingGroup::new()
      .title(t("settings.appearance"))
      .item(
        SettingItem::new(
          t("settings.theme"),
          SettingField::dropdown(
            theme_options(),
            move |cx| theme_get.read(cx).theme().code().into(),
            move |value, cx| {
              let choice = ThemeChoice::from_code(value.as_ref());
              theme_set.update(cx, |model, cx| model.set_theme(choice, cx));
            },
          ),
        )
        .description(t("settings.theme_description")),
      )
      .item(
        SettingItem::new(
          t("settings.font"),
          SettingField::dropdown(
            FontChoice::ALL
              .map(|choice| {
                (
                  choice.code().into(),
                  match choice {
                    FontChoice::Default => t("settings.font_default"),
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
        .description(t("settings.font_description")),
      )
      .item(
        SettingItem::new(
          t("settings.font_size"),
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
        .description(t("settings.font_size_description")),
      )
      .item(
        SettingItem::new(
          t("settings.line_height"),
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
        .description(t("settings.line_height_description")),
      )
      .item(
        SettingItem::new(
          t("settings.language"),
          SettingField::dropdown(
            language_options(),
            move |cx| language_get.read(cx).preferences.language.map_or("system", Language::code).into(),
            move |value, cx| {
              let choice = Language::from_code(value.as_ref());
              language_set.update(cx, |model, cx| model.set_language(choice, cx));
            },
          ),
        )
        .description(t("settings.language_description")),
      ),
  );

  let (check_get, check_set, panel_model) = (model.clone(), model.clone(), model.clone());
  let updates = SettingPage::new(t("updates.title"))
    .icon(IconName::ArrowDown)
    .description(t("updates.description"))
    .group(SettingGroup::new().title("mTerm").item(SettingItem::render(move |_, _, cx| update_panel(&panel_model, cx))))
    .group(
      SettingGroup::new().title(t("updates.automatic")).item(
        SettingItem::new(
          t("updates.check_on_launch"),
          SettingField::switch(
            move |cx| check_get.read(cx).preferences.check_updates,
            move |value, cx| check_set.update(cx, |model, cx| model.set_check_updates(value, cx)),
          ),
        )
        .description(t("updates.privacy")),
      ),
    );

  let about_foreground = theme.foreground;
  let about = SettingPage::new(t("about.title"))
    .icon(IconName::Info)
    .description(t("about.description"))
    .group(SettingGroup::new().title("mTerm").item(SettingItem::render(move |_, _, _| {
      div().w_full().text_sm().font_weight(FontWeight::NORMAL).text_color(about_foreground).child(
        DescriptionList::new()
          .columns(1)
          .bordered(false)
          .item(t("about.version"), APP_VERSION, 1)
          .item(t("about.repository"), mterm_updater::repository_slug().unwrap_or("—"), 1)
          .item(t("about.data_folder"), mterm_storage::data_dir().display().to_string(), 1)
          .item(t("about.log_folder"), mterm_storage::Diagnostics::default().dir().display().to_string(), 1),
      )
    })))
    .group(SettingGroup::new().title(t("about.support")).description(t("about.report_description")).item(SettingItem::render(|_, _, _| {
      h_flex()
        .gap_3()
        .child(
          Button::new("report-issue")
            .label(t("about.report_issue"))
            .small()
            .on_click(|_, window, cx| window.dispatch_action(Box::new(ReportIssue), cx)),
        )
        .child(Button::new("show-logs").label(t("menu.show_logs")).small().on_click(|_, window, cx| window.dispatch_action(Box::new(ShowLogs), cx)))
    })));

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
  std::iter::once(("system".into(), t("settings.system_default").into()))
    .chain(Language::ALL.map(|language| (language.code().into(), language.native_name().into())))
    .collect()
}

fn theme_options() -> Vec<(SharedString, SharedString)> {
  ThemeChoice::ALL
    .map(|choice| {
      let label = match choice {
        ThemeChoice::System => t("settings.system_default"),
        ThemeChoice::Light => t("theme.light"),
        ThemeChoice::Dark => t("theme.dark"),
        ThemeChoice::SlateDark => t("theme.slate_dark"),
        ThemeChoice::SlateLight => t("theme.slate_light"),
        ThemeChoice::GithubDark => t("theme.github_dark"),
        ThemeChoice::GithubLight => t("theme.github_light"),
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
    return caption(t("updates.development_build"), cx).into_any_element();
  }
  match update {
    AppUpdate::Idle => row()
      .child(Tag::secondary().small().child(t("updates.not_checked")))
      .child(action("check-update", t("updates.check"), false, AppModel::check_update))
      .into_any_element(),
    AppUpdate::Checking => row().child(Spinner::new().small()).child(t("updates.checking")).into_any_element(),
    AppUpdate::UpToDate => row()
      .child(Tag::success().small().child(t("updates.up_to_date")))
      .child(caption(
        match checked {
          Some(time) => tf("updates.current_version_checked", &[("version", APP_VERSION), ("time", &time)]),
          None => tf("updates.current_version", &[("version", APP_VERSION)]),
        },
        cx,
      ))
      .child(div().flex_1())
      .child(action("check-update", t("updates.check_again"), false, AppModel::check_update))
      .into_any_element(),
    AppUpdate::Available(release) => v_flex()
      .gap_3()
      .child(
        row()
          .child(Tag::warning().small().child(tf("updates.version_available", &[("version", &release.version)])))
          .child(caption(tf("updates.you_have", &[("version", APP_VERSION)]), cx))
          .child(div().flex_1())
          .when(!release.page_url.is_empty(), |this| {
            let url = release.page_url.clone();
            this.child(Button::new("release-page").ghost().small().label(t("updates.view_on_github")).on_click(move |_, _, cx| cx.open_url(&url)))
          })
          .child(action("download-update", t("updates.download"), true, AppModel::download_update)),
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
        Stage::Downloading => tf("updates.downloading", &[("version", &release.version)]),
        Stage::Verifying => t("updates.verifying").to_string(),
      })
      .into_any_element(),
    AppUpdate::Ready(release) => row()
      .child(Tag::success().small().child(tf("updates.ready", &[("version", &release.version)])))
      .child(div().flex_1())
      .child(action("restart-update", t("updates.restart"), true, AppModel::restart_to_update).disabled(false))
      .into_any_element(),
    AppUpdate::Failed(message) => v_flex()
      .gap_3()
      .child(Alert::error("update-error", message).title(t("updates.failed")))
      .child(row().child(action("check-update", t("updates.try_again"), false, AppModel::check_update)))
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
