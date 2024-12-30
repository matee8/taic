use std::io;

use thiserror::Error;

use crate::{
    chatbots::{dummy::DummyChatbot, gemini::GeminiChatbot},
    config::Config,
    session::{Session, SessionError},
    ui::Printer,
    Chatbot, ChatbotCreationError, Message, Role,
};

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum CommandCreationError {
    #[error("No command specified.")]
    MissingCommand,
    #[error("Invalid command.")]
    Invalid,
    #[error("System prompt is required.")]
    MissingPrompt,
    #[error("Chatbot name is required.")]
    MissingChatbotName,
    #[error("Model name is required.")]
    MissingModelName,
    #[error("Filename is required.")]
    MissingFilename,
}

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum CommandExecuteError {
    #[error("Failed to print message: {0}")]
    Print(#[from] io::Error),
    #[error("{0}")]
    ChatbotSwitch(#[from] ChatbotCreationError),
    #[error("{0}")]
    Session(#[from] SessionError),
    #[error("User quit.")]
    Quit,
}

pub struct CommandContext<'parts, 'session, 'chatbot, 'printer, 'config> {
    parts: &'parts [&'parts str],
    session: &'session mut Session,
    chatbot: &'chatbot mut Box<dyn Chatbot>,
    printer: &'printer Printer,
    config: &'config Config,
}

impl<'parts, 'session, 'chatbot, 'printer, 'config>
    CommandContext<'parts, 'session, 'chatbot, 'printer, 'config>
{
    #[inline]
    #[must_use]
    pub const fn new(
        parts: &'parts [&'parts str],
        session: &'session mut Session,
        chatbot: &'chatbot mut Box<dyn Chatbot>,
        printer: &'printer Printer,
        config: &'config Config,
    ) -> Self {
        Self {
            parts,
            session,
            chatbot,
            printer,
            config,
        }
    }
}

