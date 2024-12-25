use core::future::Future;
use std::env::VarError;

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod chatbots;
pub mod cli;
pub mod ui;

#[non_exhaustive]
#[derive(PartialEq, Eq, Deserialize, Serialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    #[serde(alias = "model")]
    Assistant,
}

pub struct Message {
    role: Role,
    content: String,
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
}

pub trait Chatbot {
    fn name(&self) -> &'static str;

    fn send_message(
        &self,
        messages: &[Message],
    ) -> impl Future<Output = Result<String, ChatbotError>> + Send + Sync;
}
