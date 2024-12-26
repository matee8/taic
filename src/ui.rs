use std::io;

use crossterm::{
    execute,
    style::{
        Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor,
    },
};

pub struct Printer {
    no_color: bool,
}

impl Printer {
    #[inline]
    #[must_use]
    pub const fn new(no_color: bool) -> Self {
        Self { no_color }
    }

    #[inline]
    #[must_use]
    pub fn get_user_prefix(&self) -> String {
        if self.no_color {
            "You: ".to_owned()
        } else {
            format!(
                "{}{}You: {}{}",
                SetForegroundColor(Color::Magenta),
                SetAttribute(Attribute::Bold),
                ResetColor,
                SetAttribute(Attribute::Reset)
            )
        }
    }

    #[inline]
    pub fn print_app_message(&self, message: &str) -> io::Result<()> {
        if self.no_color {
            println!("llmcli: {message}");
            Ok(())
        } else {
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
    }

    #[inline]
    pub fn print_chatbot_prefix(&self, name: &str) -> io::Result<()> {
        if self.no_color {
            print!("{name}: ");
            Ok(())
        } else {
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
    }

    #[inline]
    pub fn print_error_message(&self, message: &str) -> io::Result<()> {
        if self.no_color {
            println!("Error: {message}");
            Ok(())
        } else {
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
    }
}
