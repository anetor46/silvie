//! `POST /chat/tool-responses` — the frontend posts user input (currently:
//! Approve / Reject) for a pending tool call here. The endpoint:
//!
//! 1. Looks up the pending tool call (must belong to the authenticated user).
//! 2. Executes the deferred tool (if approved) or marks it rejected.
//! 3. Persists the real result + emits a `ToolResult` SSE event so the
//!    waiting tool card in the UI updates.
//! 4. Reconstructs the full conversation history including this just-resolved
//!    tool call, synthesises a continuation prompt, and resumes the agent
//!    stream — emitting further text / tool events over the same SSE.
//!
//! The wire body is an extensible enum so future input kinds (multi-choice,
//! survey, free-form text) plug in without protocol churn.

use std::sync::Arc;

use futures::stream::StreamExt;
use poem::{
    handler,
    web::{sse::SSE, Data, Json},
};
use serde::Deserialize;
use tracing::{debug, error, info};

use crate::{
    api::chat::{build_tool_auth, run_turn, sse_events, update_tool_status_in_db},
    auth::AuthUser,
    config::Config,
    db::DbPool,
    error::{ApiError, ApiResult},
    llm::{history::db_rows_to_rig_history, tool_dispatch, ChatTurn, LlmClient, LocaleContext},
    repos::{conversations, integrations::IntegrationsConfig},
    types::SseEvent,
};

#[derive(Debug, Deserialize)]
pub struct ToolResponseRequest {
    pub call_id: String,
    pub response: ToolResponse,
    /// Optional locale context (same shape as ChatRequest) so the resumed
    /// turn has the right datetime/timezone for any new tool calls.
    #[serde(default)]
    pub timezone: Option<String>,
    #[serde(default)]
    pub current_datetime: Option<String>,
}

/// Extensible — `kind` discriminates. Today: `confirmation`. Tomorrow:
/// `choice`, `survey`, `free_form`, etc.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ToolResponse {
    Confirmation {
        approved: bool,
        #[serde(default)]
        reason: Option<String>,
    },
}

#[handler]
pub async fn tool_response_handler(
    auth: AuthUser,
    Data(client): Data<&Arc<LlmClient>>,
    Data(pool): Data<&DbPool>,
    Data(integ_cfg): Data<&Arc<IntegrationsConfig>>,
    Data(config): Data<&Arc<Config>>,
    Json(req): Json<ToolResponseRequest>,
) -> ApiResult<SSE> {
    debug!(call_id = %req.call_id, "/chat/tool-responses received");

    // 1. Look up the pending tool row. Must belong to a conversation the
    //    user owns AND must still be pending.
    let pending = conversations::find_pending_tool_for_user(pool, auth.user.id, &req.call_id)
        .await
        .map_err(ApiError::from)?
        .ok_or(ApiError::NotFound)?;

    // 2. Build tool auth + locale upfront (used by both the execute step and
    //    the resumed agent stream).
    let tool_auth = build_tool_auth(pool, integ_cfg, auth.user.id).await;
    let locale = LocaleContext {
        timezone: req.timezone,
        current_datetime: req.current_datetime,
    };

    // 3. Execute (or reject) the pending call. Persist the resulting row.
    let (success, summary, output_json, continuation_prompt) = match &req.response {
        ToolResponse::Confirmation { approved: false, reason } => {
            let summary = reason
                .clone()
                .map(|r| format!("rejected: {r}"))
                .unwrap_or_else(|| "rejected by user".into());
            info!(call_id = %req.call_id, "tool call rejected by user");
            (false, Some(summary.clone()), None, rejection_prompt(&pending.tool_name, reason.as_deref()))
        }
        ToolResponse::Confirmation { approved: true, .. } => {
            info!(call_id = %req.call_id, "tool call approved by user; executing");
            match tool_dispatch::execute_pending(
                config,
                pool,
                &tool_auth,
                &pending.tool_name,
                &pending.args,
            )
            .await
            {
                Ok(outcome) => {
                    let prompt = if outcome.success {
                        approval_prompt(&pending.tool_name, outcome.summary.as_deref())
                    } else {
                        failure_prompt(&pending.tool_name, outcome.summary.as_deref())
                    };
                    (outcome.success, outcome.summary, outcome.output, prompt)
                }
                Err(e) => {
                    error!(call_id = %req.call_id, "dispatch error: {e:#}");
                    let summary = format!("execution failed: {e}");
                    (
                        false,
                        Some(summary.clone()),
                        None,
                        failure_prompt(&pending.tool_name, Some(&summary)),
                    )
                }
            }
        }
    };

    if let Err(e) = update_tool_status_in_db(
        pool,
        pending.conversation_id,
        &req.call_id,
        success,
        summary.clone(),
        output_json.clone(),
    )
    .await
    {
        error!("failed to persist tool result: {e:#}");
    }

    // 4. Build the SSE stream. Prepend a `ToolResult` event so the UI
    //    card updates immediately, then concat the agent continuation.
    let output_for_event = output_json.clone();
    let initial = futures::stream::once(async move {
        SseEvent::ToolResult {
            call_id: req.call_id.clone(),
            success,
            summary,
            output: output_for_event,
        }
    });

    // 5. Build a continuation ChatTurn whose history includes the
    //    now-resolved tool call + result. The prompt is the synthesised
    //    update describing what happened.
    let rows = conversations::load_history(pool, pending.conversation_id)
        .await
        .map_err(ApiError::from)?;
    let history = db_rows_to_rig_history(&rows);
    let turn = ChatTurn {
        history,
        prompt: continuation_prompt,
        locale,
        tool_auth,
    };

    let downstream = run_turn(client.clone(), pool.clone(), pending.conversation_id, turn);

    let combined = initial.chain(downstream);
    Ok(SSE::new(sse_events(combined)).keep_alive(std::time::Duration::from_secs(15)))
}

