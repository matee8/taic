extern crate alloc;

use alloc::boxed::Box;
use std::env::VarError;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod chatbots;
pub mod cli;
pub mod commands;
pub mod config;
pub mod history;
pub mod session;
pub mod ui;

#[non_exhaustive]
#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    #[serde(alias = "model")]
    Assistant,
}

#[non_exhaustive]
#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

impl Message {
    #[inline]
    #[must_use]
    pub const fn new(role: Role, content: String) -> Self {
        Self { role, content }
    }
}

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum ChatbotChatError {
    #[error("Timeout.")]
    Timeout,
    #[error("Network error: {0}.")]
    NetworkError(#[from] reqwest::Error),
    #[error("Unexpected response.")]
    UnexpectedResponse,
}

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum ChatbotCreationError {
    #[error("API key missing.")]
    ApiKeyMissing(#[from] VarError),
    #[error("Unknown chatbot.")]
    UnknownChatbot,
    #[error("Unknown model.")]
    UnknownModel,
}

#[non_exhaustive]
#[derive(Debug, Error)]
#[error("Invalid model.")]
pub struct InvalidModelError;

#[async_trait]
pub trait Chatbot {
    fn create(
        model: String,
        api_key: Option<String>,
    ) -> Result<Box<dyn Chatbot>, ChatbotCreationError>
    where
        Self: Sized;

    fn name(&self) -> &'static str;

    fn model(&self) -> &'static str;

    fn available_models(&self) -> &[&str];

    fn change_model(
        &mut self,
        new_model: String,
    ) -> Result<(), InvalidModelError>;

    async fn send_message(
        &self,
        messages: &[Message],
    ) -> Result<String, ChatbotChatError>;
}
