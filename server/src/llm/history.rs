//! Convert DB message rows into rig `Message`s for the agent's context.
//!
//! Three kinds of rows live in `messages`:
//! - `user` / `assistant`: plain text → `Message::user` / `Message::assistant`.
//! - `tool`: a structured ToolMessageContent JSON. Each one expands into TWO
//!   rig messages so the model sees a faithful conversation:
//!     * `Message::Assistant` containing an `AssistantContent::ToolCall` (the
//!       request the model made earlier)
//!     * `Message::tool_result_with_call_id(...)` with the actual outcome
//!       (real output JSON for successes, error description for failures /
//!       rejections, or the awaiting marker if still pending).

use rig::completion::{
    message::{AssistantContent, ToolCall, ToolFunction},
    Message as RigMessage,
};
use rig::OneOrMany;

use crate::repos::conversations::{Message as DbMessage, ToolMessageContent};

/// Convert an ordered slice of DB rows into rig `Message`s. Order is
/// preserved; tool rows expand into a (ToolCall, ToolResult) pair.
pub fn db_rows_to_rig_history(rows: &[DbMessage]) -> Vec<RigMessage> {
    let mut out: Vec<RigMessage> = Vec::with_capacity(rows.len() + rows.len() / 2);
    for row in rows {
        match row.role.as_str() {
            "user" => out.push(RigMessage::user(row.content.clone())),
            "assistant" => out.push(RigMessage::assistant(row.content.clone())),
            "tool" => {
                let Some(call_id) = row.tool_call_id.clone() else {
                    continue;
                };
                let Some(tool_name) = row.tool_name.clone() else {
                    continue;
                };
                let Ok(payload) = serde_json::from_str::<ToolMessageContent>(&row.content) else {
                    continue;
                };
                out.push(RigMessage::Assistant {
                    id: None,
                    content: OneOrMany::one(AssistantContent::ToolCall(ToolCall::new(
                        call_id.clone(),
                        ToolFunction::new(tool_name, payload.args.clone()),
                    ))),
                });
                let result_text = result_content_for(&payload);
                out.push(RigMessage::tool_result_with_call_id(
                    call_id.clone(),
                    Some(call_id),
                    result_text,
                ));
            }
            _ => {} // skip system / unknown
        }
    }
    out
}

/// Render the tool result body the way we want the model to see it on replay.
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
        "pending_user" | _ => {
            r#"{"status":"awaiting_user_input"}"#.to_string()
        }
    }
}
