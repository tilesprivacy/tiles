//! The Tiles command line: what it reads from the terminal, asks the daemon, and prints

mod api;
mod atproto;
mod browser;
mod commands;
mod help;
mod input;
mod sse;

pub use api::{AtprotoAccount, DaemonApi, Session};
pub use atproto::sign_in;
pub use browser::open_in_browser;
pub use commands::{share_session_command, show_session_list, show_status};
pub use input::{InputType, TilesHinter, create_line_editor, handle_input};
pub use sse::take_events;
