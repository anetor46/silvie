use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use tracing::instrument;

use super::common::GRAPH_BASE;
use super::error::{make_api_error, OutlookError};

const DESCRIPTION: &str = include_str!("../../../prompts/outlook/respond_event.md");

pub struct RespondOutlookEventTool {
    access_token: String,
    http_client: reqwest::Client,
}

impl RespondOutlookEventTool {
    pub fn new(access_token: String) -> Self {
        Self {
            access_token,
            http_client: reqwest::Client::new(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RespondOutlookEventArgs {
    pub event_id: String,
    /// One of: "accept", "tentativelyAccept", "decline".
    pub response: String,
    pub comment: Option<String>,
}

#[derive(Serialize)]
pub struct RespondOutlookEventOutput {
    pub responded: bool,
    pub response: String,
}

impl Tool for RespondOutlookEventTool {
    const NAME: &'static str = "respond_outlook_event";

    type Error = OutlookError;
    type Args = RespondOutlookEventArgs;
    type Output = RespondOutlookEventOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: DESCRIPTION.trim().to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "required": ["event_id", "response"],
                "properties": {
                    "event_id": {
                        "type": "string",
                        "description": "The Outlook event ID to respond to."
                    },
                    "response": {
                        "type": "string",
                        "enum": ["accept", "tentativelyAccept", "decline"],
                        "description": "The response to send."
                    },
                    "comment": {
                        "type": "string",
                        "description": "Optional comment included with the response."
                    }
                }
            }),
        }
    }

    #[instrument(skip(self))]
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let action = match args.response.as_str() {
            "accept" => "accept",
            "tentativelyAccept" => "tentativelyAccept",
            "decline" => "decline",
            other => {
                return Err(OutlookError::Parse(format!(
                    "invalid response '{other}': must be accept, tentativelyAccept, or decline"
                )));
            }
        };

        let payload = serde_json::json!({
            "comment": args.comment.unwrap_or_default(),
            "sendResponse": true
        });

        let resp = self
            .http_client
            .post(format!("{GRAPH_BASE}/events/{}/{}", args.event_id, action))
            .bearer_auth(&self.access_token)
            .json(&payload)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(make_api_error(status, body));
        }

        Ok(RespondOutlookEventOutput {
            responded: true,
            response: action.to_string(),
        })
    }
}
