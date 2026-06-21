use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use tracing::instrument;

use super::common::{parse_event, ApiEventItem, GRAPH_BASE};
use super::error::{make_api_error, OutlookError};

const DESCRIPTION: &str = include_str!("../../../prompts/outlook/update_event.md");

pub struct UpdateOutlookEventTool {
    access_token: String,
    http_client: reqwest::Client,
}

impl UpdateOutlookEventTool {
    pub fn new(access_token: String) -> Self {
        Self {
            access_token,
            http_client: reqwest::Client::new(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateOutlookEventArgs {
    pub event_id: String,
    pub subject: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub location: Option<String>,
    pub body: Option<String>,
    pub attendees: Option<Vec<String>>,
}

impl Tool for UpdateOutlookEventTool {
    const NAME: &'static str = "update_outlook_event";

    type Error = OutlookError;
    type Args = UpdateOutlookEventArgs;
    type Output = serde_json::Value;

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
                        "description": "The Outlook event ID to update."
                    },
                    "subject": { "type": "string" },
                    "start_time": { "type": "string", "description": "ISO 8601 UTC." },
                    "end_time":   { "type": "string", "description": "ISO 8601 UTC." },
                    "location":   { "type": "string" },
                    "body":       { "type": "string", "description": "Event description / agenda." },
                    "attendees":  {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Replaces the full attendee list."
                    }
                }
            }),
        }
    }

    #[instrument(skip(self))]
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let mut patch = serde_json::Map::new();

        if let Some(s) = args.subject {
            patch.insert("subject".to_string(), serde_json::Value::String(s));
        }
        if let Some(st) = args.start_time {
            patch.insert(
                "start".to_string(),
                serde_json::json!({ "dateTime": st, "timeZone": "UTC" }),
            );
        }
        if let Some(et) = args.end_time {
            patch.insert(
                "end".to_string(),
                serde_json::json!({ "dateTime": et, "timeZone": "UTC" }),
            );
        }
        if let Some(loc) = args.location {
            patch.insert(
                "location".to_string(),
                serde_json::json!({ "displayName": loc }),
            );
        }
        if let Some(b) = args.body {
            patch.insert(
                "body".to_string(),
                serde_json::json!({ "contentType": "Text", "content": b }),
            );
        }
        if let Some(attendees) = args.attendees {
            let list: Vec<serde_json::Value> = attendees
                .iter()
                .map(|a| {
                    serde_json::json!({
                        "emailAddress": { "address": a },
                        "type": "required"
                    })
                })
                .collect();
            patch.insert("attendees".to_string(), serde_json::Value::Array(list));
        }

        let resp = self
            .http_client
            .patch(format!("{GRAPH_BASE}/events/{}", args.event_id))
            .bearer_auth(&self.access_token)
            .json(&patch)
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
