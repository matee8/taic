use alloc::borrow::Cow;
use std::env;

use async_trait::async_trait;
use futures::StreamExt as _;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::{Chatbot, ChatbotError, InvalidModelError, ResponseStream, Role};

const GEMINI_BASE_URL: &str =
    "https://generativelanguage.googleapis.com/v1beta/models/";

const AVAILABLE_MODELS: [&str; 5] = [
    "gemini-2.0-flash-exp",
    "gemini-1.5-flash",
    "gemini-1.5-flash-8b",
    "gemini-1.5-pro",
    "gemini-1.0-pro",
];

#[derive(Serialize, Deserialize)]
struct GeminiPart<'text> {
    text: Cow<'text, str>,
}

#[derive(Serialize, Deserialize)]
struct GeminiMessage<'text> {
    role: Role,
    #[serde(borrow)]
    parts: Vec<GeminiPart<'text>>,
}

#[derive(Serialize)]
struct SystemInstruction<'text> {
    parts: Vec<GeminiPart<'text>>,
}

#[derive(Serialize)]
struct GeminiRequest<'system, 'text> {
    system_instruction: Option<SystemInstruction<'system>>,
    contents: Vec<GeminiMessage<'text>>,
}

#[derive(Deserialize)]
struct GeminiCandidate<'text> {
    #[serde(borrow)]
    content: GeminiMessage<'text>,
}

#[derive(Deserialize)]
struct GeminiResponse<'text> {
    #[serde(borrow)]
    candidates: Vec<GeminiCandidate<'text>>,
}

#[non_exhaustive]
pub struct GeminiChatbot {
    api_key: String,
    model: String,
    url: String,
    client: Client,
}

#[async_trait]
impl Chatbot for GeminiChatbot {
    #[inline]
    fn create(
        model: String,
        api_key: Option<String>,
    ) -> Result<Box<dyn Chatbot>, ChatbotError> {
        let api_key = if let Some(api_key) = api_key {
            api_key
        } else {
            env::var("GEMINI_API_KEY")?
        };

        if !AVAILABLE_MODELS.contains(&model.as_str()) {
            return Err(ChatbotError::UnknownModel);
        }

        let url =
            format!("{GEMINI_BASE_URL}{model}:streamGenerateContent?alt=sse&key={api_key}");

        let client = Client::new();

        Ok(Box::new(Self {
            api_key,
            model,
            url,
            client,
        }))
    }

    #[inline]
    fn name(&self) -> &'static str {
        "Gemini"
    }

    #[inline]
    fn model(&self) -> &'static str {
        #[expect(
            clippy::unreachable,
            reason = r#"
                `model` is validated on initialization and in `change_model`,
                so it should always be a valid name.
            "#
        )]
        match self.model.as_str() {
            "gemini-2.0-flash-exp" => "2.0 Flash (Experimental)",
            "gemini-1.5-flash" => "1.5 Flash",
            "gemini-1.5-flash-8b" => "1.5 Flash-8B",
            "gemini-1.5-pro" => "1.5 Pro",
            "gemini-1.0-pro" => "1.0 Pro (Deprecated)",
            _ => unreachable!(),
        }
    }

    #[inline]
    fn change_model(
        &mut self,
        new_model: String,
    ) -> Result<(), InvalidModelError> {
        if !AVAILABLE_MODELS.contains(&new_model.as_str()) {
            return Err(InvalidModelError);
        }

        self.model = new_model;

        self.url = format!(
            "{GEMINI_BASE_URL}{}:streamGenerateContent?alt=sse&key={}",
            self.model, self.api_key
        );

        Ok(())
    }

    #[inline]
    async fn send_message(
        &self,
        messages: &[crate::Message],
    ) -> Result<ResponseStream, ChatbotError> {
        let system = messages.iter().find(|msg| msg.role == Role::System).map(
            |system_prompt| SystemInstruction {
                parts: vec![GeminiPart {
                    text: Cow::Borrowed(&system_prompt.content),
                }],
            },
        );

        let gemini_messages: Vec<GeminiMessage<'_>> = messages
            .iter()
            .filter(|msg| msg.role != Role::System)
            .map(|msg| GeminiMessage {
                role: msg.role,
                parts: vec![GeminiPart {
                    text: Cow::Borrowed(&msg.content),
                }],
            })
            .collect();

        let request_body = GeminiRequest {
            system_instruction: system,
            contents: gemini_messages,
        };

        let resp_stream = self
            .client
            .post(&self.url)
            .json(&request_body)
            .send()
            .await
            .map_err(|err| {
                if err.is_timeout() {
                    ChatbotError::Timeout
                } else {
                    ChatbotError::NetworkError(err)
                }
            })?
            .bytes_stream();

        let stream = resp_stream
            .map(move |item| match item {
                Ok(bytes) => {
                    #[expect(
                        clippy::map_err_ignore,
                        reason = r#"
                            Invalid JSON from the API indicates a critical error
                            so we hide that detail from the end user, as they
                            cannot address this issue.
                        "#
                    )]
                    #[expect(
                        clippy::indexing_slicing,
                        reason = r#"
                            The Gemini API prepends "data: " to each JSON
                            chunk in the stream. We need to remove this
                            non-JSON prefix before deserialization.
                        "#
                    )]
                    let gemini_resp: GeminiResponse<'_> =
                        serde_json::from_slice(&bytes[5..])
                            .map_err(|_| ChatbotError::UnexpectedResponse)?;

                    Ok(gemini_resp
                        .candidates
                        .into_iter()
                        .next()
                        .and_then(|candidate| {
                            candidate
                                .content
                                .parts
                                .into_iter()
                                .next()
                                .map(|part| Ok(part.text.into_owned()))
                        })
                        .unwrap_or_else(|| {
                            Err(ChatbotError::UnexpectedResponse)
                        })?)
                }
                Err(_) => Err(ChatbotError::UnexpectedResponse),
            })
            .boxed();

        Ok(stream)
    }
}
