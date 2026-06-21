use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
}

/// New chat request shape. The server is authoritative for the user's
/// conversation history and integration tokens — the client just sends the
/// new user turn plus the conversation id and small per-request context
/// (timezone / datetime).
#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub conversation_id: Uuid,
    pub content: String,
    pub timezone: Option<String>,
    pub current_datetime: Option<String>,
}

/// Events the chat handler streams to the frontend over SSE. Tagged with
/// `type` so the SvelteKit consumer can parse with a single JSON.parse.
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SseEvent {
    Token {
        text: String,
    },
    ToolCall {
        call_id: String,
        name: String,
        args: serde_json::Value,
        requires_confirmation: bool,
    },
    ToolResult {
        call_id: String,
        success: bool,
        summary: Option<String>,
    },
    Done,
    #[allow(dead_code)]
    Error {
        message: String,
    },
}

/// What an LLM tool / its confirmation harness emits on the side channel
/// the chat handler is listening on. Distinct from `SseEvent` because the
/// chat handler may transform / batch before forwarding (e.g. flush text
/// buffer before emitting the wire SSE frame).
#[derive(Debug, Clone)]
pub enum ToolEvent {
    Call {
        call_id: String,
        name: String,
        args: serde_json::Value,
        requires_confirmation: bool,
    },
    Result {
        call_id: String,
        success: bool,
        summary: Option<String>,
    },
}

/// Unified event the LLM client emits to chat handler: text + tool lifecycle.
#[derive(Debug, Clone)]
pub enum ChatEvent {
    Text(String),
    ToolCall {
        call_id: String,
        name: String,
        args: serde_json::Value,
        requires_confirmation: bool,
    },
    ToolResult {
        call_id: String,
        success: bool,
        summary: Option<String>,
    },
}

impl From<ToolEvent> for ChatEvent {
    fn from(ev: ToolEvent) -> Self {
        match ev {
            ToolEvent::Call {
                call_id,
                name,
                args,
                requires_confirmation,
            } => ChatEvent::ToolCall {
                call_id,
                name,
                args,
                requires_confirmation,
            },
            ToolEvent::Result {
                call_id,
                success,
                summary,
            } => ChatEvent::ToolResult {
                call_id,
                success,
                summary,
            },
        }
    }
}
