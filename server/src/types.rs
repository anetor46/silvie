use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
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

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SseEvent {
    Token { text: String },
    Done,
    #[allow(dead_code)]
    Error { message: String },
}
