use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::Deserialize;
use tracing::{debug, info, instrument};

use super::common::{build_raw_message, encode_base64url, ApiSendResponse, SentMessage, GMAIL_BASE};
use super::error::{make_api_error, GmailError};

const DESCRIPTION: &str = include_str!("../../../prompts/gmail/send_email.md");

pub struct SendEmailTool {
    access_token: String,
    http_client: reqwest::Client,
}

impl SendEmailTool {
    pub fn new(access_token: String) -> Self {
        Self {
            access_token,
            http_client: reqwest::Client::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SendEmailArgs {
    /// One or more recipient email addresses.
    pub to: Vec<String>,
    /// Optional CC addresses.
    pub cc: Option<Vec<String>>,
    pub subject: String,
    pub body: String,
}

impl Tool for SendEmailTool {
    const NAME: &'static str = "send_email";

    type Error = GmailError;
    type Args = SendEmailArgs;
    type Output = SentMessage;

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
                        "description": "Recipient email addresses."
                    },
                    "cc": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "CC addresses (optional)."
                    },
                    "subject": {
                        "type": "string",
                        "description": "Email subject line."
                    },
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
        let cc = args.cc.unwrap_or_default();
        debug!(
            to = ?args.to,
            subject = %args.subject,
            body_len = args.body.len(),
            "sending email"
        );

        let raw = build_raw_message(&args.to, &cc, &args.subject, &args.body, None, None);
        let raw_b64 = encode_base64url(raw.as_bytes());

        let resp = self
            .http_client
            .post(format!("{GMAIL_BASE}/messages/send"))
            .bearer_auth(&self.access_token)
            .json(&serde_json::json!({ "raw": raw_b64 }))
            .send()
            .await?;

        let status = resp.status();
        let body = resp.text().await?;
        if !status.is_success() {
            return Err(make_api_error(status, body));
        }

        let sent: ApiSendResponse = serde_json::from_str(&body)
            .map_err(|e| GmailError::Parse(format!("{e}: {body}")))?;

        info!(id = %sent.id, "email sent");
        Ok(SentMessage {
            id: sent.id,
            thread_id: sent.thread_id,
        })
    }
}
