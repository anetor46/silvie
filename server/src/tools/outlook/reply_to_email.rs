use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use tracing::instrument;

use super::common::GRAPH_BASE;
use super::error::{make_api_error, OutlookError};

const DESCRIPTION: &str = include_str!("../../../prompts/outlook/reply_to_email.md");

pub struct ReplyOutlookEmailTool {
    access_token: String,
    http_client: reqwest::Client,
}

impl ReplyOutlookEmailTool {
    pub fn new(access_token: String) -> Self {
        Self {
            access_token,
            http_client: reqwest::Client::new(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ReplyOutlookEmailArgs {
    /// The message ID to reply to (from get_outlook_email).
    pub message_id: String,
    pub body: String,
}

#[derive(Serialize)]
pub struct ReplyOutlookEmailOutput {
    pub sent: bool,
}

impl Tool for ReplyOutlookEmailTool {
    const NAME: &'static str = "reply_outlook_email";

    type Error = OutlookError;
    type Args = ReplyOutlookEmailArgs;
    type Output = ReplyOutlookEmailOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: DESCRIPTION.trim().to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "required": ["message_id", "body"],
                "properties": {
                    "message_id": {
                        "type": "string",
                        "description": "The Outlook message ID to reply to."
                    },
                    "body": {
                        "type": "string",
                        "description": "Plain-text reply body."
                    }
                }
            }),
        }
    }

    #[instrument(skip(self))]
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let payload = serde_json::json!({
            "message": {
                "body": {
                    "contentType": "Text",
                    "content": args.body
                }
            }
        });

        let resp = self
            .http_client
            .post(format!("{GRAPH_BASE}/messages/{}/reply", args.message_id))
            .bearer_auth(&self.access_token)
            .json(&payload)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(make_api_error(status, body));
        }

        Ok(ReplyOutlookEmailOutput { sent: true })
    }
}
