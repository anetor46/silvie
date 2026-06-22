//! Out-of-stream execution of a previously-deferred write tool call.
//!
//! When a write tool is called during a chat stream, the `ToolWrapper` only
//! records its intent (args persisted to DB, ToolCall event emitted) — the
//! actual side-effect doesn't run until the user approves via
//! `/chat/tool-responses`. That endpoint calls into this module to do the
//! deferred work, then resumes the conversation with the real result in
//! history.
//!
//! ⚠️ **Keep this in sync with the write tools registered with the agent in
//! `LlmClient::add_*_tools`.** If a write tool is added there but not here,
//! the user will see the confirmation card but clicking Approve will fail
//! with "unknown write tool". The same applies to `friendly_action` in
//! `api::tool_responses` — every write tool needs an entry there too.

use anyhow::{anyhow, Result};
use rig::tool::Tool;
use serde::Serialize;
use tracing::error;

use crate::config::{Config, StripeConfig};
use crate::db::DbPool;
use crate::llm::context::{StripePaymentRefs, ToolAuth};
use crate::tools::gmail::{ReplyToEmailTool, SendEmailTool};
use crate::tools::google_calendar::{
    CreateCalendarEventTool, DeleteCalendarEventTool, RespondToEventTool, UpdateCalendarEventTool,
};
use crate::tools::outlook::{
    CreateOutlookEventTool, DeleteOutlookEventTool, ReplyOutlookEmailTool, RespondOutlookEventTool,
    SendOutlookEmailTool, UpdateOutlookEventTool,
};
use crate::tools::travelport::{HotelBookTool, HotelBookToolDeps, HotelCancelBookingTool};

/// Result of a deferred tool execution. Both fields end up in the DB row's
/// updated payload + the SSE `ToolResult` event the frontend sees.
pub struct ExecOutcome {
    pub success: bool,
    pub summary: Option<String>,
    pub output: Option<serde_json::Value>,
}

