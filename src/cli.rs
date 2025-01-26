use clap::{ColorChoice, Parser, Subcommand};

#[non_exhaustive]
#[derive(Parser)]
#[command(name = "llmcli", author, version, about, propagate_version = true)]
#[command(color = ColorChoice::Never)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[non_exhaustive]
#[derive(Subcommand)]
pub enum Command {
    #[command(about = "Initialize configuration")]
    Init,
}
