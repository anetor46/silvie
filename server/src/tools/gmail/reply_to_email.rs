use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::Deserialize;
use tracing::{debug, info, instrument};

use super::common::{build_raw_message, encode_base64url, ApiSendResponse, SentMessage, GMAIL_BASE};
use super::error::{make_api_error, GmailError};

const DESCRIPTION: &str = include_str!("../../../prompts/gmail/reply_to_email.md");

pub struct ReplyToEmailTool {
    access_token: String,
    http_client: reqwest::Client,
}

impl ReplyToEmailTool {
    pub fn new(access_token: String) -> Self {
        Self {
            access_token,
            http_client: reqwest::Client::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ReplyToEmailArgs {
    /// The Gmail thread ID to reply into (from `list_emails` or `get_email`).
    pub thread_id: String,
    /// One or more recipient email addresses (typically the original sender).
    pub to: Vec<String>,
    /// Optional CC addresses.
    pub cc: Option<Vec<String>>,
    /// Subject line — defaults to the original subject if omitted, prefixed
    /// with "Re: " if not already present.
    pub subject: String,
    pub body: String,
    /// The `Message-ID` header value from the email you are replying to
    /// (obtained via `get_email`). Pass this so email clients thread correctly.
    pub message_id_header: Option<String>,
}

impl Tool for ReplyToEmailTool {
    const NAME: &'static str = "reply_to_email";

    type Error = GmailError;
    type Args = ReplyToEmailArgs;
    type Output = SentMessage;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: DESCRIPTION.trim().to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "required": ["thread_id", "to", "subject", "body"],
                "properties": {
                    "thread_id": {
                        "type": "string",
                        "description": "The Gmail thread ID to reply into."
                    },
                    "to": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Recipient email addresses (typically the original sender)."
                    },
                    "cc": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "CC addresses (optional)."
                    },
                    "subject": {
                        "type": "string",
                        "description": "Subject line. Prefix with 'Re: ' if replying \
                            to keep threading visible."
                    },
                    "body": {
                        "type": "string",
                        "description": "Plain-text reply body."
                    },
                    "message_id_header": {
                        "type": "string",
                        "description": "The Message-ID header value from the original email \
                            (from `get_email` → message_id_header). Used for proper \
                            In-Reply-To threading in email clients. Optional but recommended."
                    }
                }
            }),
        }
    }

    #[instrument(skip(self))]
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let cc = args.cc.unwrap_or_default();
        debug!(
            thread_id = %args.thread_id,
            to = ?args.to,
            subject = %args.subject,
            body_len = args.body.len(),
            has_in_reply_to = args.message_id_header.is_some(),
            "sending reply"
        );

        let in_reply_to = args.message_id_header.as_deref();
        let raw =
            build_raw_message(&args.to, &cc, &args.subject, &args.body, in_reply_to, in_reply_to);
        let raw_b64 = encode_base64url(raw.as_bytes());

        let resp = self
            .http_client
            .post(format!("{GMAIL_BASE}/messages/send"))
            .bearer_auth(&self.access_token)
            .json(&serde_json::json!({
                "raw": raw_b64,
                "threadId": args.thread_id,
            }))
            .send()
            .await?;

        let status = resp.status();
        let body = resp.text().await?;
        if !status.is_success() {
            return Err(make_api_error(status, body));
        }

        let sent: ApiSendResponse = serde_json::from_str(&body)
            .map_err(|e| GmailError::Parse(format!("{e}: {body}")))?;

        info!(id = %sent.id, thread_id = %sent.thread_id, "reply sent");
        Ok(SentMessage {
            id: sent.id,
            thread_id: sent.thread_id,
        })
    }
}
