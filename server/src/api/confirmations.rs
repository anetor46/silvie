//! `POST /chat/confirmations` — the frontend posts an Approve / Reject
//! decision here when the user clicks a confirmation widget. The decision
//! is routed through the `ConfirmationRegistry` to the parked tool call in
//! the still-open `/chat` SSE stream that originated the request.

use std::sync::Arc;

use poem::{handler, web::{Data, Json}};
use serde::Deserialize;
use tracing::{info, warn};

use crate::{
    auth::AuthUser,
    error::{ApiError, ApiResult},
    llm::{ConfirmationRegistry, Decision},
};

#[derive(Debug, Deserialize)]
pub struct ConfirmationRequest {
    pub call_id: String,
    pub approved: bool,
    #[serde(default)]
    pub reason: Option<String>,
}

#[handler]
pub async fn confirmation_handler(
    _auth: AuthUser,
    Data(registry): Data<&Arc<ConfirmationRegistry>>,
    Json(req): Json<ConfirmationRequest>,
) -> ApiResult<&'static str> {
    let decision = if req.approved {
        Decision::Approved
    } else {
        Decision::Rejected {
            reason: req.reason.clone(),
        }
    };

    if registry.resolve(&req.call_id, decision) {
        info!(call_id = %req.call_id, approved = req.approved, "tool confirmation resolved");
        Ok("ok")
    } else {
        warn!(call_id = %req.call_id, "no pending confirmation for call_id");
        Err(ApiError::NotFound)
    }
}
