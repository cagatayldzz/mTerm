//! Persistent state for mTerm, stored under `~/Library/Application Support/mTerm`.

mod diagnostics;
mod dropped_files;
mod history;
mod paths;
mod preferences;
mod state;
mod window_size;

pub use diagnostics::{issue_url, scrub_home, Diagnostics};
pub use dropped_files::{open_dropped_paths, prepare_dropped_paths, DroppedPaths};
pub use history::History;
pub use paths::{data_dir, directory_from_url};
pub use preferences::{FontChoice, Preferences, ThemeChoice};
pub use state::{SavedBounds, SavedState, SavedTab, SavedWindow};
pub use window_size::WindowSize;