/// Human-readable description of what a write tool *does*. Used in the
/// synthesised continuation prompts so the model talks about "sending
/// the email" instead of echoing the raw `send_email` identifier at the
/// user.
///
/// ⚠️ Keep this in sync with the write tools in `llm::client::add_*_tools`
/// and `llm::tool_dispatch::execute_pending`.
fn friendly_action(tool_name: &str) -> &'static str {
    match tool_name {
        // Gmail
        "send_email" => "sending the email",
        "reply_to_email" => "replying to the email",
        // Google Calendar
        "create_calendar_event" => "creating the calendar event",
        "update_calendar_event" => "updating the calendar event",
        "delete_calendar_event" => "deleting the calendar event",
        "respond_to_event" => "responding to the calendar invitation",
        // Outlook
        "send_outlook_email" => "sending the Outlook email",
        "reply_outlook_email" => "replying to the Outlook email",
        "create_outlook_event" => "creating the Outlook calendar event",
        "update_outlook_event" => "updating the Outlook calendar event",
        "delete_outlook_event" => "deleting the Outlook calendar event",
        "respond_outlook_event" => "responding to the Outlook calendar invitation",
        // Travelport
        "hotel_book" => "booking the hotel",
        _ => "the requested action",
    }
}

fn approval_prompt(tool_name: &str, summary: Option<&str>) -> String {
    let action = friendly_action(tool_name);
    match summary {
        Some(s) => format!(
            "[The user approved {action}. It completed successfully: {s}. Acknowledge briefly and continue.]"
        ),
        None => format!(
            "[The user approved {action} and it completed successfully. Acknowledge briefly and continue.]"
        ),
    }
}

fn rejection_prompt(tool_name: &str, reason: Option<&str>) -> String {
    let action = friendly_action(tool_name);
    match reason {
        Some(r) => format!(
            "[The user rejected {action} with the reason: \"{r}\". Acknowledge and offer an alternative if appropriate.]"
        ),
        None => format!(
            "[The user decided not to proceed with {action}. The action was NOT performed. Acknowledge and offer an alternative if appropriate.]"
        ),
    }
}

fn failure_prompt(tool_name: &str, error: Option<&str>) -> String {
    let action = friendly_action(tool_name);
    match error {
        Some(e) => format!(
            "[{action} was approved but FAILED to execute: {e}. Apologise and either retry with different parameters or offer an alternative.]"
        ),
        None => format!(
            "[{action} was approved but FAILED to execute. Apologise and either retry or offer an alternative.]"
        ),
    }
}
