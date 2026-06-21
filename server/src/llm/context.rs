//! Input shapes for `LlmClient::stream`. `ChatTurn` carries a
//! pre-reconstructed rig message history (built by the chat / tool-response
//! handlers from DB rows) plus the immediate prompt for this turn.

use rig::completion::Message as RigMessage;

/// Everything a single chat turn needs from the caller.
#[derive(Debug, Default)]
pub struct ChatTurn {
    /// Conversation history excluding the immediate prompt — including
    /// past tool calls and results as rig `AssistantContent::ToolCall` /
    /// `UserContent::ToolResult` messages, so the model has full context.
    pub history: Vec<RigMessage>,
    /// The current user input (or a synthesized continuation prompt when
    /// resuming from a tool response).
    pub prompt: String,
    /// Per-request locale context surfaced to the model in the system prompt.
    pub locale: LocaleContext,
    /// Auth credentials for tools the agent can call on the user's behalf.
    pub tool_auth: ToolAuth,
}

#[derive(Debug, Default, Clone)]
pub struct LocaleContext {
    pub timezone: Option<String>,
    pub current_datetime: Option<String>,
}

/// Per-user credentials the LLM tools need at call time.
#[derive(Debug, Default, Clone)]
pub struct ToolAuth {
    pub google_access_token: Option<String>,
    pub stripe_payment: Option<StripePaymentRefs>,
}

#[derive(Debug, Clone)]
pub struct StripePaymentRefs {
    pub customer_id: String,
    pub payment_method_id: String,
}
