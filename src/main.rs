use clap::Parser as _;
use llmcli::cli::{Args, Command};

fn main() {
    let args = Args::parse();

    match args.command {
        Command::Gemini { model } => {
            println!("Starting Gemini chatbot with model {model:?}...");
        }
        Command::Dummy => {
            println!("Starting Dummy chatbot...");
        }
        _ => {
            println!("Unknown command! Stopping...");
        }
    }
}
