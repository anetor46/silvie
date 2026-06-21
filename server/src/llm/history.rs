//! Convert DB message rows into rig `Message`s for the agent's context.
//!
//! Gemini (and other providers) require a specific message structure:
//! every assistant message containing a tool call must be **immediately**
//! preceded by a user message OR a tool result, and every assistant turn
//! must be ONE message — even when it contains both text and one or more
//! tool calls. Emitting separate `Message::Assistant{Text}` then
//! `Message::Assistant{ToolCall}` items breaks the contract and Gemini
//! rejects the request on the next turn.
//!
//! So this module groups consecutive `assistant` + `tool` DB rows into a
//! single `Message::Assistant` carrying all their contents, followed by a
//! single `Message::User` carrying every tool result for that turn.

use rig::completion::{
    message::{AssistantContent, ToolCall, ToolFunction, ToolResult, ToolResultContent, UserContent},
    Message as RigMessage,
};
use rig::OneOrMany;

use crate::repos::conversations::{Message as DbMessage, ToolMessageContent};

/// Convert an ordered slice of DB rows into rig `Message`s. The output is
/// Gemini-compliant: each assistant message contains text + tool calls
/// together; tool results are bundled into the next user message.
pub fn db_rows_to_rig_history(rows: &[DbMessage]) -> Vec<RigMessage> {
    let mut out: Vec<RigMessage> = Vec::with_capacity(rows.len());
    let mut i = 0;
    while i < rows.len() {
        let row = &rows[i];
        match row.role.as_str() {
            "user" => {
                out.push(RigMessage::user(row.content.clone()));
                i += 1;
            }
            "assistant" | "tool" => {
                // Collect a contiguous "model turn":
                //   - at most one leading assistant text row
                //   - zero or more tool rows immediately following
                // The next assistant text row (if any) belongs to the
                // FOLLOWING model turn (after the tool results land), so
                // we stop there.
                let mut assistant_contents: Vec<AssistantContent> = Vec::new();
                let mut tool_results: Vec<UserContent> = Vec::new();

                if row.role == "assistant" {
                    if !row.content.is_empty() {
                        assistant_contents.push(AssistantContent::text(row.content.clone()));
                    }
                    i += 1;
                }

                while i < rows.len() && rows[i].role == "tool" {
                    let tool_row = &rows[i];
                    let (Some(call_id), Some(tool_name)) =
                        (tool_row.tool_call_id.clone(), tool_row.tool_name.clone())
                    else {
                        i += 1;
                        continue;
                    };
                    let Ok(payload) = serde_json::from_str::<ToolMessageContent>(&tool_row.content)
                    else {
                        i += 1;
                        continue;
                    };
                    assistant_contents.push(AssistantContent::ToolCall(ToolCall::new(
                        call_id.clone(),
                        ToolFunction::new(tool_name, payload.args.clone()),
                    )));
                    tool_results.push(UserContent::ToolResult(ToolResult {
                        id: call_id.clone(),
                        call_id: Some(call_id),
                        content: OneOrMany::one(ToolResultContent::text(result_content_for(
                            &payload,
                        ))),
                    }));
                    i += 1;
                }

                if let Ok(content) = OneOrMany::many(assistant_contents) {
                    out.push(RigMessage::Assistant { id: None, content });
                }
                if let Ok(content) = OneOrMany::many(tool_results) {
                    out.push(RigMessage::User { content });
                }
            }
            _ => i += 1, // skip system / unknown
        }
    }
    out
}

/// Render the tool result body the way we want the model to see it on
/// replay. For successful calls we serialise the actual output JSON; for
/// errored / rejected calls we surface the summary; for still-pending
/// calls (rare in replay — only happens if the user is mid-confirmation)
/// we use the original awaiting marker.
fn result_content_for(payload: &ToolMessageContent) -> String {
    if let Some(out) = &payload.output {
        return serde_json::to_string(out).unwrap_or_else(|_| out.to_string());
    }
    match payload.status.as_str() {
        "success" => payload
            .summary
            .clone()
            .unwrap_or_else(|| "ok".to_string()),
        "error" => {
            let reason = payload.summary.as_deref().unwrap_or("failed");
            format!("{{\"error\":\"{}\"}}", reason.replace('"', "\\\""))
        }
        // 'running' would only land here if the cancellation sweep missed
        // it (process crash mid-stream). Treat as cancelled so the model
        // doesn't loop on a phantom awaiting marker.
        "running" => r#"{"error":"cancelled"}"#.to_string(),
        // 'pending_user' really means the user hasn't decided yet — but
        // by the time we replay history, that state should be terminal.
        _ => r#"{"status":"awaiting_user_input"}"#.to_string(),
    }
}
