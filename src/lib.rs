use core::future::Future;

use thiserror::Error;

pub mod chatbots;
pub mod cli;

#[non_exhaustive]
#[derive(PartialEq, Eq)]
pub enum Role {
    System,
    User,
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
    #[error("Invalid API key.")]
    InvalidApiKey,
    #[error("Authentication error.")]
    AuthenticationError,
    #[error("Rate limit exceeded.")]
    RateLimitExceeded,
    #[error("Server error.")]
    ServerError,
    #[error("Model overloaded.")]
    ModelOverloaded,
    #[error("Network error: {0}.")]
    NetworkError(String),
    #[error("Unexpected response: {0}.")]
    UnexpectedResponse(String),
}

pub trait Chatbot {
    fn name(&self) -> &'static str;

    fn send_message(
        &self,
        messages: &[Message],
    ) -> impl Future<Output = Result<String, ChatbotError>> + Send + Sync;
}
