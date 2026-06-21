use std::pin::Pin;

use anyhow::{anyhow, Result};
use futures::{stream, Stream, StreamExt};
use rig::{
    agent::MultiTurnStreamItem,
    client::CompletionClient,
    providers::gemini,
    streaming::{StreamedAssistantContent, StreamingChat},
    tool::ToolDyn,
};
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::config::{Config, StripeConfig, TravelportCredentials};
use crate::db::DbPool;
use crate::tools::gmail::{GetEmailTool, ListEmailsTool, ReplyToEmailTool, SendEmailTool};
use crate::tools::google_calendar::{
    CreateCalendarEventTool, DeleteCalendarEventTool, FindFreeTimeTool, GoogleCalendarTool,
    RespondToEventTool, UpdateCalendarEventTool,
};
use crate::tools::outlook::{
    CreateOutlookEventTool, DeleteOutlookEventTool, FindOutlookFreeTimeTool, GetOutlookEmailTool,
    ListOutlookEmailsTool, ListOutlookEventsTool, ReplyOutlookEmailTool, RespondOutlookEventTool,
    SendOutlookEmailTool, UpdateOutlookEventTool,
};
use crate::tools::travelport::{HotelBookTool, HotelSearchTool};
use crate::types::{ChatEvent, ToolEvent};

use super::context::{ChatTurn, LocaleContext, StripePaymentRefs, ToolAuth};
use super::harness::ToolWrapper;

const DEFAULT_MODEL: &str = "gemini-3.1-flash-lite";

const CALENDAR_PREAMBLE_TEMPLATE: &str = include_str!("../../prompts/google_calendar/preamble.md");
const GMAIL_PREAMBLE: &str = include_str!("../../prompts/gmail/preamble.md");
const OUTLOOK_PREAMBLE_TEMPLATE: &str = include_str!("../../prompts/outlook/preamble.md");
const HOTEL_PREAMBLE: &str = include_str!("../../prompts/travelport/preamble.md");
const CONFIRMATION_PREAMBLE: &str = include_str!("../../prompts/confirmation/preamble.md");

/// LLM client with all persistent dependencies attached. One instance per
/// process; cloning is cheap.
pub struct LlmClient {
    gemini: gemini::Client,
    model: String,
    db_pool: DbPool,
    stripe: Option<StripeConfig>,
    travelport: Option<TravelportCredentials>,
}

impl LlmClient {
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

    /// Stream one chat turn. The caller has already reconstructed the history
    /// (including past tool calls + results) and supplies the immediate
    /// `prompt` (either real user text or a synthesized continuation marker
    /// after a tool response). Returns a stream of `ChatEvent`s.
    pub async fn stream(
        &self,
        turn: ChatTurn,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatEvent>> + Send>>> {
        let mut preamble = String::new();
        let mut tools: Vec<Box<dyn ToolDyn>> = Vec::new();

        // The harness preamble is always present — it explains the
        // `awaiting_user_input` marker to the model.
        push_preamble(&mut preamble, CONFIRMATION_PREAMBLE);

        let (tool_tx, tool_rx) = tokio::sync::mpsc::unbounded_channel::<ToolEvent>();

        self.add_calendar_tools(&mut preamble, &mut tools, &turn.locale, &turn.tool_auth, &tool_tx);
        self.add_outlook_tools(&mut preamble, &mut tools, &turn.locale, &turn.tool_auth, &tool_tx);
        self.add_travel_tools(&mut preamble, &mut tools, &turn.tool_auth, &tool_tx);

        let mut builder = self.gemini.agent(&self.model);
        if !preamble.is_empty() {
            builder = builder.preamble(&preamble);
        }

        let agent = builder.default_max_turns(10).tools(tools).build();
        let stream = agent.stream_chat(turn.prompt, turn.history).await;

        // Forward rig text events; tool events arrive via the side channel.
        let rig_mapped = stream.filter_map(|item| async move {
            match item {
                Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Text(
                    text,
                ))) => Some(Ok(ChatEvent::Text(text.text))),
                Ok(_) => None,
                Err(e) => Some(Err(anyhow!("rig stream error: {e}"))),
            }
        });

