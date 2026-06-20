//! Input/output shapes for `LlmClient::stream`. Replaces the previous
//! 7-positional `stream_chat(messages, google_access_token, timezone, …,
//! db_pool)` signature with a typed builder-friendly struct.

use crate::types::ChatMessage;

/// Everything a single chat turn needs from the caller.
#[derive(Debug, Default)]
pub struct ChatTurn {
    /// Full message history including the new user turn at the end.
    pub messages: Vec<ChatMessage>,
    /// Per-request locale context surfaced to the model in the system prompt.
    pub locale: LocaleContext,
    /// Auth credentials for tools the agent can call on the user's behalf.
    /// Each is independent — Google can be present without Stripe and vice versa.
    pub tool_auth: ToolAuth,
}

#[derive(Debug, Default, Clone)]
pub struct LocaleContext {
    pub timezone: Option<String>,
    pub current_datetime: Option<String>,
}

/// Per-user credentials the LLM tools need at call time. The persistent
/// configuration (Stripe key, Travelport credentials, DB pool, …) lives on
/// `LlmClient` itself — these are the *user-specific* bits.
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