/// Run the write tool identified by `tool_name` with the previously-stored
/// `args_json`. The caller is responsible for persisting the outcome to the
/// DB and emitting the `ToolResult` event.
pub async fn execute_pending(
    config: &Config,
    db_pool: &DbPool,
    tool_auth: &ToolAuth,
    tool_name: &str,
    args_json: &serde_json::Value,
) -> Result<ExecOutcome> {
    match tool_name {
        // ── Gmail ───────────────────────────────────────────────────────────
        "send_email" => {
            let token = require_google(tool_auth)?;
            let tool = SendEmailTool::new(token);
            run(&tool, args_json).await
        }
        "reply_to_email" => {
            let token = require_google(tool_auth)?;
            let tool = ReplyToEmailTool::new(token);
            run(&tool, args_json).await
        }
        // ── Google Calendar ─────────────────────────────────────────────────
        "create_calendar_event" => {
            let token = require_google(tool_auth)?;
            let tool = CreateCalendarEventTool::new(token);
            run(&tool, args_json).await
        }
        "update_calendar_event" => {
            let token = require_google(tool_auth)?;
            let tool = UpdateCalendarEventTool::new(token);
            run(&tool, args_json).await
        }
        "delete_calendar_event" => {
            let token = require_google(tool_auth)?;
            let tool = DeleteCalendarEventTool::new(token);
            run(&tool, args_json).await
        }
        "respond_to_event" => {
            let token = require_google(tool_auth)?;
            let tool = RespondToEventTool::new(token);
            run(&tool, args_json).await
        }
        // ── Outlook ─────────────────────────────────────────────────────────
        "send_outlook_email" => {
            let token = require_outlook(tool_auth)?;
            let tool = SendOutlookEmailTool::new(token);
            run(&tool, args_json).await
        }
        "reply_outlook_email" => {
            let token = require_outlook(tool_auth)?;
            let tool = ReplyOutlookEmailTool::new(token);
            run(&tool, args_json).await
        }
        "create_outlook_event" => {
            let token = require_outlook(tool_auth)?;
            let tool = CreateOutlookEventTool::new(token);
            run(&tool, args_json).await
        }
        "update_outlook_event" => {
            let token = require_outlook(tool_auth)?;
            let tool = UpdateOutlookEventTool::new(token);
            run(&tool, args_json).await
        }
        "delete_outlook_event" => {
            let token = require_outlook(tool_auth)?;
            let tool = DeleteOutlookEventTool::new(token);
            run(&tool, args_json).await
        }
        "respond_outlook_event" => {
            let token = require_outlook(tool_auth)?;
            let tool = RespondOutlookEventTool::new(token);
            run(&tool, args_json).await
        }
        // ── Travelport ──────────────────────────────────────────────────────
        "hotel_book" => {
            let stripe: &StripeConfig = config
                .stripe
                .as_ref()
                .ok_or_else(|| anyhow!("Stripe is not configured"))?;
            let tp = tool_auth
                .travelport
                .as_ref()
                .ok_or_else(|| anyhow!("Travelport is not configured"))?;
            let pm: &StripePaymentRefs = tool_auth
                .stripe_payment
                .as_ref()
                .ok_or_else(|| anyhow!("no saved payment method on file"))?;
            let user_id = tool_auth
                .user_id
                .ok_or_else(|| anyhow!("authenticated user missing from tool auth"))?;
            let tool = HotelBookTool::new(HotelBookToolDeps {
                travelport: tp.clone(),
                stripe_key: stripe.secret_key.clone(),
                customer_id: pm.customer_id.clone(),
                payment_method_id: pm.payment_method_id.clone(),
                user_id,
                conversation_id: tool_auth.conversation_id,
                db_pool: db_pool.clone(),
            });
            run(&tool, args_json).await
        }
        "hotel_cancel_booking" => {
            let stripe: &StripeConfig = config
                .stripe
                .as_ref()
                .ok_or_else(|| anyhow!("Stripe is not configured"))?;
            let tp = tool_auth
                .travelport
                .as_ref()
                .ok_or_else(|| anyhow!("Travelport is not configured"))?;
            let user_id = tool_auth
                .user_id
                .ok_or_else(|| anyhow!("authenticated user missing from tool auth"))?;
            let tool = HotelCancelBookingTool::new(
                tp.clone(),
                stripe.secret_key.clone(),
                db_pool.clone(),
                user_id,
            );
            run(&tool, args_json).await
        }
        _ => Err(anyhow!("unknown write tool: {tool_name}")),
    }
}

fn require_google(tool_auth: &ToolAuth) -> Result<String> {
    tool_auth
        .google_access_token
        .clone()
        .ok_or_else(|| anyhow!("Google access token unavailable"))
}

fn require_outlook(tool_auth: &ToolAuth) -> Result<String> {
    tool_auth
        .outlook_access_token
        .clone()
        .ok_or_else(|| anyhow!("Outlook access token unavailable"))
}

/// Generic helper that decodes the stored args JSON into the tool's typed
/// `Args`, runs it, and packages the result into an `ExecOutcome`.
async fn run<T>(tool: &T, args_json: &serde_json::Value) -> Result<ExecOutcome>
where
    T: Tool,
    T::Output: Serialize,
{
    let args: T::Args = match serde_json::from_value(args_json.clone()) {
        Ok(a) => a,
        Err(e) => {
            error!("failed to decode tool args: {e:#}");
            return Ok(ExecOutcome {
                success: false,
                summary: Some(format!("invalid arguments: {e}")),
                output: None,
            });
        }
    };
    match tool.call(args).await {
        Ok(out) => {
            let output_json = serde_json::to_value(&out).ok();
            Ok(ExecOutcome {
                success: true,
                summary: None,
                output: output_json,
            })
        }
        Err(e) => {
            let msg = e.to_string();
            error!("deferred tool execution failed: {msg}");
            Ok(ExecOutcome {
                success: false,
                summary: Some(msg),
                output: None,
            })
        }
    }
}
