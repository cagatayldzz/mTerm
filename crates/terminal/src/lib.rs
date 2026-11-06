//! A shell session: the PTY, terminal emulation (via `alacritty_terminal`) and
//! key encoding. Nothing here depends on the interface; the UI reads
//! [`Snapshot`]s and writes bytes produced by [`encode_key`].

mod keys;
mod palette;
mod session;
mod snapshot;

pub use keys::{dropped_paths, encode_key, paste_bytes, Modifiers};
pub use palette::{Palette, Rgb};
pub use session::{Modes, Session, SessionEvent, SessionOptions};
pub use snapshot::{Cursor, Snapshot, Span};
