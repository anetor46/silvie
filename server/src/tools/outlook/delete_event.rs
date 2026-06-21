use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use tracing::instrument;

use super::common::GRAPH_BASE;
use super::error::{make_api_error, OutlookError};

const DESCRIPTION: &str = include_str!("../../../prompts/outlook/delete_event.md");

pub struct DeleteOutlookEventTool {
    access_token: String,
    http_client: reqwest::Client,
}

impl DeleteOutlookEventTool {
    pub fn new(access_token: String) -> Self {
        Self {
            access_token,
            http_client: reqwest::Client::new(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DeleteOutlookEventArgs {
    pub event_id: String,
}

#[derive(Serialize)]
pub struct DeleteOutlookEventOutput {
    pub deleted: bool,
}

impl Tool for DeleteOutlookEventTool {
    const NAME: &'static str = "delete_outlook_event";

    type Error = OutlookError;
    type Args = DeleteOutlookEventArgs;
    type Output = DeleteOutlookEventOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: DESCRIPTION.trim().to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "required": ["event_id"],
                "properties": {
                    "event_id": {
                        "type": "string",
                        "description": "The Outlook event ID to delete."
                    }
                }
            }),
        }
    }

    #[instrument(skip(self))]
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let resp = self
            .http_client
            .delete(format!("{GRAPH_BASE}/events/{}", args.event_id))
            .bearer_auth(&self.access_token)
            .send()
            .await?;

        let status = resp.status();
        // 204 No Content is the success response for DELETE.
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(make_api_error(status, body));
        }

        Ok(DeleteOutlookEventOutput { deleted: true })
    }
}
