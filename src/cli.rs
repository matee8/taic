use core::fmt::{self, Display, Formatter};
use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

#[non_exhaustive]
#[derive(Parser)]
#[command(author, version, about)]
#[command(propagate_version = true)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Command>,
    #[arg(short, long, help = "Set the system prompt")]
    pub system_prompt: Option<String>,
    #[arg(long, help = "Disable colored output")]
    pub no_color: bool,
    #[arg(short, long, help = "Custom config file path", value_name = "FILE")]
    pub config: Option<PathBuf>,
    #[arg(
        help = "Input prompt (optional, reads from stdin if `-`, no prompt starts interactive mode, only works if default_chatbot and default_model is set in configuration file)"
    )]
    pub prompt: Option<String>,
}

#[non_exhaustive]
#[derive(Debug, Clone, ValueEnum)]
pub enum GeminiModel {
    #[clap(name = "gemini-2.0-flash-exp")]
    Flash2_0Exp,
    #[clap(name = "gemini-1.5-flash")]
    Flash1_5,
    #[clap(name = "gemini-1.5-flash-8b")]
    Flash1_5_8B,
    #[clap(name = "gemini-1.5-pro")]
    Pro1_5,
    #[clap(name = "gemini-1.0-pro")]
    Pro1,
}

impl Display for GeminiModel {
    #[inline]
    #[expect(
        clippy::min_ident_chars,
        reason = r#"
            `f` is the default parameter name for `Display` trait 
            implementation.
        "#
    )]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Flash2_0Exp => write!(f, "gemini-2.0-flash-exp"),
            Self::Flash1_5 => write!(f, "gemini-1.5-flash"),
            Self::Flash1_5_8B => write!(f, "gemini-1.5-flash-8b"),
            Self::Pro1_5 => write!(f, "gemini-1.5-pro"),
            Self::Pro1 => write!(f, "gemini-1.0-pro"),
        }
    }
}

#[non_exhaustive]
#[derive(Subcommand)]
pub enum Command {
    #[command(about = "Chat with the Gemini chatbot")]
    Gemini {
        #[arg(short, long, value_enum, default_value_t = GeminiModel::Flash1_5)]
        model: GeminiModel,
        #[arg(
            help = "Input prompt (optional, reads from stdin if `-`, no prompt starts interactive mode)"
        )]
        prompt: Option<String>,
    },
    #[command(about = "Chat with the Dummy chatbot")]
    Dummy {
        #[arg(
            help = "Input prompt (optional, reads from stdin if `-`, no prompt starts interactive mode)"
        )]
        prompt: Option<String>,
    },
}
