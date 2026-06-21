use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use tracing::instrument;

use super::common::{parse_event, ApiEventItem, GRAPH_BASE};
use super::error::{make_api_error, OutlookError};

const DESCRIPTION: &str = include_str!("../../../prompts/outlook/create_event.md");

pub struct CreateOutlookEventTool {
    access_token: String,
    http_client: reqwest::Client,
}

impl CreateOutlookEventTool {
    pub fn new(access_token: String) -> Self {
        Self {
            access_token,
            http_client: reqwest::Client::new(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateOutlookEventArgs {
    pub subject: String,
    /// ISO 8601 UTC (e.g. "2026-06-22T09:00:00Z").
    pub start_time: String,
    /// ISO 8601 UTC.
    pub end_time: String,
    pub location: Option<String>,
    pub body: Option<String>,
    pub attendees: Option<Vec<String>>,
}

impl Tool for CreateOutlookEventTool {
    const NAME: &'static str = "create_outlook_event";

    type Error = OutlookError;
    type Args = CreateOutlookEventArgs;
    type Output = serde_json::Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: DESCRIPTION.trim().to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "required": ["subject", "start_time", "end_time"],
                "properties": {
                    "subject": { "type": "string" },
                    "start_time": {
                        "type": "string",
                        "description": "Start time in ISO 8601 UTC."
                    },
                    "end_time": {
                        "type": "string",
                        "description": "End time in ISO 8601 UTC."
                    },
                    "location": { "type": "string" },
                    "body": {
                        "type": "string",
                        "description": "Optional event description / agenda."
                    },
                    "attendees": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Attendee email addresses."
                    }
                }
            }),
        }
    }

    #[instrument(skip(self))]
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let attendees: Vec<serde_json::Value> = args
            .attendees
            .unwrap_or_default()
            .iter()
            .map(|a| serde_json::json!({
                "emailAddress": { "address": a },
                "type": "required"
            }))
            .collect();

        let mut payload = serde_json::json!({
            "subject": args.subject,
            "start": { "dateTime": args.start_time, "timeZone": "UTC" },
            "end":   { "dateTime": args.end_time,   "timeZone": "UTC" },
            "attendees": attendees
        });

        if let Some(loc) = args.location {
            payload["location"] = serde_json::json!({ "displayName": loc });
        }
        if let Some(b) = args.body {
            payload["body"] = serde_json::json!({ "contentType": "Text", "content": b });
        }

        let resp = self
            .http_client
            .post(format!("{GRAPH_BASE}/events"))
            .bearer_auth(&self.access_token)
            .json(&payload)
            .send()
            .await?;

        let status = resp.status();
        let body = resp.text().await?;
        if !status.is_success() {
            return Err(make_api_error(status, body));
        }

        let event: ApiEventItem = serde_json::from_str(&body)
            .map_err(|e| OutlookError::Parse(format!("{e}: {body}")))?;

        Ok(serde_json::to_value(parse_event(event)).unwrap_or_default())
    }
}
