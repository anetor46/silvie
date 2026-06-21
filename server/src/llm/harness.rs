//! Generic tool wrapper that handles two cross-cutting concerns:
//!
//! 1. **Visualization.** Every wrapped tool emits a `ToolEvent::Call` on the
//!    side channel before executing and a `ToolEvent::Result` after — these
//!    flow through the chat stream into the frontend as status cards.
//! 2. **Confirmation gating.** Tools marked as `Write` block on a
//!    `ConfirmationRegistry` entry before executing. The frontend presents
//!    Approve / Reject buttons; the `/chat/confirmations` endpoint resolves
//!    the registry entry which unblocks the call.
//!
//! Wrap a tool: `ToolWrapper::new_read(inner, tx)` for read tools or
//! `ToolWrapper::new_write(inner, tx, registry)` for write tools. The
//! wrapper is itself a `rig::Tool` with the same NAME / Args / Output as
//! its inner — so it is a drop-in replacement at the rig registration site.

use std::sync::Arc;
use std::time::Duration;

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::Serialize;
use tracing::{info, warn};
use uuid::Uuid;

use super::confirmation::{ConfirmationRegistry, Decision};
use crate::types::ToolEvent;

/// Wait at most this long for the user to approve / reject a write tool
/// before timing out and erroring back to the model. Kept generous since
/// the user may step away briefly while composing.
const CONFIRMATION_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Debug, thiserror::Error)]
pub enum WrapperError<E> {
    #[error("user rejected the action{}", .reason.as_ref().map(|r| format!(": {r}")).unwrap_or_default())]
    Rejected { reason: Option<String> },
    #[error("user did not respond in time")]
    Timeout,
    #[error("confirmation cancelled")]
    Cancelled,
    #[error(transparent)]
    Inner(E),
}

pub struct ToolWrapper<T: Tool> {
    inner: T,
    tx: tokio::sync::mpsc::UnboundedSender<ToolEvent>,
    /// `Some` for write tools, `None` for read tools.
    confirmation: Option<Arc<ConfirmationRegistry>>,
}

impl<T: Tool> ToolWrapper<T> {
    pub fn new_read(inner: T, tx: tokio::sync::mpsc::UnboundedSender<ToolEvent>) -> Self {
        Self {
            inner,
            tx,
            confirmation: None,
        }
    }

    pub fn new_write(
        inner: T,
        tx: tokio::sync::mpsc::UnboundedSender<ToolEvent>,
        registry: Arc<ConfirmationRegistry>,
    ) -> Self {
        Self {
            inner,
            tx,
            confirmation: Some(registry),
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
    type Output = T::Output;

    async fn definition(&self, prompt: String) -> ToolDefinition {
        self.inner.definition(prompt).await
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let call_id = Uuid::new_v4().to_string();
        let args_json = serde_json::to_value(&args).unwrap_or(serde_json::Value::Null);
        let requires_confirmation = self.confirmation.is_some();

        // 1. Announce the call to the frontend.
        let _ = self.tx.send(ToolEvent::Call {
            call_id: call_id.clone(),
            name: T::NAME.to_string(),
            args: args_json,
            requires_confirmation,
        });

        // 2. If write, block on user decision.
        if let Some(registry) = &self.confirmation {
            let rx = registry.register(call_id.clone());
            let decision = match tokio::time::timeout(CONFIRMATION_TIMEOUT, rx).await {
                Ok(Ok(d)) => d,
                Ok(Err(_)) => {
                    warn!(call_id, "confirmation sender dropped before responding");
                    let _ = self.tx.send(ToolEvent::Result {
                        call_id: call_id.clone(),
                        success: false,
                        summary: Some("confirmation cancelled".into()),
                    });
                    return Err(WrapperError::Cancelled);
                }
                Err(_) => {
                    warn!(call_id, "confirmation timed out");
                    registry.drop_entry(&call_id);
                    let _ = self.tx.send(ToolEvent::Result {
                        call_id: call_id.clone(),
                        success: false,
                        summary: Some("user did not respond in time".into()),
                    });
                    return Err(WrapperError::Timeout);
                }
            };

            if let Decision::Rejected { reason } = decision {
                info!(call_id, ?reason, "user rejected tool call");
                let _ = self.tx.send(ToolEvent::Result {
                    call_id: call_id.clone(),
                    success: false,
                    summary: Some(
                        reason
                            .clone()
                            .map(|r| format!("rejected: {r}"))
                            .unwrap_or_else(|| "rejected by user".into()),
                    ),
                });
                return Err(WrapperError::Rejected { reason });
            }

            info!(call_id, "user approved tool call");
        }

        // 3. Execute and emit result.
        match self.inner.call(args).await {
            Ok(out) => {
                let _ = self.tx.send(ToolEvent::Result {
                    call_id,
                    success: true,
                    summary: None,
                });
                Ok(out)
            }
            Err(e) => {
                let msg = e.to_string();
                let _ = self.tx.send(ToolEvent::Result {
                    call_id,
                    success: false,
                    summary: Some(msg),
                });
                Err(WrapperError::Inner(e))
            }
        }
    }
}
