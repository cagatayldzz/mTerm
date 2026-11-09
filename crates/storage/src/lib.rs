//! Persistent state for mTerm, stored under `~/Library/Application Support/mTerm`.

mod dropped_files;
mod history;
mod paths;
mod preferences;
mod state;
mod window_size;

pub use dropped_files::prepare_dropped_paths;
pub use history::History;
pub use paths::{data_dir, directory_from_url};
pub use preferences::{FontChoice, Preferences, ThemeChoice};
pub use state::{SavedState, SavedTab, SavedWindow};
pub use window_size::WindowSize;