#[non_exhaustive]
pub enum Command<'parts> {
    Clear,
    System { prompt: Message },
    SwitchChatbot { name: &'parts str },
    ListChatbots,
    SwitchModel { name: &'parts str },
    ListModels,
    Info,
    Save { filename: &'parts str },
    Load { filename: &'parts str },
    Delete { filename: &'parts str },
    Sessions,
    Help,
    Quit,
}

impl<'parts> Command<'parts> {
    #[inline]
    pub fn from_parts(
        parts: &'parts [&str],
    ) -> Result<Self, CommandCreationError> {
        let Some(command_name) = parts.first() else {
            return Err(CommandCreationError::MissingCommand);
        };

        match *command_name {
            "/clear" | "/c" => Ok(Self::Clear),
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
                    let new_msg =
                        Message::new(Role::System, parts[1..].join(" "));
                    Ok(Self::System { prompt: new_msg })
                } else {
                    Err(CommandCreationError::MissingPrompt)
                }
            }
            "/chatbot" | "/cb" => parts.get(1).map_or(
                Err(CommandCreationError::MissingChatbotName),
                |name| Ok(Self::SwitchChatbot { name }),
            ),
            "/list_chatbots" | "/lb" => Ok(Command::ListChatbots),
            "/model" | "/m" => parts
                .get(1)
                .map_or(Err(CommandCreationError::MissingModelName), |name| {
                    Ok(Self::SwitchModel { name })
                }),
            "/list_models" | "/lm" => Ok(Self::ListModels),
            "/info" | "/i" => Ok(Self::Info),
            "/save" | "/s" => parts.get(1).map_or(
                Err(CommandCreationError::MissingFilename),
                |filename| Ok(Self::Save { filename }),
            ),
            "/load" | "/l" => parts.get(1).map_or(
                Err(CommandCreationError::MissingFilename),
                |filename| Ok(Self::Load { filename }),
            ),
            "/delete" | "/d" => parts.get(1).map_or(
                Err(CommandCreationError::MissingFilename),
                |filename| Ok(Self::Delete { filename }),
            ),
            "/help" | "/h" => Ok(Self::Help),
            "/quit" | "/q" => Ok(Self::Quit),
            _ => Err(CommandCreationError::Invalid),
        }
    }

    #[inline]
    pub fn execute(
        self,
        context: &mut CommandContext<'_, '_, '_, '_, '_>,
    ) -> Result<(), CommandExecuteError> {
        match self {
            Self::Clear => {
                context.session.messages.clear();
                context.printer.print_app_message("Context cleared.")?;
            }
            Self::System { prompt } => {
                context
                    .session
                    .messages
                    .retain(|msg| msg.role != Role::System);
                context.session.messages.insert(0, prompt);
                context.printer.print_app_message("System prompt set.")?;
            }
            Self::SwitchChatbot { name } => {
                let new_chatbot = match name {
                    "gemini" => GeminiChatbot::create(
                        context
                            .config
                            .default_models
                            .as_ref()
                            .and_then(|models| models.gemini.clone())
                            .ok_or(ChatbotCreationError::UnknownModel)?,
                        context
                            .config
                            .api_keys
                            .as_ref()
                            .and_then(|api_keys| api_keys.gemini.clone()),
                    )?,
                    "dummy" => DummyChatbot::create("1".to_owned(), None)?,
                    _ => {
                        context
                            .printer
                            .print_error_message("Invalid chatbot.")?;
                        return Ok(());
                    }
                };
                *context.chatbot = new_chatbot;
                context.printer.print_app_message(&format!(
                    "Chatbot changed to {}",
                    context.chatbot.name()
                ))?;
            }
            Self::ListChatbots => {
                context.printer.print_app_message("Available chatbots:")?;
                context
                    .printer
                    .print_app_message("\tgemini - Google Gemini")?;
                context.printer.print_app_message("\tdummy - Dummy")?;
            }
            Self::SwitchModel { name } => {
                match context.chatbot.change_model(name.to_owned()) {
                    Ok(()) => {
                        context.printer.print_app_message(&format!(
                            "Chatbot model changed to {}",
                            context.chatbot.model()
                        ))?;
                    }
                    Err(err) => {
                        context
                            .printer
                            .print_error_message(&err.to_string())?;
                    }
                }
            }
            Self::ListModels => {
                context.printer.print_app_message("Available models:")?;
                for model in context.chatbot.available_models() {
                    context.printer.print_app_message(&format!("\t{model}"))?;
                }
            }
            Self::Info => {
                context.printer.print_app_message(&format!(
                    "Current chatbot: {}",
                    context.chatbot.name()
                ))?;
                context.printer.print_app_message(&format!(
                    "Current model: {}",
                    context.chatbot.model()
                ))?;
                if let &Some(system_msg) = &context
                    .session
                    .messages
                    .iter()
                    .find(|msg| msg.role == Role::System)
                {
                    context.printer.print_app_message(&format!(
                        "System prompt: {}",
                        system_msg.content
                    ))?;
                }
            }
            Self::Save { filename } => {
                context.session.save(filename)?;
                context.printer.print_app_message(&format!(
                    "Session saved to {filename}.json"
                ))?;
            }
            Self::Load { filename } => {
                let loaded_session = Session::load(filename)?;
                *context.session = loaded_session;
                context.printer.print_app_message(&format!(
                    "Session loaded from {filename}.json"
                ))?;
            }
            Self::Delete { filename } => {
                Session::delete(filename)?;
                context.printer.print_app_message(&format!(
                    "Session {filename}.json deleted."
                ))?;
            }
            Self::Sessions => {
                let sessions = Session::list_all()?;
                if sessions.is_empty() {
                    context
                        .printer
                        .print_error_message("No saved sessions found.")?;
                } else {
                    context.printer.print_app_message("Saved sessions:")?;
                    for elem in sessions {
                        context
                            .printer
                            .print_app_message(&format!("\t{elem}"))?;
                    }
                }
            }
            Self::Help => {
                context.printer.print_app_message("Available commands:")?;
                context.printer.print_app_message(
                "\t/clear or /c - Clear the conversation history (including system prompt)",
            )?;
                context.printer.print_app_message(
                "\t/system <prompt> or /sys <prompt> - Set the system prompt",
            )?;
                context.printer.print_app_message(
                "\t/chatbot <chatbot> or /cb <chatbot> - Change the chatbot",
            )?;
                context.printer.print_app_message(
                    "\t/list_chatbots or /lc - List all available chatbots",
                )?;
                context.printer.print_app_message(
                    "\t/model <model> or /m <model> - Change the chatbot model",
                )?;
                context.printer.print_app_message(
                "\t/list_models or /lm - List all available models for current chatbot"
            )?;
                context.printer.print_app_message(
                "\t/info or /i - Display current chatbot and model information",
            )?;
                context.printer.print_app_message(
                    "\t/save <filename> or /s <filename> - Save the session",
                )?;
                context.printer.print_app_message(
                "\t/load <filename> or /l <filename> - Load a saved session",
            )?;
                context.printer.print_app_message(
                    "\t/delete <filename> or /d - Delete a session",
                )?;
                context.printer.print_app_message(
                    "\t/sessions or /se - List all saved session",
                )?;
                context.printer.print_app_message(
                    "\t/delete <filename> or /d - Delete a session",
                )?;
                context.printer.print_app_message(
                    "\t/help or /h - List all available commands",
                )?;
                context.printer.print_app_message(
                    "\t/quit or /q - Exit the application",
                )?;
            }
            Self::Quit => {
                context.printer.print_app_message("Quitting...")?;
                return Err(CommandExecuteError::Quit);
            }
        }

        Ok(())
    }
}
