use core::future::Future;

use thiserror::Error;

pub mod cli;

#[non_exhaustive]
pub enum Role {
    System,
    User,
    Assistant,
}

pub struct Message<'content> {
    role: Role,
    content: &'content str,
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
    fn name() -> &'static str;

    fn send_message(
        &self,
        messages: &[Message<'_>],
    ) -> impl Future<Output = Result<String, ChatbotError>> + Send + Sync;
}
