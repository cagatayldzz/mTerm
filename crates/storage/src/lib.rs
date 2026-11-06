//! Persistent state for mTerm, stored under `~/Library/Application Support/mTerm`.

mod history;
mod paths;
mod preferences;
mod window_size;

pub use history::History;
pub use paths::{data_dir, directory_from_url};
pub use preferences::Preferences;
pub use window_size::WindowSize;
