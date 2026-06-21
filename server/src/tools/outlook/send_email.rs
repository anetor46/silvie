use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use tracing::instrument;

use super::common::GRAPH_BASE;
use super::error::{make_api_error, OutlookError};

const DESCRIPTION: &str = include_str!("../../../prompts/outlook/send_email.md");

pub struct SendOutlookEmailTool {
    access_token: String,
    http_client: reqwest::Client,
}

impl SendOutlookEmailTool {
    pub fn new(access_token: String) -> Self {
        Self {
            access_token,
            http_client: reqwest::Client::new(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SendOutlookEmailArgs {
    pub to: Vec<String>,
    pub cc: Option<Vec<String>>,
    pub subject: String,
    pub body: String,
}

#[derive(Serialize)]
pub struct SendOutlookEmailOutput {
    pub sent: bool,
}

impl Tool for SendOutlookEmailTool {
    const NAME: &'static str = "send_outlook_email";

    type Error = OutlookError;
    type Args = SendOutlookEmailArgs;
    type Output = SendOutlookEmailOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: DESCRIPTION.trim().to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "required": ["to", "subject", "body"],
                "properties": {
                    "to": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Recipient email address(es)."
                    },
                    "cc": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "CC recipient email address(es)."
                    },
                    "subject": { "type": "string" },
                    "body": {
                        "type": "string",
                        "description": "Plain-text email body."
                    }
                }
            }),
        }
    }

    #[instrument(skip(self))]
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let to_recipients: Vec<serde_json::Value> = args
            .to
            .iter()
            .map(|a| serde_json::json!({ "emailAddress": { "address": a } }))
            .collect();

        let cc_recipients: Vec<serde_json::Value> = args
            .cc
            .unwrap_or_default()
            .iter()
            .map(|a| serde_json::json!({ "emailAddress": { "address": a } }))
            .collect();

        let payload = serde_json::json!({
            "message": {
                "subject": args.subject,
                "body": {
                    "contentType": "Text",
                    "content": args.body
                },
                "toRecipients": to_recipients,
                "ccRecipients": cc_recipients
            },
            "saveToSentItems": true
        });

        let resp = self
            .http_client
            .post(format!("{GRAPH_BASE}/sendMail"))
            .bearer_auth(&self.access_token)
            .json(&payload)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(make_api_error(status, body));
        }

        Ok(SendOutlookEmailOutput { sent: true })
    }
}
