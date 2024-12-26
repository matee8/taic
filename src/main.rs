use std::{
    io::{self, ErrorKind, IsTerminal as _, Read as _},
    process,
};

use clap::Parser as _;
use futures::StreamExt as _;
use llmcli::{
    chatbots::{dummy::DummyChatbot, gemini::GeminiChatbot},
    cli::{Args, Command},
    config::{Config, ConfigLoadError},
    ui::Printer,
    Chatbot, ChatbotCreationError, ChatbotError, Message, Role,
};
use rustyline::{error::ReadlineError, DefaultEditor};
use thiserror::Error;

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let printer = Printer::new(args.no_color);

    let config = match Config::load(args.config) {
        Ok(config) => Some(config),
        Err(ConfigLoadError::Io(err))
            if matches!(err.kind(), ErrorKind::NotFound) =>
        {
            None
        }
        Err(err) => {
            if let Err(err) = printer.print_error_message(&err.to_string()) {
                eprintln!("Error printing message: {err}");
            }
            process::exit(1);
        }
    };

    let (chatbot, prompt): (
        Result<Box<dyn Chatbot>, ChatbotCreationError>,
        Option<String>,
    ) = match args.command {
        Some(Command::Gemini { model, prompt }) => {
            let api_key = if let Some(config) = config {
                config.api_keys.gemini
            } else {
                None
            };

            (GeminiChatbot::create(model.to_string(), api_key), prompt)
        }
        Some(Command::Dummy { prompt }) => {
            (DummyChatbot::create(String::new(), None), prompt)
        }
        Some(_) => (Err(ChatbotCreationError::UnknownChatbot), None),
        None => {
            if let Some(config) = config {
                match config.default_chatbot.as_str() {
                    "gemini" => (
                        GeminiChatbot::create(
                            config.default_model,
                            config.api_keys.gemini,
                        ),
                        args.prompt,
                    ),
                    "dummy" => {
                        (DummyChatbot::create(String::new(), None), args.prompt)
                    }
                    _ => (Err(ChatbotCreationError::UnknownChatbot), None),
                }
            } else {
                (Err(ChatbotCreationError::UnknownChatbot), None)
            }
        }
    };

    let chatbot = match chatbot {
        Ok(chatbot) => chatbot,
        Err(err) => {
            if let Err(err) = printer.print_error_message(&err.to_string()) {
                eprintln!("Error printing message: {err}");
            }
            process::exit(1);
        }
    };

    if let Err(err) =
        run_chat(chatbot, args.system_prompt, prompt, &printer).await
    {
        if let Err(err) = printer.print_error_message(&err.to_string()) {
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
    #[error("{0}")]
    Command(#[from] CommandError),
}

async fn run_chat(
    mut chatbot: Box<dyn Chatbot>,
    system_prompt: Option<String>,
    prompt: Option<String>,
    printer: &Printer,
) -> Result<(), ChatError> {
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

        let user_message = Message::new(Role::User, input);
        hist.push(user_message);

        printer.print_chatbot_prefix(chatbot.name())?;

        handle_chat_message(&hist, &*chatbot).await?;

        return Ok(());
    }

    let mut rl = DefaultEditor::new()?;

    let user_prefix = printer.get_user_prefix();

    loop {
        let input = rl.readline(&user_prefix)?;

        if input.trim().is_empty() {
            continue;
        }

        if input.starts_with('/') {
            handle_command(&input, &mut hist, &mut chatbot, printer)?;
            continue;
        }

        handle_chat_message(&hist, &*chatbot).await?;

        if !io::stdin().is_terminal() {
            break Ok(());
        }
    }
}

#[derive(Debug, Error)]
enum CommandError {
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("{0}")]
    ChatbotSwitch(#[from] ChatbotCreationError),
    #[error("User quit.")]
    Quit,
}

#[expect(
    clippy::too_many_lines,
    reason = r#"
        Each command requires its own match arm, making further reduction
        difficult.
    "#
)]
fn handle_command(
    line: &str,
    hist: &mut Vec<Message>,
    chatbot: &mut Box<dyn Chatbot>,
    printer: &Printer,
) -> Result<(), CommandError> {
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
        "/chatbot" | "/cb" => {
            if let Some(new_chatbot) = parts.get(1) {
                let new_chatbot = match *new_chatbot {
                    "gemini" => GeminiChatbot::create(
                        "gemini-1.5-flash".to_owned(),
                        None,
                    )?,
                    "dummy" => DummyChatbot::create("1".to_owned(), None)?,
                    _ => {
                        printer.print_error_message("Invalid chatbot.")?;
                        return Ok(());
                    }
                };

                *chatbot = new_chatbot;
                printer.print_app_message(&format!(
                    "Chatbot changed to {}",
                    chatbot.name()
                ))?;
            } else {
                printer.print_error_message(
                    "Chatbot is required. Usage: /chatbot <chatbot>",
                )?;
            }
        }
        "/list_chatbots" | "/lc" => {
            printer.print_app_message("Available chatbots:")?;
            printer.print_app_message("\tgemini - Google Gemini")?;
            printer.print_app_message("\tdummy - Dummy")?;
        }
        "/model" | "/m" => {
            if let Some(new_model) = parts.get(1) {
                match chatbot.change_model((*new_model).to_owned()) {
                    Ok(()) => {
                        printer.print_app_message(&format!(
                            "Chatbot model changed to {}",
                            chatbot.model()
                        ))?;
                    }
                    Err(err) => {
                        printer.print_error_message(&err.to_string())?;
                    }
                }
            } else {
                printer.print_error_message(
                    "Model is required. Usage: /model <model>",
                )?;
            }
        }
        "/list_models" | "/lm" => {
            printer.print_app_message("Available models:")?;
            for model in chatbot.available_models() {
                printer.print_app_message(&format!("\t{model}"))?;
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
                "\t/clear or /c - Clear the conversation history (including system prompt)",
            )?;
            printer.print_app_message(
                "\t/system <prompt> or /s <prompt> - Set the system prompt",
            )?;
            printer.print_app_message(
                "\t/model <model> or /m <model> - Change the chatbot model",
            )?;
            printer.print_app_message("\t/list_models or /lm - List all available models for current chatbot")?;
            printer.print_app_message(
                "\t/chatbot <chatbot> or /cb <chatbot> - Change the chatbot",
            )?;
            printer.print_app_message(
                "\t/list_chatbots or /lc - List all available chatbots",
            )?;
            printer.print_app_message(
                "\t/info or /i - Display current chatbot and model information",
            )?;
            printer.print_app_message(
                "\t/help or /h - List all available commands",
            )?;
            printer
                .print_app_message("\t/quit or /q - Exit the application")?;
        }
        "/quit" | "/q" => {
            printer.print_app_message("Quitting...")?;
            return Err(CommandError::Quit);
        }
        _ => {
            printer.print_error_message(
                "Invalid command. Use /help or /h for a list of commands.",
            )?;
        }
    }

    Ok(())
}

async fn handle_chat_message(
    hist: &[Message],
    chatbot: &dyn Chatbot,
) -> Result<Message, ChatError> {
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

    Ok(Message::new(Role::Assistant, full_resp))
}
