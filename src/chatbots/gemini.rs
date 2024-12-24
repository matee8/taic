use crate::{cli::GeminiModel, Chatbot, ChatbotError, Role};

#[non_exhaustive]
pub struct GeminiChatbot {
    model: GeminiModel,
}

impl GeminiChatbot {
    #[inline]
    #[must_use]
    pub const fn new(model: GeminiModel) -> Self {
        Self { model }
    }
}

impl Chatbot for GeminiChatbot {
    #[inline]
    fn name(&self) -> &'static str {
        "Gemini"
    }

    #[inline]
    async fn send_message(
        &self,
        messages: &[crate::Message],
    ) -> Result<String, ChatbotError> {
        todo!();
    }
}
