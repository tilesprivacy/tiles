//! What `tiles run` reads from the terminal and prints to it

mod api;
mod atproto;
mod browser;
mod commands;
mod help;
mod input;
mod sse;

pub use api::DaemonApi;
pub use atproto::{AtprotoAccount, share_session_command, sign_in};
pub use browser::open_in_browser;
pub use commands::show_session_list;
pub use input::{InputType, TilesHinter, create_line_editor, handle_input};
pub use sse::take_events;
