//! Reading a line from the terminal and telling what kind of input it is

use rustyline::completion::Completer;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::history::DefaultHistory;
use rustyline::validate::Validator;
use rustyline::{Config, Editor, Helper};

use crate::help::show_help;

pub struct TilesHinter;

impl Hinter for TilesHinter {
    type Hint = String;

    fn hint(&self, line: &str, _pos: usize, _ctx: &rustyline::Context<'_>) -> Option<Self::Hint> {
        if line.is_empty() {
            Some("Send a message (/help to show available commands)".to_string())
        } else {
            None
        }
    }
}

impl Completer for TilesHinter {
    type Candidate = String;
}

impl Highlighter for TilesHinter {
    fn highlight_hint<'h>(&self, hint: &'h str) -> std::borrow::Cow<'h, str> {
        std::borrow::Cow::Owned(format!("\x1b[2m{}\x1b[0m", hint))
    }
}

impl Validator for TilesHinter {}

impl Helper for TilesHinter {}

pub fn create_line_editor() -> rustyline::Result<Editor<TilesHinter, DefaultHistory>> {
    let config = Config::builder().auto_add_history(true).build();
    let mut editor = Editor::with_config(config)?;
    editor.set_helper(Some(TilesHinter));
    Ok(editor)
}

#[derive(Debug, PartialEq, Eq)]
pub enum InputType {
    Skip,
    Command(String),
    Exit,
    Prompt,
    Skill,
}

pub fn handle_input(input: &str) -> InputType {
    if let Some(cmd) = input.strip_prefix('/') {
        let keyword = cmd.split_whitespace().next().unwrap_or("");
        match keyword.to_lowercase().as_str() {
            "help" | "?" => {
                show_help();
                InputType::Skip
            }
            "bye" => InputType::Exit,
            "" => {
                println!("Empty command. Type /help for available commands.");
                InputType::Skip
            }
            _ => InputType::Command(cmd.to_owned()),
        }
    } else if let Some(_skill) = input.strip_prefix('$') {
        InputType::Skill
    } else {
        InputType::Prompt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_and_an_empty_command_are_answered_without_the_agent() {
        assert_eq!(handle_input("/help"), InputType::Skip);
        assert_eq!(handle_input("/?"), InputType::Skip);
        assert_eq!(handle_input("/"), InputType::Skip);
    }

    #[test]
    fn bye_exits() {
        assert_eq!(handle_input("/bye"), InputType::Exit);
    }

    #[test]
    fn other_slash_commands_keep_their_arguments() {
        assert_eq!(
            handle_input("/resume abc"),
            InputType::Command("resume abc".to_owned())
        );
    }

    #[test]
    fn a_dollar_line_is_a_skill_and_the_rest_is_a_prompt() {
        assert_eq!(handle_input("$review"), InputType::Skill);
        assert_eq!(handle_input("what is 2+2"), InputType::Prompt);
    }
}
