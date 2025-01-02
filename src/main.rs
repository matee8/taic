use std::{
    io::{self, IsTerminal as _, Read as _},
    process,
};

use clap::Parser as _;
use llmcli::{
    chatbots::{dummy::DummyChatbot, gemini::GeminiChatbot},
    cli::{Args, ChatbotArg},
    commands::{Command, CommandContext, CommandExecuteError},
    config::Config,
    history::{self, HistoryError},
    session::Session,
    ui::Printer,
    Chatbot, ChatbotChatError, ChatbotCreationError, Role,
};
use rustyline::{error::ReadlineError, DefaultEditor};
use thiserror::Error;

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let printer = Printer::new(args.no_color);
    let config = Config::load(args.config).unwrap_or_else(|err| {
        if let Err(err) = printer.print_error_message(&err.to_string()) {
            eprintln!("Error: {err}");
        }
        process::exit(1);
    });
    let chatbot = create_chatbot(args.command, &config).unwrap_or_else(|err| {
        if let Err(err) = printer.print_error_message(&err.to_string()) {
            eprintln!("Error: {err}");
        }
        process::exit(1);
    });

    let mut session = Session::new();

    if let Some(system_prompt) = args.system_prompt {
        session.add_message(Role::System, system_prompt);
    }

    let mut app = App::new(chatbot, &printer, session);

    let res = if let Some(prompt) = args.prompt {
        app.run_single_prompt(prompt).await
    } else {
        app.run_repl(config).await
    };

    if let Err(err) = res {
        if let Err(err) = printer.print_error_message(&err.to_string()) {
            eprintln!("Error printing message: {err}");
        }
        if !matches!(err, ChatError::Quit)
            && !matches!(err, ChatError::Readline(ReadlineError::Interrupted))
        {
            process::exit(1);
        }
    }
}

fn create_chatbot(
    chatbot: Option<ChatbotArg>,
    config: &Config,
) -> Result<Box<dyn Chatbot>, ChatbotCreationError> {
    match chatbot {
        Some(ChatbotArg::Gemini { model }) => {
            let api_key = config
                .api_keys
                .as_ref()
                .and_then(|api_keys| api_keys.gemini.clone());

            GeminiChatbot::create(model.to_string(), api_key)
        }
        Some(ChatbotArg::Dummy) => DummyChatbot::create(String::new(), None),
        Some(_) => Err(ChatbotCreationError::UnknownChatbot),
        None => {
            let default_chatbot = config
                .default_chatbot
                .as_ref()
                .ok_or(ChatbotCreationError::UnknownChatbot)?;

            let api_keys = config.api_keys.as_ref();

            match default_chatbot.as_str() {
                "gemini" => GeminiChatbot::create(
                    config
                        .default_models
                        .as_ref()
                        .and_then(|models| models.gemini.clone())
                        .ok_or(ChatbotCreationError::UnknownModel)?,
                    api_keys.and_then(|api_keys| api_keys.gemini.clone()),
                ),
                "dummy" => DummyChatbot::create(String::new(), None),
                _ => Err(ChatbotCreationError::UnknownChatbot),
            }
        }
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
    Chatbot(#[from] ChatbotChatError),
    #[error("{0}")]
    History(#[from] HistoryError),
    #[error("User quit.")]
    Quit,
}

struct App<'printer> {
    chatbot: Box<dyn Chatbot>,
    printer: &'printer Printer,
    session: Session,
}

impl<'printer> App<'printer> {
    const fn new(
        chatbot: Box<dyn Chatbot>,
        printer: &'printer Printer,
        session: Session,
    ) -> Self {
        Self {
            chatbot,
            printer,
            session,
        }
    }

    async fn run_single_prompt(
        &mut self,
        prompt: String,
    ) -> Result<(), ChatError> {
        let input = if prompt == "-" {
            let mut input = String::new();
            io::stdin()
                .read_to_string(&mut input)
                .map_err(ChatError::Read)?;
            input
        } else {
            prompt
        };

        self.session.add_message(Role::User, input);

        self.printer
            .print_chatbot_prefix(self.chatbot.name())
            .map_err(ChatError::Print)?;

        self.handle_chat_message().await?;

        Ok(())
    }

    async fn run_repl(&mut self, config: Config) -> Result<(), ChatError> {
        let mut rl = DefaultEditor::new()?;
        let history_file = history::locate_file(&config)?;
        rl.load_history(&*history_file)?;
        let user_prefix = self.printer.get_user_prefix();

        loop {
            print!("{user_prefix}");
            let input = match rl.readline("") {
                Ok(line) => Ok(line),
                Err(err) => {
                    if matches!(err, ReadlineError::Interrupted) {
                        rl.save_history(&&*history_file)?;
                    }
                    Err(err)
                }
            }?;

            if input.trim().is_empty() {
                continue;
            }

            if input.starts_with('/') {
                rl.add_history_entry(&input)?;

                let parts: Vec<&str> = input.split_whitespace().collect();

                let command = Command::from_parts(&parts);

                match command {
                    Ok(command) => {
                        let mut context = CommandContext::new(
                            &parts,
                            &mut self.session,
                            &mut self.chatbot,
                            self.printer,
                            &config,
                        );

                        if let Err(err) = command.execute(&mut context) {
                            match err {
                                CommandExecuteError::Quit => {
                                    rl.save_history(&&*history_file)?;
                                    break Err(ChatError::Quit);
                                }
                                CommandExecuteError::Print(_)
                                | CommandExecuteError::ChatbotSwitch(_)
                                | CommandExecuteError::Session(_)
                                | _ => self
                                    .printer
                                    .print_error_message(&err.to_string())
                                    .map_err(ChatError::Print)?,
                            }
                        }
                    }
                    Err(err) => self
                        .printer
                        .print_error_message(&err.to_string())
                        .map_err(ChatError::Print)?,
                }
                continue;
            }

            self.session.add_message(Role::User, input);

            self.printer
                .print_chatbot_prefix(self.chatbot.name())
                .map_err(ChatError::Print)?;

            self.handle_chat_message().await?;

            if !io::stdin().is_terminal() {
                break Ok(());
            }
        }
    }

    async fn handle_chat_message(&mut self) -> Result<(), ChatError> {
        let result = self.chatbot.send_message(&self.session.messages).await?;

        print!("{result}");

        self.session.add_message(Role::Assistant, result);

        Ok(())
    }
}
