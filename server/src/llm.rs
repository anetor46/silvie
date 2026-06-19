use anyhow::{anyhow, Result};
use futures::{Stream, StreamExt};
use rig::{
    agent::MultiTurnStreamItem,
    client::CompletionClient,
    completion::Message,
    providers::gemini,
    streaming::{StreamedAssistantContent, StreamingChat},
    tool::ToolDyn,
};
use std::pin::Pin;

use crate::calendar::{
    CreateCalendarEventTool, DeleteCalendarEventTool, GoogleCalendarTool, RespondToEventTool,
    UpdateCalendarEventTool,
};
use crate::types::{ChatMessage, Role};

const MODEL: &str = "gemini-3.1-flash-lite";

const CALENDAR_PREAMBLE_TEMPLATE: &str =
    include_str!("../prompts/calendar_preamble.md");

fn build_calendar_preamble(timezone: Option<&str>, current_datetime: Option<&str>) -> String {
    let datetime_context = match (current_datetime, timezone) {
        (Some(dt), Some(tz)) => format!("{dt} ({tz})"),
        (Some(dt), None) => dt.to_string(),
        _ => "unknown".to_string(),
    };
    CALENDAR_PREAMBLE_TEMPLATE
        .replace("{{CURRENT_DATETIME}}", &datetime_context)
}

pub struct LlmClient {
    client: gemini::Client,
}

impl LlmClient {
    pub fn new(api_key: &str) -> Self {
        Self {
            client: gemini::Client::new(api_key).expect("failed to build Gemini client"),
        }
    }

    /// Stream a chat completion. Returns a Stream of text chunks.
    /// If `google_access_token` is provided the agent gains a Google Calendar tool.
    pub async fn stream_chat(
        &self,
        messages: Vec<ChatMessage>,
        google_access_token: Option<String>,
        timezone: Option<String>,
        current_datetime: Option<String>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
        let (system_prompt, history, prompt) = split_history(&messages)?;

        let mut preamble = system_prompt.unwrap_or_default();
        let mut tools: Vec<Box<dyn ToolDyn>> = Vec::new();

        if let Some(token) = google_access_token {
            let cal_preamble =
                build_calendar_preamble(timezone.as_deref(), current_datetime.as_deref());
            if preamble.is_empty() {
                preamble = cal_preamble;
            } else {
                preamble.push_str("\n\n");
                preamble.push_str(&cal_preamble);
            }
            tools.push(Box::new(GoogleCalendarTool::new(token.clone())));
            tools.push(Box::new(CreateCalendarEventTool::new(token.clone())));
            tools.push(Box::new(UpdateCalendarEventTool::new(token.clone())));
            tools.push(Box::new(DeleteCalendarEventTool::new(token.clone())));
            tools.push(Box::new(RespondToEventTool::new(token)));
        }

        let mut builder = self.client.agent(MODEL);
        if !preamble.is_empty() {
            builder = builder.preamble(&preamble);
        }

        let agent = builder.default_max_turns(10).tools(tools).build();
        let stream = agent.stream_chat(prompt, history).await;

        let mapped = stream.filter_map(|item| async move {
            match item {
                Ok(MultiTurnStreamItem::StreamAssistantItem(
                    StreamedAssistantContent::Text(text),
                )) => Some(Ok(text.text)),
                Ok(_) => None,
                Err(e) => Some(Err(anyhow!("rig stream error: {e}"))),
            }
        });

        Ok(Box::pin(mapped))
    }
}

/// Splits `[system?, user, assistant, user, ..., user]` into:
/// - the system prompt (optional)
/// - the chat history (everything except the last message), as rig `Message`s
/// - the final user prompt
fn split_history(messages: &[ChatMessage]) -> Result<(Option<String>, Vec<Message>, String)> {
    if messages.is_empty() {
        return Err(anyhow!("messages array is empty"));
    }

    let mut system_prompt: Option<String> = None;
    let mut rest: Vec<&ChatMessage> = Vec::with_capacity(messages.len());

    for m in messages {
        match m.role {
            Role::System => {
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
            Role::User => Message::user(m.content.clone()),
            Role::Assistant => Message::assistant(m.content.clone()),
            Role::System => unreachable!("system messages filtered above"),
        })
        .collect();

    Ok((system_prompt, history, prompt))
}
