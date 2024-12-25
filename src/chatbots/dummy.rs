use crate::{Chatbot, ChatbotError, Role};

#[non_exhaustive]
#[derive(Default)]
pub struct DummyChatbot;

impl DummyChatbot {
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Chatbot for DummyChatbot {
    #[inline]
    fn name(&self) -> &'static str {
        "Dummy"
    }

    #[inline]
    async fn send_message(
        &self,
        messages: &[crate::Message],
    ) -> Result<String, ChatbotError> {
        let msg = messages.last().map_or_else(
            || "Dummy response to empty conversation.".to_owned(),
            |last_msg| {
                if last_msg.role == Role::User {
                    format!("Dummy response to: \"{}\".", last_msg.content)
                } else {
                    "Dummy response.".to_owned()
                }
            },
        );

        println!("{msg}");

        Ok(msg)
    }
}
