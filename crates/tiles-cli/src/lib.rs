//! The Tiles command line: what it reads from the terminal, asks the daemon, and prints

mod api;
mod atproto;
mod browser;
mod commands;
mod help;
mod input;
mod sse;

pub use api::{AtprotoAccount, DaemonApi, Session};
pub use atproto::{sign_in, sign_out};
pub use browser::open_in_browser;
pub use commands::config::{get_setting, list_settings, reset_settings, set_settings};
pub use commands::repl::{
    LoadedSession, load_session_history, share_session_command, show_session_list, show_status,
};
pub use commands::server::{start_server, stop_server};
pub use input::{InputType, TilesHinter, create_line_editor, handle_input};
pub use sse::take_events;
