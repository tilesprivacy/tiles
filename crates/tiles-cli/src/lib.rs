//! What `tiles run` reads from the terminal and prints to it

mod help;
mod input;

pub use input::{InputType, TilesHinter, create_line_editor, handle_input};
