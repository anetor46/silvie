use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};

use super::common::EVENTS_URL;
use super::error::{make_api_error, CalendarError};

const DESCRIPTION: &str = include_str!("../../../prompts/google_calendar/delete_event.md");

pub struct DeleteCalendarEventTool {
    access_token: String,
    http_client: reqwest::Client,
}

impl DeleteCalendarEventTool {
    pub fn new(access_token: String) -> Self {
        Self {
            access_token,
            http_client: reqwest::Client::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct DeleteEventArgs {
    pub event_id: String,
}

#[derive(Serialize)]
pub struct DeleteOutput {
    pub deleted: bool,
    pub event_id: String,
}

impl Tool for DeleteCalendarEventTool {
    const NAME: &'static str = "delete_calendar_event";

    type Error = CalendarError;
    type Args = DeleteEventArgs;
    type Output = DeleteOutput;

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
                        "description": "ID of the event to delete (from get_calendar_events)."
                    }
                }
            }),
        }
    }

    #[instrument(skip(self))]
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        debug!(
            access_token_len = self.access_token.len(),
            event_id = %args.event_id,
            "deleting calendar event"
        );

        let url = format!("{EVENTS_URL}/{}", args.event_id);
        let response = self
            .http_client
            .delete(&url)
            .bearer_auth(&self.access_token)
            .query(&[("sendUpdates", "all")])
            .send()
            .await?;

        let status = response.status();
        debug!("delete event response status: {status}");

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(make_api_error(status, body));
        }

        Ok(DeleteOutput {
            deleted: true,
            event_id: args.event_id,
        })
    }
}