        let tool_stream = UnboundedReceiverStream::new(tool_rx)
            .map(|ev| Ok::<ChatEvent, anyhow::Error>(ChatEvent::from(ev)));
        let merged = stream::select(rig_mapped, tool_stream);
        Ok(Box::pin(merged))
    }

    fn add_calendar_tools(
        &self,
        preamble: &mut String,
        tools: &mut Vec<Box<dyn ToolDyn>>,
        locale: &LocaleContext,
        auth: &ToolAuth,
        tool_tx: &tokio::sync::mpsc::UnboundedSender<ToolEvent>,
    ) {
        let Some(token) = auth.google_access_token.clone() else {
            return;
        };
        push_preamble(preamble, &build_calendar_preamble(locale));
        push_preamble(preamble, GMAIL_PREAMBLE);

        // Read tools.
        tools.push(Box::new(ToolWrapper::new_read(
            GoogleCalendarTool::new(token.clone()),
            tool_tx.clone(),
        )));
        tools.push(Box::new(ToolWrapper::new_read(
            FindFreeTimeTool::new(token.clone()),
            tool_tx.clone(),
        )));
        tools.push(Box::new(ToolWrapper::new_read(
            ListEmailsTool::new(token.clone()),
            tool_tx.clone(),
        )));
        tools.push(Box::new(ToolWrapper::new_read(
            GetEmailTool::new(token.clone()),
            tool_tx.clone(),
        )));

        // Write tools — emit only; execution is deferred to /chat/tool-responses.
        tools.push(Box::new(ToolWrapper::new_write(
            CreateCalendarEventTool::new(token.clone()),
            tool_tx.clone(),
        )));
        tools.push(Box::new(ToolWrapper::new_write(
            UpdateCalendarEventTool::new(token.clone()),
            tool_tx.clone(),
        )));
        tools.push(Box::new(ToolWrapper::new_write(
            DeleteCalendarEventTool::new(token.clone()),
            tool_tx.clone(),
        )));
        tools.push(Box::new(ToolWrapper::new_write(
            RespondToEventTool::new(token.clone()),
            tool_tx.clone(),
        )));
        tools.push(Box::new(ToolWrapper::new_write(
            SendEmailTool::new(token.clone()),
            tool_tx.clone(),
        )));
        tools.push(Box::new(ToolWrapper::new_write(
            ReplyToEmailTool::new(token),
            tool_tx.clone(),
        )));
    }

    fn add_outlook_tools(
        &self,
        preamble: &mut String,
        tools: &mut Vec<Box<dyn ToolDyn>>,
        locale: &LocaleContext,
        auth: &ToolAuth,
        tool_tx: &tokio::sync::mpsc::UnboundedSender<ToolEvent>,
    ) {
        let Some(token) = auth.outlook_access_token.clone() else {
            return;
        };
        push_preamble(preamble, &build_outlook_preamble(locale));

        // Read tools.
        tools.push(Box::new(ToolWrapper::new_read(
            ListOutlookEmailsTool::new(token.clone()),
            tool_tx.clone(),
        )));
        tools.push(Box::new(ToolWrapper::new_read(
            GetOutlookEmailTool::new(token.clone()),
            tool_tx.clone(),
        )));
        tools.push(Box::new(ToolWrapper::new_read(
            ListOutlookEventsTool::new(token.clone()),
            tool_tx.clone(),
        )));
        tools.push(Box::new(ToolWrapper::new_read(
            FindOutlookFreeTimeTool::new(token.clone()),
            tool_tx.clone(),
        )));

        // Write tools — deferred to /chat/tool-responses.
        tools.push(Box::new(ToolWrapper::new_write(
            SendOutlookEmailTool::new(token.clone()),
            tool_tx.clone(),
        )));
        tools.push(Box::new(ToolWrapper::new_write(
            ReplyOutlookEmailTool::new(token.clone()),
            tool_tx.clone(),
        )));
        tools.push(Box::new(ToolWrapper::new_write(
            CreateOutlookEventTool::new(token.clone()),
            tool_tx.clone(),
        )));
        tools.push(Box::new(ToolWrapper::new_write(
            UpdateOutlookEventTool::new(token.clone()),
            tool_tx.clone(),
        )));
        tools.push(Box::new(ToolWrapper::new_write(
            DeleteOutlookEventTool::new(token.clone()),
            tool_tx.clone(),
        )));
        tools.push(Box::new(ToolWrapper::new_write(
            RespondOutlookEventTool::new(token),
            tool_tx.clone(),
        )));
    }

    fn add_travel_tools(
        &self,
        preamble: &mut String,
        tools: &mut Vec<Box<dyn ToolDyn>>,
        auth: &ToolAuth,
        tool_tx: &tokio::sync::mpsc::UnboundedSender<ToolEvent>,
    ) {
        let Some(tp) = self.travelport.as_ref() else {
            return;
        };
        push_preamble(preamble, HOTEL_PREAMBLE);
        tools.push(Box::new(ToolWrapper::new_read(
            HotelSearchTool::new(tp.client_id.clone(), tp.client_secret.clone()),
            tool_tx.clone(),
        )));

        if let (Some(stripe), Some(pm)) = (self.stripe.as_ref(), auth.stripe_payment.as_ref()) {
            let StripePaymentRefs {
                customer_id,
                payment_method_id,
            } = pm.clone();
            tools.push(Box::new(ToolWrapper::new_write(
                HotelBookTool::new(
                    tp.client_id.clone(),
                    tp.client_secret.clone(),
                    stripe.secret_key.clone(),
                    customer_id,
                    payment_method_id,
                    self.db_pool.clone(),
                ),
                tool_tx.clone(),
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

fn build_outlook_preamble(locale: &LocaleContext) -> String {
    let datetime_context = match (locale.current_datetime.as_deref(), locale.timezone.as_deref()) {
        (Some(dt), Some(tz)) => format!("{dt} ({tz})"),
        (Some(dt), None) => dt.to_string(),
        _ => "unknown".to_string(),
    };
    OUTLOOK_PREAMBLE_TEMPLATE.replace("{{CURRENT_DATETIME}}", &datetime_context)
}

fn build_calendar_preamble(locale: &LocaleContext) -> String {
    let datetime_context = match (locale.current_datetime.as_deref(), locale.timezone.as_deref()) {
        (Some(dt), Some(tz)) => format!("{dt} ({tz})"),
        (Some(dt), None) => dt.to_string(),
        _ => "unknown".to_string(),
    };
    CALENDAR_PREAMBLE_TEMPLATE.replace("{{CURRENT_DATETIME}}", &datetime_context)
}
