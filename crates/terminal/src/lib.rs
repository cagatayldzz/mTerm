//! A shell session: the PTY, terminal emulation (via `alacritty_terminal`) and
//! key encoding. Nothing here depends on the interface; the UI reads
//! [`Snapshot`]s and writes bytes produced by [`encode_key`].

mod keys;
mod mouse;
mod palette;
mod search;
mod session;
mod snapshot;

pub use keys::{dropped_paths, encode_key, paste_bytes, Modifiers};
pub use mouse::{encode_mouse, MouseAction, MouseButton, MouseEvent};
pub use palette::{Palette, Rgb};
pub use search::{Highlight, Match};
pub use session::{Modes, Session, SessionEvent, SessionOptions};
pub use snapshot::{Cursor, CursorShape, Snapshot, Span};
