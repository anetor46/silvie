use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};

use super::common::{ApiMessageFull, OutlookEmailFull, GRAPH_BASE};
use super::error::{make_api_error, OutlookError};

const DESCRIPTION: &str = include_str!("../../../prompts/outlook/get_email.md");
const MAX_BODY_LEN: usize = 6_000;

pub struct GetOutlookEmailTool {
    access_token: String,
    http_client: reqwest::Client,
}

impl GetOutlookEmailTool {
    pub fn new(access_token: String) -> Self {
        Self {
            access_token,
            http_client: reqwest::Client::new(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GetOutlookEmailArgs {
    /// The Outlook message ID.
    pub message_id: String,
}

impl Tool for GetOutlookEmailTool {
    const NAME: &'static str = "get_outlook_email";

    type Error = OutlookError;
    type Args = GetOutlookEmailArgs;
    type Output = OutlookEmailFull;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: DESCRIPTION.trim().to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "required": ["message_id"],
                "properties": {
                    "message_id": {
                        "type": "string",
                        "description": "The Outlook message ID (from list_outlook_emails)."
                    }
                }
            }),
        }
    }

    #[instrument(skip(self))]
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        debug!(id = %args.message_id, "fetching full Outlook email");

        let select = "id,subject,from,toRecipients,ccRecipients,receivedDateTime,internetMessageId,body";
        let resp = self
            .http_client
            .get(format!("{GRAPH_BASE}/messages/{}", args.message_id))
            .bearer_auth(&self.access_token)
            .query(&[("$select", select)])
            .send()
            .await?;

        let status = resp.status();
        let body = resp.text().await?;
        if !status.is_success() {
            return Err(make_api_error(status, body));
        }

        let msg: ApiMessageFull = serde_json::from_str(&body)
            .map_err(|e| OutlookError::Parse(format!("{e}: {body}")))?;

        let from = msg.from.as_ref().map(|r| r.display()).unwrap_or_default();
        let to = msg
            .to_recipients
            .unwrap_or_default()
            .iter()
            .map(|r| r.display())
            .collect();
        let cc = msg
            .cc_recipients
            .unwrap_or_default()
            .iter()
            .map(|r| r.display())
            .collect();

        let raw_body = msg
            .body
            .and_then(|b| b.content)
            .unwrap_or_default();
        let truncated = raw_body.len() > MAX_BODY_LEN;
        let body_text = if truncated {
            raw_body[..MAX_BODY_LEN].to_string()
        } else {
            raw_body
        };

        Ok(OutlookEmailFull {
            id: msg.id,
            subject: msg.subject.unwrap_or_else(|| "(no subject)".to_string()),
            from,
            to,
            cc,
            received_at: msg.received_date_time.unwrap_or_default(),
            internet_message_id: msg.internet_message_id,
            body: body_text,
            truncated,
        })
    }
}
