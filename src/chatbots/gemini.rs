use alloc::borrow::Cow;
use std::env;

use futures::StreamExt as _;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::{
    cli::GeminiModel, Chatbot, ChatbotError, InvalidModelError, ResponseStream, Role
};

const GEMINI_BASE_URL: &str =
    "https://generativelanguage.googleapis.com/v1beta/models/";

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
    url: String,
    client: Client,
    model: GeminiModel,
}

impl GeminiChatbot {
    #[inline]
    pub fn new(model: GeminiModel) -> Result<Self, ChatbotError> {
        let api_key = env::var("GEMINI_API_KEY")?;

        let url =
            format!("{GEMINI_BASE_URL}{model}:streamGenerateContent?alt=sse&key={api_key}");

        let client = Client::new();

        Ok(Self {
            api_key,
            url,
            client,
            model,
        })
    }
}

impl Chatbot for GeminiChatbot {
    #[inline]
    fn name(&self) -> &'static str {
        "Gemini"
    }

    #[inline]
    fn model(&self) -> &'static str {
        match self.model {
            GeminiModel::Flash2_0Exp => "2.0 Flash (Experimental)",
            GeminiModel::Flash1_5 => "1.5 Flash",
            GeminiModel::Flash1_5_8B => "1.5 Flash-8B",
            GeminiModel::Pro1_5 => "1.5 Pro",
            GeminiModel::Pro1 => "1.0 Pro (Deprecated)",
        }
    }

    #[inline]
    fn change_model(
        &mut self,
        new_model: &str,
    ) -> Result<(), InvalidModelError> {
        self.model = match new_model {
            "gemini-2.0-flash-exp" => Ok(GeminiModel::Flash2_0Exp),
            "gemini-1.5-flash" => Ok(GeminiModel::Flash1_5),
            "gemini-1.5-flash.8b" => Ok(GeminiModel::Flash1_5_8B),
            "gemini-1.5-pro" => Ok(GeminiModel::Pro1_5),
            "gemini-1.0-pro" => Ok(GeminiModel::Pro1),
            _ => Err(InvalidModelError),
        }?;

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
