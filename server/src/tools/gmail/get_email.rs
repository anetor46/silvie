use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::Deserialize;
use tracing::{debug, instrument};

use super::common::{extract_plain_text, header_value, ApiMessage, EmailFull, GMAIL_BASE};
use super::error::{make_api_error, GmailError};

const DESCRIPTION: &str = include_str!("../../../prompts/gmail/get_email.md");
const BODY_TRUNCATE_CHARS: usize = 6_000;

pub struct GetEmailTool {
    access_token: String,
    http_client: reqwest::Client,
}

impl GetEmailTool {
    pub fn new(access_token: String) -> Self {
        Self {
            access_token,
            http_client: reqwest::Client::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct GetEmailArgs {
    /// Gmail message ID (from `list_emails`).
    pub message_id: String,
}

impl Tool for GetEmailTool {
    const NAME: &'static str = "get_email";

    type Error = GmailError;
    type Args = GetEmailArgs;
    type Output = EmailFull;

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
                        "description": "The Gmail message ID to fetch (obtained from list_emails)."
                    }
                }
            }),
        }
    }

    #[instrument(skip(self))]
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        debug!(message_id = %args.message_id, "fetching full email");

        let resp = self
            .http_client
            .get(format!("{GMAIL_BASE}/messages/{}", args.message_id))
            .bearer_auth(&self.access_token)
            .query(&[("format", "full")])
            .send()
            .await?;

        let status = resp.status();
        let body = resp.text().await?;
        if !status.is_success() {
            return Err(make_api_error(status, body));
        }

        let msg: ApiMessage =
            serde_json::from_str(&body).map_err(|e| GmailError::Parse(format!("{e}: {body}")))?;

        let payload = msg.payload.as_ref();
        let headers = payload
            .and_then(|p| p.headers.as_deref())
            .unwrap_or(&[]);

        let raw_body = payload
            .and_then(|p| extract_plain_text(p))
            .unwrap_or_else(|| "(no readable body)".to_string());

        let (body_text, truncated) = if raw_body.len() > BODY_TRUNCATE_CHARS {
            (raw_body[..BODY_TRUNCATE_CHARS].to_string(), true)
        } else {
            (raw_body, false)
        };

        Ok(EmailFull {
            id: msg.id.unwrap_or_default(),
            thread_id: msg.thread_id.unwrap_or_default(),
            from: header_value(headers, "From").unwrap_or("").to_string(),
            to: header_value(headers, "To").unwrap_or("").to_string(),
            cc: header_value(headers, "Cc").map(String::from),
            subject: header_value(headers, "Subject")
                .unwrap_or("(no subject)")
                .to_string(),
            date: header_value(headers, "Date").unwrap_or("").to_string(),
            message_id_header: header_value(headers, "Message-ID").map(String::from),
            body: body_text,
            truncated,
        })
    }
}
