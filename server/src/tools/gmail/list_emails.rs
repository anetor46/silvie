use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};

use super::common::{
    header_value, ApiListResponse, ApiMessage, EmailSummary, GMAIL_BASE,
};
use super::error::{make_api_error, GmailError};

const DESCRIPTION: &str = include_str!("../../../prompts/gmail/list_emails.md");

pub struct ListEmailsTool {
    access_token: String,
    http_client: reqwest::Client,
}

impl ListEmailsTool {
    pub fn new(access_token: String) -> Self {
        Self {
            access_token,
            http_client: reqwest::Client::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ListEmailsArgs {
    /// Gmail search query (e.g. "is:unread from:boss@company.com").
    pub query: Option<String>,
    pub max_results: Option<u32>,
    /// Whether to include spam and trash. Defaults to false.
    pub include_spam_trash: Option<bool>,
}

#[derive(Serialize)]
pub struct ListEmailsOutput {
    pub emails: Vec<EmailSummary>,
    pub total_returned: usize,
}

impl Tool for ListEmailsTool {
    const NAME: &'static str = "list_emails";

    type Error = GmailError;
    type Args = ListEmailsArgs;
    type Output = ListEmailsOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: DESCRIPTION.trim().to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Gmail search query (e.g. 'is:unread', \
                            'from:alice@example.com subject:invoice'). \
                            Omit for the most recent emails."
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum number of emails to return (1–25). Defaults to 10.",
                        "minimum": 1,
                        "maximum": 25
                    },
                    "include_spam_trash": {
                        "type": "boolean",
                        "description": "Include spam and trash folders. Defaults to false."
                    }
                }
            }),
        }
    }

    #[instrument(skip(self))]
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let max = args.max_results.unwrap_or(10).clamp(1, 25);
        let include_spam = args.include_spam_trash.unwrap_or(false);

        debug!(
            query = ?args.query,
            max,
            include_spam,
            "listing emails"
        );

        // 1. Get message refs from messages.list.
        let max_str = max.to_string();
        let include_str = include_spam.to_string();
        let mut list_params: Vec<(&str, &str)> = vec![
            ("maxResults", &max_str),
            ("includeSpamTrash", &include_str),
        ];
        let q = args.query.clone().unwrap_or_default();
        if !q.is_empty() {
            list_params.push(("q", &q));
        }

        let list_resp = self
            .http_client
            .get(format!("{GMAIL_BASE}/messages"))
            .bearer_auth(&self.access_token)
            .query(&list_params)
            .send()
            .await?;

        let list_status = list_resp.status();
        let list_body = list_resp.text().await?;
        if !list_status.is_success() {
            return Err(make_api_error(list_status, list_body));
        }

        let list: ApiListResponse = serde_json::from_str(&list_body)
            .map_err(|e| GmailError::Parse(format!("{e}: {list_body}")))?;

        let message_refs = list.messages.unwrap_or_default();
        debug!(count = message_refs.len(), "fetching message metadata");

        // 2. Fetch metadata for each message ref.
        let mut emails = Vec::with_capacity(message_refs.len());
        for msg_ref in message_refs {
            let msg_resp = self
                .http_client
                .get(format!("{GMAIL_BASE}/messages/{}", msg_ref.id))
                .bearer_auth(&self.access_token)
                .query(&[("format", "metadata"), ("metadataHeaders", "From,To,Subject,Date")])
                .send()
                .await?;

            let msg_status = msg_resp.status();
            let msg_body = msg_resp.text().await?;
            if !msg_status.is_success() {
                // Log and skip — don't fail the whole list over one bad message.
                tracing::warn!(
                    id = %msg_ref.id,
                    "failed to fetch email metadata ({}): {}",
                    msg_status,
                    msg_body
                );
                continue;
            }

            let msg: ApiMessage = serde_json::from_str(&msg_body)
                .map_err(|e| GmailError::Parse(format!("{e}: {msg_body}")))?;

            let headers = msg
                .payload
                .as_ref()
                .and_then(|p| p.headers.as_deref())
                .unwrap_or(&[]);

            let unread = msg
                .label_ids
                .as_deref()
                .unwrap_or(&[])
                .iter()
                .any(|l| l == "UNREAD");

            emails.push(EmailSummary {
                id: msg.id.unwrap_or(msg_ref.id),
                thread_id: msg.thread_id.unwrap_or(msg_ref.thread_id),
                from: header_value(headers, "From").unwrap_or("").to_string(),
                to: header_value(headers, "To").unwrap_or("").to_string(),
                subject: header_value(headers, "Subject")
                    .unwrap_or("(no subject)")
                    .to_string(),
                date: header_value(headers, "Date").unwrap_or("").to_string(),
                snippet: msg.snippet.unwrap_or_default(),
                unread,
            });
        }

        let total = emails.len();
        Ok(ListEmailsOutput {
            emails,
            total_returned: total,
        })
    }
}
