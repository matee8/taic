extern crate alloc;

use alloc::boxed::Box;
use core::pin::Pin;
use std::env::VarError;

use async_trait::async_trait;
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod chatbots;
pub mod cli;
pub mod config;
pub mod ui;

type ResponseStream =
    Pin<Box<dyn Stream<Item = Result<String, ChatbotError>> + Send + 'static>>;

#[non_exhaustive]
#[derive(PartialEq, Eq, Deserialize, Serialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    #[serde(alias = "model")]
    Assistant,
}

#[non_exhaustive]
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
#[error("Invalid model.")]
pub struct InvalidModelError;

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum ChatbotError {
    #[error("API key missing.")]
    ApiKeyMissing(#[from] VarError),
    #[error("Timeout.")]
    Timeout,
    #[error("Server error.")]
    ServerError,
    #[error("Network error: {0}.")]
    NetworkError(#[from] reqwest::Error),
    #[error("Unexpected response.")]
    UnexpectedResponse,
    #[error("Unknown chatbot.")]
    UnknownChatbot,
    #[error("Unknown model.")]
    UnknownModel,
}

#[async_trait]
pub trait Chatbot {
    fn create(
        model: String,
        api_key: Option<String>,
    ) -> Result<Box<dyn Chatbot>, ChatbotError>
    where
        Self: Sized;

    fn name(&self) -> &'static str;

    fn model(&self) -> &'static str;

    fn change_model(
        &mut self,
        new_model: String,
    ) -> Result<(), InvalidModelError>;

    async fn send_message(
        &self,
        messages: &[Message],
    ) -> Result<ResponseStream, ChatbotError>;
}

pub trait Model {}
