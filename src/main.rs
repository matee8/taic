use std::{
    io::{self, Write as _},
    process,
};

use clap::Parser as _;
use llmcli::{
    chatbots::dummy::DummyChatbot,
    cli::{Args, Command},
    Chatbot, ChatbotError, Message, Role,
};
use thiserror::Error;

#[tokio::main]
async fn main() {
    let args = Args::parse();

    if let Err(err) = run_chat(match args.command {
        Command::Dummy => DummyChatbot::new(),
        _ => {
            eprintln!("Error: Unknown chatbot.");
            process::exit(1);
        }
    })
    .await
    {
        eprintln!("Error: {err}");
        process::exit(1);
    }
}

#[derive(Debug, Error)]
enum ChatError {
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("{0}")]
    Chatbot(#[from] ChatbotError),
}

#[expect(
    clippy::single_call_fn,
    reason = r#"Traits with `async fn` have limitations using dynamic dispatch. 
                `async_trait` uses the heap which isn't the optimal solution.
                This function instead uses static dispatch to work around those."#
)]
async fn run_chat<C>(chatbot: C) -> Result<(), ChatError>
where
    C: Chatbot + Send + Sync,
{
    let mut hist = Vec::new();

    loop {
        print!("You: ");
        io::stdout().flush()?;

        let mut prompt = String::new();
        io::stdin().read_line(&mut prompt)?;

        let _: Option<char> = prompt.pop();

        if prompt.trim().is_empty() {
            continue;
        }

        hist.push(Message::new(Role::User, prompt));

        let resp = chatbot.send_message(&hist).await?;
        println!("{}: {resp}", chatbot.name());
        hist.push(Message::new(Role::Assistant, resp));
    }
}
