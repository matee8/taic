use std::{
    io::{self, IsTerminal as _, Read as _},
    process,
};

use clap::Parser as _;
use futures::StreamExt as _;
use llmcli::{
    chatbots::{dummy::DummyChatbot, gemini::GeminiChatbot},
    cli::{Args, Command},
    ui::Printer,
    Chatbot, ChatbotError, Message, Role,
};
use rustyline::{error::ReadlineError, DefaultEditor};
use thiserror::Error;

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let printer = Printer::new(args.no_color);

    if let Err(err) = match args.command {
        Command::Gemini { model, prompt } => match GeminiChatbot::new(model) {
            Ok(chatbot) => {
                run_chat(chatbot, args.system_prompt, prompt, &printer).await
            }
            Err(err) => Err(err.into()),
        },
        Command::Dummy { prompt } => {
            run_chat(DummyChatbot::new(), args.system_prompt, prompt, &printer)
                .await
        }
        _ => Err(ChatError::UnknownChatbot),
    } {
        if let Err(err) = printer.print_error_message(&err.to_string()) {
            eprintln!("Error printing message: {err}");
        }
        process::exit(1);
    }
}

// Traits with `async fn` have limitations using dynamic dispatch.
// `async_trait` uses the heap which isn't the optimal solution.
// This function instead uses static dispatch to work around those.
async fn run_chat<C>(
    chatbot: C,
    system_prompt: Option<String>,
    prompt: Option<String>,
    printer: &Printer,
) -> Result<(), ChatError>
where
    C: Chatbot + Send + Sync,
{
    let mut hist = Vec::new();

    if let Some(system_prompt) = system_prompt {
        hist.push(Message::new(Role::System, system_prompt));
    }

    if let Some(prompt) = prompt {
        let input = if prompt == "-" {
            let mut input = String::new();
            io::stdin().read_to_string(&mut input)?;
            input
        } else {
            prompt
        };

        handle_chat_message(input, &mut hist, &chatbot, printer).await?;

        return Ok(());
    }

    let mut rl = DefaultEditor::new()?;

    let input_prompt = printer.get_input_prompt();

    loop {
        let input = rl.readline(&input_prompt)?;

        if !io::stdin().is_terminal() {
            handle_chat_message(input, &mut hist, &chatbot, printer).await?;
            break Ok(());
        }

        if input.trim().is_empty() {
            continue;
        }

        if input.starts_with('/') {
            handle_command(&input, &mut hist, &chatbot, printer)?;
        } else {
            handle_chat_message(input, &mut hist, &chatbot, printer).await?;
        }
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
    #[error("User quit.")]
    Quit,
}

fn handle_command<C>(
    line: &str,
    hist: &mut Vec<Message>,
    chatbot: &C,
    printer: &Printer,
) -> Result<(), ChatError>
where
    C: Chatbot + Send + Sync,
{
    let parts: Vec<&str> = line.split_whitespace().collect();
    let Some(command) = parts.first() else {
        printer.print_error_message("No command specified.")?;
        return Ok(());
    };

    match *command {
        "/clear" | "/c" => {
            hist.clear();
            printer.print_app_message("Context cleared.")?;
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
                let new_msg = Message::new(Role::System, parts[1..].join(" "));
                if let Some(first) = hist.first_mut() {
                    if first.role == Role::System {
                        *first = new_msg;
                    }
                } else {
                    hist.insert(0, new_msg);
                }
                printer.print_app_message("System prompt set.")?;
            } else {
                printer.print_error_message(
                    "System prompt is required. Usage: /system <prompt>",
                )?;
            }
        }
        "/info" | "/i" => {
            printer.print_app_message(&format!(
                "Current chatbot: {}",
                chatbot.name()
            ))?;
            printer.print_app_message(&format!(
                "Current model: {}",
                chatbot.model()
            ))?;
            if let &Some(system_msg) =
                &hist.iter().find(|msg| msg.role == Role::System)
            {
                printer.print_app_message(&format!(
                    "System prompt: {}",
                    system_msg.content
                ))?;
            }
        }
        "/help" | "/h" => {
            printer.print_app_message("Available commands:")?;
            printer.print_app_message(
                "/clear or /c - Clear the conversation history (including system prompt)",
            )?;
            printer.print_app_message(
                "/system <prompt> or /s <prompt> - Set the system prompt",
            )?;
            printer.print_app_message(
                "/info or /i - Display current chatbot and model information",
            )?;
            printer.print_app_message(
                "/help or /h - List all available commands",
            )?;
            printer.print_app_message("/quit or /q - Exit the application")?;
        }
        "/quit" | "/q" => {
            printer.print_app_message("Exiting...")?;
            return Err(ChatError::Quit);
        }
        _ => {
            printer.print_error_message(
                "Invalid command. Use /help or /h for a list of commands.",
            )?;
        }
    }

    Ok(())
}

async fn handle_chat_message<C>(
    line: String,
    hist: &mut Vec<Message>,
    chatbot: &C,
    printer: &Printer,
) -> Result<(), ChatError>
where
    C: Chatbot + Send + Sync,
{
    let user_message = Message::new(Role::User, line);
    hist.push(user_message);

    printer.print_chatbot_prompt(chatbot.name())?;

    let mut full_resp = String::new();

    let mut stream = chatbot.send_message(hist).await?;

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

    Ok(())
}
