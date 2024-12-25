use core::fmt::{self, Display, Formatter};

use clap::{Parser, Subcommand, ValueEnum};

#[non_exhaustive]
#[derive(Parser)]
#[command(author, version, about)]
#[command(propagate_version = true)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[non_exhaustive]
#[derive(Debug, Clone, ValueEnum)]
pub enum GeminiModel {
    #[clap(name = "gemini-1.5-flash")]
    Flash,
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
            &Self::Flash => writeln!(f, "gemini-1.5-flash"),
        }
    }
}

#[non_exhaustive]
#[derive(Subcommand)]
pub enum Command {
    Gemini {
        #[arg(short, long, value_enum, default_value_t = GeminiModel::Flash)]
        model: GeminiModel,
    },
    Dummy,
}
