//! Thin wrapper over rig's Gemini provider that turns a `Vec<ChatMessage>` into a
//! token stream of `String` chunks. Isolated here so the rest of the server is
//! provider-agnostic.

use anyhow::{anyhow, Result};
use futures::Stream;
use rig::{
    completion::{Message, Prompt},
    providers::gemini,
    streaming::{StreamingChat, StreamingChoice},
};
use std::pin::Pin;
use tokio_stream::StreamExt;

use crate::types::{ChatMessage, Role};

const MODEL: &str = "gemini-2.0-flash";

pub struct LlmClient {
    client: gemini::Client,
}

impl LlmClient {
    pub fn new(api_key: &str) -> Self {
        Self {
            client: gemini::Client::new(api_key),
        }
    }

    /// Stream a chat completion. Returns a Stream of text chunks.
    pub async fn stream_chat(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
        let (system_prompt, history, prompt) = split_history(&messages)?;

        let mut builder = self
            .client
            .agent(MODEL);

        if let Some(sys) = system_prompt {
            builder = builder.preamble(&sys);
        }

        let agent = builder.build();

        let stream = agent
            .stream_chat(&prompt, history)
            .await
            .map_err(|e| anyhow!("rig stream_chat failed: {e}"))?;

        let mapped = stream.map(|item| match item {
            Ok(StreamingChoice::Message(t)) => Ok(t),
            Ok(StreamingChoice::ToolCall(_, _, _)) => Ok(String::new()), // ignore tool calls for v1
            Err(e) => Err(anyhow!("rig stream error: {e}")),
        });

        Ok(Box::pin(mapped))
    }
}

/// Splits `[system?, user, assistant, user, ..., user]` into:
/// - the system prompt (optional)
/// - the chat history (everything except the last message), as rig `Message`s
/// - the final user prompt
///
/// Errors if the final message isn't from the user, or if there are no user messages.
fn split_history(messages: &[ChatMessage]) -> Result<(Option<String>, Vec<Message>, String)> {
    if messages.is_empty() {
        return Err(anyhow!("messages array is empty"));
    }

    let mut system_prompt: Option<String> = None;
    let mut rest: Vec<&ChatMessage> = Vec::with_capacity(messages.len());

    for m in messages {
        match m.role {
            Role::System => {
                // Concatenate multiple system messages if present.
                system_prompt = Some(match system_prompt {
                    Some(s) => format!("{s}\n\n{}", m.content),
                    None => m.content.clone(),
                });
            }
            _ => rest.push(m),
        }
    }

    let last = rest
        .last()
        .ok_or_else(|| anyhow!("no user/assistant messages provided"))?;

    if !matches!(last.role, Role::User) {
        return Err(anyhow!("the final message must be a user message"));
    }

    let prompt = last.content.clone();

    let history: Vec<Message> = rest[..rest.len() - 1]
        .iter()
        .map(|m| match m.role {
            Role::User => Message {
                role: "user".into(),
                content: m.content.clone(),
            },
            Role::Assistant => Message {
                role: "assistant".into(),
                content: m.content.clone(),
            },
            Role::System => unreachable!("system messages have been filtered above"),
        })
        .collect();

    Ok((system_prompt, history, prompt))
}
