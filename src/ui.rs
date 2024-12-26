use std::io;

use crossterm::{
    execute,
    style::{
        Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor,
    },
};

// Cannot be a `const fn` because we apply ANSI escape codes for colors based
// on terminal capabilities, which are determined at runtime.
// Using `crossterm` functions directly within a const str is also
// not possible as they are not `const fn` compatible.
#[inline]
#[must_use]
pub fn get_input_prompt() -> String {
    format!(
        "{}{}You: {}{}",
        SetForegroundColor(Color::Magenta),
        SetAttribute(Attribute::Bold),
        ResetColor,
        SetAttribute(Attribute::Reset)
    )
}

#[inline]
pub fn print_app_message(message: &str) -> io::Result<()> {
    execute!(
        io::stdout(),
        SetForegroundColor(Color::Blue),
        SetAttribute(Attribute::Bold),
        Print("llmcli: "),
        ResetColor,
        SetAttribute(Attribute::Reset),
        Print(message),
        Print("\n"),
    )
}

#[inline]
pub fn print_chatbot_prompt(name: &str) -> io::Result<()> {
    execute!(
        io::stdout(),
        SetForegroundColor(Color::Cyan),
        SetAttribute(Attribute::Bold),
        Print(name),
        Print(": "),
        ResetColor,
        SetAttribute(Attribute::Reset),
    )
}

#[inline]
pub fn print_error_message(message: &str) -> io::Result<()> {
    execute!(
        io::stdout(),
        SetForegroundColor(Color::Red),
        SetAttribute(Attribute::Bold),
        Print("Error: "),
        ResetColor,
        SetAttribute(Attribute::Reset),
        Print(message),
        Print("\n"),
    )
}
