use std::{
    env,
    fs::File,
    io::{self, ErrorKind, IsTerminal as _, Read as _},
    path::PathBuf,
    process,
};

use clap::Parser as _;
use futures::StreamExt as _;
use llmcli::{
    chatbots::{dummy::DummyChatbot, gemini::GeminiChatbot},
    cli::{Args, Command},
    config::{Config, ConfigLoadError},
    session::{Session, SessionError},
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
            #[expect(
                clippy::let_underscore_must_use,
                reason = r#"
                    We are exiting immediately after printing the error message,
                    so we don't need to handle a potential error.
                    The `unwrap_or_else` is only used to log the error to stderr
                    in case of `print_error_message` failure.
                "#
            )]
            let _: Result<(), ()> = printer
                .print_error_message(&err.to_string())
                .map_err(|err| {
                    eprintln!("Error printing message: {err}");
                });
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
            #[expect(
                clippy::let_underscore_must_use,
                reason = r#"
                    See the one above this.
                "#
            )]
            let _: Result<(), ()> = printer
                .print_error_message(&err.to_string())
                .map_err(|err| {
                    eprintln!("Error printing message: {err}");
                });
            process::exit(1);
        }
    };

    if let Err(err) =
        run_chat(chatbot, args.system_prompt, prompt, &printer).await
    {
        #[expect(
            clippy::let_underscore_must_use,
            reason = r#"
                See the one above this.
            "#
        )]
        let _: Result<(), ()> = printer
            .print_error_message(&err.to_string())
            .map_err(|err| {
                eprintln!("Error printing message: {err}");
            });
        process::exit(1);
    }
}

#[derive(Debug, Error)]
enum ChatError {
    #[error("Failed to read from stdin: {0}.")]
    Read(io::Error),
    #[error("Failed to print message: {0}.")]
    Print(io::Error),
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
    let mut session = Session::new();

    if let Some(system_prompt) = system_prompt {
        session
            .messages
            .push(Message::new(Role::System, system_prompt));
    }

    if let Some(prompt) = prompt {
        let input = if prompt == "-" {
            let mut input = String::new();
            io::stdin()
                .read_to_string(&mut input)
                .map_err(ChatError::Read)?;
            input
        } else {
            prompt
        };

        let user_message = Message::new(Role::User, input);
        session.messages.push(user_message);

        printer
            .print_chatbot_prefix(chatbot.name())
            .map_err(ChatError::Print)?;

        handle_chat_message(&session.messages, &*chatbot).await?;

        return Ok(());
    }

    let mut rl = DefaultEditor::new()?;

    let history_file = if let Ok(path) = env::var("LLMCLI_HISTORY_FILE") {
        if let Err(err) = rl.load_history(&path) {
            printer
                .print_error_message(&format!(
                    "Failed to load history from file ({path:?}): {err}"
                ))
                .map_err(ChatError::Print)?;
        }
        Some(PathBuf::from(path))
    } else if let Some(mut path) = dirs::cache_dir() {
        path.push("llmcli_history.txt");
        if !path.exists() {
            if let Err(err) = File::create(&path) {
                printer
                    .print_error_message(&format!(
                        "Failed to create history file ({path:?}): {err}"
                    ))
                    .map_err(ChatError::Print)?;
            };
        }
        if let Err(err) = rl.load_history(&path) {
            printer
                .print_error_message(&format!(
                    "Failed to load history from file ({path:?}): {err}"
                ))
                .map_err(ChatError::Print)?;
        }
        Some(path)
    } else {
        printer
            .print_error_message("Failed to find cache directory for history.")
            .map_err(ChatError::Print)?;
        None
    };

    let user_prefix = printer.get_user_prefix();

