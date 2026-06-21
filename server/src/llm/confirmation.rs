//! Per-process registry tracking pending tool confirmations.
//!
//! When a write tool runs through `ToolWrapper`, it generates a `call_id`,
//! registers a `oneshot::Sender<Decision>` here, and parks on the receiver.
//! When the user clicks Approve / Reject in the chat UI, the `/chat/confirmations`
//! endpoint looks up the call_id and `resolve()`s it — waking the parked
//! tool call, which then proceeds (or fails) accordingly.
//!
//! The registry is shared between the chat stream task (registering) and the
//! confirmations HTTP handler (resolving). It is in-memory only; if the
//! process restarts mid-confirmation, the parked tool call dies with the
//! agent stream, and the frontend will get an "expired" response when it
//! tries to post the decision.

use std::collections::HashMap;
use std::sync::Mutex;

use tokio::sync::oneshot;

#[derive(Debug)]
pub enum Decision {
    Approved,
    Rejected { reason: Option<String> },
}

#[derive(Default)]
pub struct ConfirmationRegistry {
    inner: Mutex<HashMap<String, oneshot::Sender<Decision>>>,
}

impl ConfirmationRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a pending confirmation for `call_id` and return the receiver
    /// the caller awaits on. Overwrites any prior entry with the same id
    /// (drops the previous sender, which makes its awaiter resolve to
    /// `Err(RecvError)`).
    pub fn register(&self, call_id: String) -> oneshot::Receiver<Decision> {
        let (tx, rx) = oneshot::channel();
        let mut guard = self.inner.lock().expect("ConfirmationRegistry mutex poisoned");
        guard.insert(call_id, tx);
        rx
    }

    /// Resolve a pending confirmation. Returns `true` if a matching call_id
    /// was found and the decision delivered, `false` if no such pending
    /// confirmation existed (e.g. already resolved, timed out, or never
    /// registered).
    pub fn resolve(&self, call_id: &str, decision: Decision) -> bool {
        let mut guard = self.inner.lock().expect("ConfirmationRegistry mutex poisoned");
        let Some(tx) = guard.remove(call_id) else {
            return false;
        };
        // `send` returns Err if the receiver was dropped — i.e. the tool
        // call already gave up (timeout, stream cancel). Treat that as
        // "no pending confirmation" from the API caller's POV.
        tx.send(decision).is_ok()
    }

    /// Remove an entry (typically used by the awaiter after timeout so the
    /// API caller gets a clean "not found" response).
    pub fn drop_entry(&self, call_id: &str) {
        let mut guard = self.inner.lock().expect("ConfirmationRegistry mutex poisoned");
        guard.remove(call_id);
    }
}
