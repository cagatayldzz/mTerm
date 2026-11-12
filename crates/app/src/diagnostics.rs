//! Crash reports, the log folder and "Report an Issue": what the user needs when mTerm misbehaves.

use gpui_kit::{
  component::{button::Button, notification::Notification, Sizable as _, WindowExt as _},
  *,
};
use mterm_storage::{issue_url, scrub_home, Diagnostics};
use mterm_ui::{t, ReportIssue, ShowLogs};
use std::{backtrace::Backtrace, panic, path::PathBuf, process::Command, time::Duration};

const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// How long after launch the previous crash is announced, once the windows are up.
const ANNOUNCE_DELAY: Duration = Duration::from_millis(1500);
/// How much of a crash report and of the log goes into the prefilled issue.
const REPORT_LINES: usize = 40;
const LOG_LINES: usize = 20;

fn home() -> PathBuf {
  std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default()
}

/// Writes a crash report to the log folder whenever any thread panics, then lets the normal
/// panic output run. Reports are only stored; sending one is the user's decision.
pub(crate) fn install_panic_hook() {
  let previous = panic::take_hook();
  panic::set_hook(Box::new(move |info| {
    let message = info
      .payload()
      .downcast_ref::<&str>()
      .map(|message| message.to_string())
      .or_else(|| info.payload().downcast_ref::<String>().cloned())
      .unwrap_or_else(|| "(no message)".to_string());
    let location = info.location().map(|location| location.to_string()).unwrap_or_else(|| "unknown location".to_string());
    let thread = std::thread::current().name().unwrap_or("unnamed").to_string();
    let report = format!("{}\nthread '{thread}' panicked at {location}:\n{message}\n\n{}\n", system_summary(), Backtrace::force_capture());
    let _ = Diagnostics::default().write_crash(&scrub_home(&report, &home()));
    previous(info);
  }));
}

/// The mTerm and macOS versions, for reports.
fn system_summary() -> String {
  let macos = Command::new("sw_vers")
    .arg("-productVersion")
    .output()
    .ok()
    .and_then(|output| String::from_utf8(output.stdout).ok())
    .map(|version| version.trim().to_string())
    .filter(|version| !version.is_empty())
    .unwrap_or_else(|| "unknown".to_string());
  format!("mTerm {VERSION}, macOS {macos} ({})", std::env::consts::ARCH)
}

pub(crate) fn log_startup() {
  let _ = Diagnostics::default().log(&format!("started: {}", system_summary()));
}

/// Opens a new GitHub issue with the versions, the last crash and the end of the log filled in.
pub(crate) fn report_issue(cx: &mut App) {
  let diagnostics = Diagnostics::default();
  let mut body = format!(
    "**What happened?**\n\n\n**Environment**\n{}\n\nFull logs: Help → Show Logs in Finder. If mTerm crashed, attach the newest `mTerm-*.ips` file from `~/Library/Logs/DiagnosticReports`.\n",
    system_summary()
  );
  if let Some(crash) = diagnostics.latest_crash() {
    body.push_str(&format!("\n**Last crash report**\n```\n{}\n```\n", first_lines(&crash, REPORT_LINES)));
  }
  let log = diagnostics.log_tail(LOG_LINES);
  if !log.is_empty() {
    body.push_str(&format!("\n**Recent log**\n```\n{}\n```\n", scrub_home(&log, &home())));
  }
  cx.open_url(&issue_url(REPOSITORY, "Bug: ", &body));
}

fn first_lines(text: &str, count: usize) -> String {
  text.lines().take(count).collect::<Vec<_>>().join("\n")
}

/// Opens the folder with the log and crash reports in Finder.
pub(crate) fn show_logs(cx: &mut App) {
  let diagnostics = Diagnostics::default();
  let _ = std::fs::create_dir_all(diagnostics.dir());
  cx.open_with_system(diagnostics.dir());
}

/// Tells the user, once, that the previous run ended in a crash, and offers to report it.
pub(crate) fn announce_previous_crash(cx: &mut App) {
  cx.spawn(async move |cx| {
    cx.background_executor().timer(ANNOUNCE_DELAY).await;
    cx.update(|cx| {
      let Some(handle) = cx.windows().into_iter().next() else { return };
      if Diagnostics::default().take_unannounced_crash().is_none() {
        return;
      }
      let _ = handle.update(cx, |_, window, cx| {
        window.push_notification(
          Notification::warning(t("mTerm quit unexpectedly last time. Nothing was sent anywhere; you decide whether to report it."))
            .title(t("mTerm crashed"))
            .action(|_, _, cx| {
              Button::new("report-crash").label(t("Report an Issue")).small().on_click(cx.listener(|notification, _, window, cx| {
                window.dispatch_action(Box::new(ReportIssue), cx);
                notification.dismiss(window, cx);
              }))
            }),
          cx,
        );
      });
    });
  })
  .detach();
}

/// Registers the actions behind the Help menu and the Settings buttons.
pub(crate) fn register_actions(cx: &mut App) {
  cx.on_action(|_: &ReportIssue, cx| report_issue(cx));
  cx.on_action(|_: &ShowLogs, cx| show_logs(cx));
}
