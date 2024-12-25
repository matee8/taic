extern crate alloc;

use alloc::borrow::Cow;
use std::env;

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::{cli::GeminiModel, Chatbot, ChatbotError, Role};

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
struct GeminiRequest<'text> {
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
    url: String,
    client: Client,
}

impl GeminiChatbot {
    #[inline]
    pub fn new(model: &GeminiModel) -> Result<Self, ChatbotError> {
        let api_key = env::var("GEMINI_API_KEY")?;

        let url =
            format!("{GEMINI_BASE_URL}{model}:generateContent?key={api_key}");

        let client = Client::new();

        Ok(Self { url, client })
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
        let gemini_messages: Vec<GeminiMessage<'_>> = messages
            .iter()
            .map(|msg| GeminiMessage {
                role: msg.role,
                parts: vec![GeminiPart {
                    text: Cow::Borrowed(&msg.content),
                }],
            })
            .collect();

        let request_body = GeminiRequest {
            contents: gemini_messages,
        };

        let resp = self
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
            })?;

        let status = resp.status();

        let payload = resp.text().await.map_err(|err| {
            if err.is_timeout() {
                ChatbotError::Timeout
            } else {
                ChatbotError::NetworkError(err)
            }
        })?;

        if status.is_success() {
            #[expect(
                clippy::map_err_ignore,
                reason = r#"Invalid JSON from the API indicates a critical error
                            so we hide that detail from the end user, as they
                            cannot address this issue."#
            )]
            let gemini_resp: GeminiResponse<'_> =
                serde_json::from_str(&payload)
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
                .unwrap_or_else(|| Err(ChatbotError::UnexpectedResponse))?)
        } else {
            Err(ChatbotError::ServerError)
        }
    }
}
