use std::{io, process};

use clap::Parser as _;
use crossterm::style::{
    Attribute, Color, ResetColor, SetAttribute, SetForegroundColor,
};
use futures::StreamExt as _;
use llmcli::{
    chatbots::{dummy::DummyChatbot, gemini::GeminiChatbot},
    cli::{Args, Command},
    ui, Chatbot, ChatbotError, Message, Role,
};
use rustyline::{error::ReadlineError, DefaultEditor};
use thiserror::Error;

#[tokio::main]
async fn main() {
    let args = Args::parse();

    if let Err(err) = match args.command {
        Command::Gemini { model } => match GeminiChatbot::new(&model) {
            Ok(chatbot) => run_chat(chatbot, args.system_prompt).await,
            Err(err) => Err(err.into()),
        },
        Command::Dummy => {
            run_chat(DummyChatbot::new(), args.system_prompt).await
        }
        _ => Err(ChatError::UnknownChatbot),
    } {
        if let Err(err) = ui::print_error_message(&err.to_string()) {
            eprintln!("Error printing message: {err}");
        }
        process::exit(1);
    }
}

#[derive(Debug, Error)]
enum ChatError {
    #[error("Input/output error.")]
    Io(#[from] io::Error),
    #[error("{0}.")]
    Readline(#[from] ReadlineError),
    #[error("{0}")]
    Chatbot(#[from] ChatbotError),
    #[error("Unkown chatbot.")]
    UnknownChatbot,
}

// Traits with `async fn` have limitations using dynamic dispatch.
// `async_trait` uses the heap which isn't the optimal solution.
// This function instead uses static dispatch to work around those.
async fn run_chat<C>(
    chatbot: C,
    system_prompt: Option<String>,
) -> Result<(), ChatError>
where
    C: Chatbot + Send + Sync,
{
    let mut hist = Vec::new();

    if let Some(prompt) = system_prompt {
        hist.push(Message::new(Role::System, prompt));
    }

    let mut rl = DefaultEditor::new()?;

    // Cannot be a const str because we apply ANSI escape codes for colors based
    // on terminal capabilities, which are determined at runtime.
    // Using `crossterm` functions directly within a const str is also
    // not possible as they are not `const fn` compatible.
    let input_prompt = format!(
        "{}{}You: {}{}",
        SetForegroundColor(Color::Magenta),
        SetAttribute(Attribute::Bold),
        ResetColor,
        SetAttribute(Attribute::Reset)
    );

    loop {
        let prompt = rl.readline(&input_prompt)?;

        if prompt.trim().is_empty() {
            continue;
        }

        hist.push(Message::new(Role::User, prompt));

        ui::print_chatbot_message(chatbot.name())?;
        let mut full_resp = String::new();

        let mut stream = chatbot.send_message(&hist).await?;

        while let Some(result) = stream.next().await {
            match result {
                Ok(text) => {
                    print!("{text}");
                    full_resp.push_str(&text);
                }
                Err(err) => {
                    return Err(err.into());
                }
            }
        }

        hist.push(Message::new(Role::Assistant, full_resp));
    }
}
