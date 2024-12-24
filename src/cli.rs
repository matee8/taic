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
    #[clap(name = "gemini-pro")]
    Pro,
    #[clap(name = "gemini-ultra")]
    Ultra,
}

#[non_exhaustive]
#[derive(Subcommand)]
pub enum Command {
    Gemini {
        #[arg(short, long, value_enum, default_value_t = GeminiModel::Pro)]
        model: GeminiModel,
    },
    Dummy,
}
