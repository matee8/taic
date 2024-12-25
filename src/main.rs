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

        if prompt.starts_with('/') {
            let parts: Vec<&str> = prompt.split_whitespace().collect();
            let Some(command) = parts.first() else {
                ui::print_error_message("No command specified.")?;
                continue;
            };

            match *command {
                "/clear" | "/c" => {
                    hist.clear();
                    ui::print_app_message("Context cleared.")?;
                }
                "/system" | "/s" => {
                    if parts.len() > 1 {
                        #[expect(
                            clippy::indexing_slicing,
                            reason = r#"
                                Safe to index: `/system` command requires at
                                least one argument, ensuring `parts` has
                                length >= 2
                            "#
                        )]
                        let new_msg =
                            Message::new(Role::System, parts[1..].join(" "));
                        if let Some(first) = hist.first_mut() {
                            if first.role == Role::System {
                                *first = new_msg;
                            }
                        } else {
                            hist.insert(0, new_msg);
                        }
                        ui::print_app_message("System prompt set.")?;
                    } else {
                        ui::print_error_message("System prompt is required. Usage: /system <prompt>")?;
                    }
                }
                "/info" | "/i" => {
                    ui::print_app_message(&format!(
                        "Current chatbot: {}",
                        chatbot.name()
                    ))?;
                    if let &Some(system_msg) =
                        &hist.iter().find(|msg| msg.role == Role::System)
                    {
                        ui::print_app_message(&format!(
                            "System prompt: {}",
                            system_msg.content
                        ))?;
                    }
                }
                "/help" | "/h" => {
                    ui::print_app_message("Available commands:")?;
                    ui::print_app_message(
                        "/clear or /c - Clear the conversation history (including system prompt)",
                    )?;
                    ui::print_app_message("/system <prompt> or /s <prompt> - Set the system prompt")?;
                    ui::print_app_message("/info or /i - Display current chatbot and model information")?;
                    ui::print_app_message(
                        "/help or /h - List all available commands",
                    )?;
                    ui::print_app_message(
                        "/quit or /q - Exit the application",
                    )?;
                }
                "/quit" | "/q" => {
                    ui::print_app_message("Exiting...")?;
                    break Ok(());
                }
                _ => {
                    ui::print_error_message("Invalid command. Use /help or /h for a list of commands.")?;
                }
            }
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
