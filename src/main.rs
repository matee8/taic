use clap::Parser;

#[derive(Parser)]
#[command(name = "llmcli", version, about)]
struct Cli {
    #[arg(short, long, value_name = "LLM")]
    model: String,
    #[arg(long, value_name = "KEY")]
    api_key: String,
    #[arg(long, value_name = "ENDPOINT")]
    api_endpoint: Option<String>,
    #[arg(short, long, value_name = "INTEGER", default_value_t = 0.7)]
    temperature: f32,
    #[arg(short, long, value_name = "STRING")]
    prompt: String,
    #[arg(short, long)]
    reset: bool,
}

fn main() {
    let cli = Cli::parse();

    println!("{}", cli.model);
}
