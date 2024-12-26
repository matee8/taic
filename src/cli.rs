use core::fmt::{self, Display, Formatter};

use clap::{Parser, Subcommand, ValueEnum};

#[non_exhaustive]
#[derive(Parser)]
#[command(author, version, about)]
#[command(propagate_version = true)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
    #[arg(short, long, help = "Set the system prompt")]
    pub system_prompt: Option<String>,
    #[arg(long, help = "Disable colored output")]
    pub no_color: bool,
}

#[non_exhaustive]
#[derive(Debug, Clone, ValueEnum)]
pub enum GeminiModel {
    #[clap(name = "gemini-1.5-flash")]
    Flash1_5,
}

impl Display for GeminiModel {
    #[inline]
    #[expect(
        clippy::min_ident_chars,
        reason = r#"`f` is the default parameter name for `Display` trait
                    implementation."#
    )]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            &Self::Flash1_5 => writeln!(f, "gemini-1.5-flash"),
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
            help = "Input prompt (optional, no prompt starts interactive mode and reads from stdin)"
        )]
        prompt: Option<String>,
    },
}
