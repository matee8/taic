use async_trait::async_trait;

use crate::{
    Chatbot, ChatbotChatError, ChatbotCreationError, InvalidModelError, Role,
};

const AVAILABLE_MODELS: [&str; 2] = ["1", "2"];

#[non_exhaustive]
#[derive(Default)]
pub struct DummyChatbot {
    model: String,
}

impl DummyChatbot {}

#[async_trait]
impl Chatbot for DummyChatbot {
    #[inline]
    fn create(
        model: String,
        _api_key: Option<String>,
    ) -> Result<Box<dyn Chatbot>, ChatbotCreationError> {
        if AVAILABLE_MODELS.contains(&model.as_str()) {
            Ok(Box::new(Self { model }))
        } else {
            Err(ChatbotCreationError::UnknownModel)
        }
    }

    #[inline]
    fn name(&self) -> &'static str {
        "Dummy"
    }

    #[inline]
    fn model(&self) -> &'static str {
        match self.model.as_str() {
            "1" => "Model 1",
            "2" => "Model 2",
            _ => "Invalid Model",
        }
    }

    #[inline]
    fn available_models(&self) -> &[&str] {
        &AVAILABLE_MODELS
    }

    #[inline]
    fn change_model(
        &mut self,
        new_model: String,
    ) -> Result<(), InvalidModelError> {
        if AVAILABLE_MODELS.contains(&new_model.as_str()) {
            self.model = new_model;
            Ok(())
        } else {
            Err(InvalidModelError)
        }
    }

    #[inline]
    async fn send_message(
        &self,
        messages: &[crate::Message],
    ) -> Result<String, ChatbotChatError> {
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

        Ok(msg)
    }
}
