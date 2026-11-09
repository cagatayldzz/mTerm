#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

//! The mTerm executable; the application lives in `mterm-app`.

fn main() {
  mterm_app::run();
}
