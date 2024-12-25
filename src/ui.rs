use std::io;

use crossterm::{
    execute,
    style::{
        Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor,
    },
};

#[inline]
pub fn print_chatbot_message(name: &str) -> io::Result<()> {
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
