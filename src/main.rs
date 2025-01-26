use std::process;

use clap::Parser as _;
use llmcli::{
    cli::{Cli, Command},
    config::ConfigManager,
};

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Init) => {
            let cfg_mgr = ConfigManager::new().unwrap_or_else(|err| {
                eprintln!("Error: {err}");
                process::exit(1);
            });
            cfg_mgr.init_default_config().unwrap_or_else(|err| {
                eprintln!("Error: {err}");
                process::exit(1);
            });
            println!("Configuration initialized at: {:?}", cfg_mgr.config_path);
        }
        Some(_) => {
            eprintln!("Error: Unknown command.");
            process::exit(1);
        }
        None => {}
    }
}