    loop {
        let input = match rl.readline(&user_prefix) {
            Ok(line) => Ok(line),
            Err(err) => {
                if matches!(err, ReadlineError::Interrupted) {
                    if let Some(ref history_file) = history_file {
                        if let Err(err) = rl.save_history(history_file) {
                            printer.print_error_message(
                                &format!(
                                    "Failed to save history to file ({history_file:?}): {err}."
                                )
                            )
                            .map_err(ChatError::Print)?;
                        }
                    } else {
                        printer.print_error_message(
                            &format!(
                                "Failed to save history to file ({history_file:?}): No history file specified."
                            )
                        ).map_err(ChatError::Print)?;
                    }
                }
                Err(err)
            }
        }?;

        if input.trim().is_empty() {
            continue;
        }

        if input.starts_with('/') {
            if let Err(err) =
                handle_command(&input, &mut session, &mut chatbot, printer)
            {
                if matches!(err, CommandError::Quit) {
                    if let Some(ref history_file) = history_file {
                        if let Err(err) = rl.save_history(history_file) {
                            printer.print_error_message(
                                &format!(
                                    "Failed to save history to file ({history_file:?}): {err}"
                                )
                            ).map_err(ChatError::Print)?;
                        } else {
                            printer.print_error_message(
                                &format!(
                                    "Failed to save history to file ({history_file:?}): No history file specified."
                                )
                            ).map_err(ChatError::Print)?;
                        }
                    }
                }
                return Err(err.into());
            };
            rl.add_history_entry(input.as_str())?;
            continue;
        }

        let user_message = Message::new(Role::User, input);
        session.messages.push(user_message);

        printer
            .print_chatbot_prefix(chatbot.name())
            .map_err(ChatError::Print)?;

        handle_chat_message(&session.messages, &*chatbot).await?;

        if !io::stdin().is_terminal() {
            break Ok(());
        }
    }
}

#[derive(Debug, Error)]
enum CommandError {
    #[error("Failed to print message: {0}")]
    Print(#[from] io::Error),
    #[error("{0}")]
    ChatbotSwitch(#[from] ChatbotCreationError),
    #[error("{0}")]
    Session(#[from] SessionError),
    #[error("User quit.")]
    Quit,
}

fn handle_command(
    line: &str,
    session: &mut Session,
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
            session.messages.clear();
            printer.print_app_message("Context cleared.")?;
        }
        "/system" | "/sys" => {
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
                session.messages.retain(|msg| msg.role != Role::System);
                session.messages.insert(0, new_msg);
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
                &session.messages.iter().find(|msg| msg.role == Role::System)
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
                "\t/system <prompt> or /sys <prompt> - Set the system prompt",
            )?;
            printer.print_app_message(
                "\t/chatbot <chatbot> or /cb <chatbot> - Change the chatbot",
            )?;
            printer.print_app_message(
                "\t/list_chatbots or /lc - List all available chatbots",
            )?;
            printer.print_app_message(
                "\t/model <model> or /m <model> - Change the chatbot model",
            )?;
            printer.print_app_message(
                "\t/list_models or /lm - List all available models for current chatbot"
            )?;
            printer.print_app_message(
                "\t/info or /i - Display current chatbot and model information",
            )?;
            printer.print_app_message(
                "\t/save <filename> or /s <filename> - Save the session",
            )?;
            printer.print_app_message(
                "\t/load <filename> or /l <filename> - Load a saved session",
            )?;
            printer.print_app_message(
                "\t/delete <filename> or /d - Delete a session",
            )?;
            printer.print_app_message(
                "\t/sessions or /se - List all saved session",
            )?;
            printer.print_app_message(
                "\t/delete <filename> or /d - Delete a session",
            )?;
            printer.print_app_message(
                "\t/help or /h - List all available commands",
            )?;
            printer
                .print_app_message("\t/quit or /q - Exit the application")?;
        }
        "/save" | "/s" => {
            if let Some(filename) = parts.get(1) {
                session.save(filename)?;
                printer.print_app_message(&format!(
                    "Session saved to {filename}.json"
                ))?;
            } else {
                printer.print_error_message(
                    "Filename is required. Usage: /save <filename>",
                )?;
            }
        }
        "/load" | "/l" => {
            if let Some(filename) = parts.get(1) {
                let loaded_session = Session::load(filename)?;
                *session = loaded_session;
                printer.print_app_message(&format!(
                    "Session loaded from {filename}.json"
                ))?;
            } else {
                printer.print_error_message(
                    "Filename is required. Usage: /load <filename>",
                )?;
            }
        }
        "/sessions" | "/se" => {
            let sessions = Session::list_all()?;
            if sessions.is_empty() {
                printer.print_error_message("No saved sessions found.")?;
            } else {
                printer.print_app_message("Saved sessions:")?;
                for elem in sessions {
                    printer.print_app_message(&format!("\t{elem}"))?;
                }
            }
        }
        "/delete" | "/d" => {
            if let Some(filename) = parts.get(1) {
                Session::delete(filename)?;
                printer.print_app_message(&format!(
                    "Session {filename}.json deleted."
                ))?;
            } else {
                printer.print_error_message(
                    "Filename is required. Usage: /delete <filename>",
                )?;
            }
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
