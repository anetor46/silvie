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

use crate::config::{Config, StripeConfig, TravelportCredentials};
use crate::db::DbPool;
use crate::tools::google_calendar::{
    CreateCalendarEventTool, DeleteCalendarEventTool, FindFreeTimeTool, GoogleCalendarTool,
    RespondToEventTool, UpdateCalendarEventTool,
};
use crate::tools::travelport::{HotelBookTool, HotelSearchTool};
use crate::types::{ChatMessage, Role};

use super::context::{ChatTurn, LocaleContext, StripePaymentRefs, ToolAuth};

const DEFAULT_MODEL: &str = "gemini-3.1-flash-lite";

const CALENDAR_PREAMBLE_TEMPLATE: &str = include_str!("../../prompts/google_calendar/preamble.md");
const HOTEL_PREAMBLE: &str = include_str!("../../prompts/travelport/preamble.md");

/// LLM client with all persistent dependencies attached. One instance per
/// process; cloning is cheap (everything is `Arc`-internal or `Copy`-ish).
pub struct LlmClient {
    gemini: gemini::Client,
    model: String,
    db_pool: DbPool,
    stripe: Option<StripeConfig>,
    travelport: Option<TravelportCredentials>,
}

impl LlmClient {
    /// Build the client. Returns an error if the Gemini SDK can't initialise
    /// (rather than panicking like the previous `new()` did).
    pub fn new(api_key: &str, config: &Config, db_pool: DbPool) -> Result<Self> {
        let gemini = gemini::Client::new(api_key)
            .map_err(|e| anyhow!("failed to build Gemini client: {e}"))?;
        Ok(Self {
            gemini,
            model: DEFAULT_MODEL.to_string(),
            db_pool,
            stripe: config.stripe.clone(),
            travelport: config.travelport.clone(),
        })
    }

    /// Stream one chat turn. Returns a stream of text chunks (tool calls
    /// are handled internally by `rig`).
    pub async fn stream(
        &self,
        turn: ChatTurn,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
        let (system_prompt, history, prompt) = split_history(&turn.messages)?;
        let mut preamble = system_prompt.unwrap_or_default();
        let mut tools: Vec<Box<dyn ToolDyn>> = Vec::new();

        self.add_calendar_tools(&mut preamble, &mut tools, &turn.locale, &turn.tool_auth);
        self.add_travel_tools(&mut preamble, &mut tools, &turn.tool_auth);

        let mut builder = self.gemini.agent(&self.model);
        if !preamble.is_empty() {
            builder = builder.preamble(&preamble);
        }

        let agent = builder.default_max_turns(10).tools(tools).build();
        let stream = agent.stream_chat(prompt, history).await;

        let mapped = stream.filter_map(|item| async move {
            match item {
                Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Text(
                    text,
                ))) => Some(Ok(text.text)),
                Ok(_) => None,
                Err(e) => Some(Err(anyhow!("rig stream error: {e}"))),
            }
        });
        Ok(Box::pin(mapped))
    }

    fn add_calendar_tools(
        &self,
        preamble: &mut String,
        tools: &mut Vec<Box<dyn ToolDyn>>,
        locale: &LocaleContext,
        auth: &ToolAuth,
    ) {
        let Some(token) = auth.google_access_token.clone() else {
            return;
        };
        push_preamble(preamble, &build_calendar_preamble(locale));
        tools.push(Box::new(GoogleCalendarTool::new(token.clone())));
        tools.push(Box::new(CreateCalendarEventTool::new(token.clone())));
        tools.push(Box::new(UpdateCalendarEventTool::new(token.clone())));
        tools.push(Box::new(DeleteCalendarEventTool::new(token.clone())));
        tools.push(Box::new(RespondToEventTool::new(token.clone())));
        tools.push(Box::new(FindFreeTimeTool::new(token)));
    }

    fn add_travel_tools(
        &self,
        preamble: &mut String,
        tools: &mut Vec<Box<dyn ToolDyn>>,
        auth: &ToolAuth,
    ) {
        let Some(tp) = self.travelport.as_ref() else {
            return;
        };
        push_preamble(preamble, HOTEL_PREAMBLE);
        tools.push(Box::new(HotelSearchTool::new(
            tp.client_id.clone(),
            tp.client_secret.clone(),
        )));

        // Hotel booking additionally requires Stripe + a user-side payment method.
        if let (Some(stripe), Some(pm)) = (self.stripe.as_ref(), auth.stripe_payment.as_ref()) {
            let StripePaymentRefs {
                customer_id,
                payment_method_id,
            } = pm.clone();
            tools.push(Box::new(HotelBookTool::new(
                tp.client_id.clone(),
                tp.client_secret.clone(),
                stripe.secret_key.clone(),
                customer_id,
                payment_method_id,
                self.db_pool.clone(),
            )));
        }
    }
}

fn push_preamble(preamble: &mut String, addition: &str) {
    if preamble.is_empty() {
        preamble.push_str(addition);
    } else {
        preamble.push_str("\n\n");
        preamble.push_str(addition);
    }
}

fn build_calendar_preamble(locale: &LocaleContext) -> String {
    let datetime_context = match (locale.current_datetime.as_deref(), locale.timezone.as_deref()) {
        (Some(dt), Some(tz)) => format!("{dt} ({tz})"),
        (Some(dt), None) => dt.to_string(),
        _ => "unknown".to_string(),
    };
    CALENDAR_PREAMBLE_TEMPLATE.replace("{{CURRENT_DATETIME}}", &datetime_context)
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
