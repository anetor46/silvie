//! Generic tool wrapper handling two cross-cutting concerns:
//!
//! 1. **Visualization.** Every wrapped tool emits a `ToolEvent::Call` to the
//!    side channel before executing, and (for read tools) a
//!    `ToolEvent::Result` after.
//! 2. **Confirmation gating — non-blocking.** Write tools DON'T execute their
//!    inner logic during the agent stream. They persist the pending call
//!    args, emit the `Call` event (with `requires_confirmation: true`), and
//!    immediately return a sentinel `Awaiting` output that the preamble
//!    teaches the model to recognise — the agent turn ends naturally with a
//!    short ack from the model.
//!
//! Actual execution happens later in `tool_dispatch::execute_pending` when
//! the user clicks Approve in the `/chat/tool-responses` endpoint.

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::Serialize;
use uuid::Uuid;

use crate::types::ToolEvent;

#[derive(Debug, thiserror::Error)]
pub enum WrapperError<E> {
    #[error(transparent)]
    Inner(E),
}

/// What the model sees as the tool's output. With `#[serde(untagged)]` this
/// serialises transparently — `Done(t)` looks just like `t`, and
/// `Awaiting(...)` looks like the awaiting marker JSON.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum WrappedOutput<T> {
    Done(T),
    Awaiting(AwaitingMarker),
}

#[derive(Debug, Serialize)]
pub struct AwaitingMarker {
    pub status: &'static str,
    pub message: &'static str,
}

const AWAITING: AwaitingMarker = AwaitingMarker {
    status: "awaiting_user_input",
    message:
        "The user must approve or reject this action via the UI before it runs. \
         Respond with a brief one-line acknowledgement (e.g. \"Waiting for your confirmation.\") \
         and DO NOT call any further tools in this turn.",
};

pub struct ToolWrapper<T: Tool> {
    inner: T,
    tx: tokio::sync::mpsc::UnboundedSender<ToolEvent>,
    requires_confirmation: bool,
}

impl<T: Tool> ToolWrapper<T> {
    pub fn new_read(inner: T, tx: tokio::sync::mpsc::UnboundedSender<ToolEvent>) -> Self {
        Self {
            inner,
            tx,
            requires_confirmation: false,
        }
    }

    pub fn new_write(inner: T, tx: tokio::sync::mpsc::UnboundedSender<ToolEvent>) -> Self {
        Self {
            inner,
            tx,
            requires_confirmation: true,
        }
    }
}

impl<T> Tool for ToolWrapper<T>
where
    T: Tool + Send + Sync + 'static,
    T::Args: Serialize,
{
    const NAME: &'static str = T::NAME;
    type Error = WrapperError<T::Error>;
    type Args = T::Args;
    type Output = WrappedOutput<T::Output>;

    async fn definition(&self, prompt: String) -> ToolDefinition {
        self.inner.definition(prompt).await
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let call_id = Uuid::new_v4().to_string();
        let args_json = serde_json::to_value(&args).unwrap_or(serde_json::Value::Null);

        // Always announce the call to the frontend.
        let _ = self.tx.send(ToolEvent::Call {
            call_id: call_id.clone(),
            name: T::NAME.to_string(),
            args: args_json,
            requires_confirmation: self.requires_confirmation,
        });

        // Write tool: defer execution. The `/chat/tool-responses` endpoint
        // will run the actual work after the user clicks Approve.
        if self.requires_confirmation {
            return Ok(WrappedOutput::Awaiting(AWAITING));
        }

        // Read tool: run inline and emit the result event.
        match self.inner.call(args).await {
            Ok(out) => {
                let output_json = serde_json::to_value(&out).ok();
                let _ = self.tx.send(ToolEvent::Result {
                    call_id,
                    success: true,
                    summary: None,
                    output: output_json,
                });
                Ok(WrappedOutput::Done(out))
            }
            Err(e) => {
                let msg = e.to_string();
                let _ = self.tx.send(ToolEvent::Result {
                    call_id,
                    success: false,
                    summary: Some(msg),
                    output: None,
                });
                Err(WrapperError::Inner(e))
            }
        }
    }
}
