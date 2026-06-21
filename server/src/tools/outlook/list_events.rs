use chrono::{Duration, Utc};
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};

use super::common::{parse_event, ApiEventListResponse, OutlookEvent, GRAPH_BASE};
use super::error::{make_api_error, OutlookError};

const DESCRIPTION: &str = include_str!("../../../prompts/outlook/list_events.md");

pub struct ListOutlookEventsTool {
    access_token: String,
    http_client: reqwest::Client,
}

impl ListOutlookEventsTool {
    pub fn new(access_token: String) -> Self {
        Self {
            access_token,
            http_client: reqwest::Client::new(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ListOutlookEventsArgs {
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub max_results: Option<u32>,
}

#[derive(Serialize)]
pub struct ListOutlookEventsOutput {
    pub events: Vec<OutlookEvent>,
}

impl Tool for ListOutlookEventsTool {
    const NAME: &'static str = "get_outlook_calendar_events";

    type Error = OutlookError;
    type Args = ListOutlookEventsArgs;
    type Output = ListOutlookEventsOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: DESCRIPTION.trim().to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "start_time": {
                        "type": "string",
                        "description": "Start of the time range in ISO 8601 UTC \
                            (e.g. 2026-06-22T00:00:00Z). Defaults to now."
                    },
                    "end_time": {
                        "type": "string",
                        "description": "End of the time range in ISO 8601 UTC. \
                            Defaults to 7 days from now."
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum number of events to return (1–50). Defaults to 10.",
                        "minimum": 1,
                        "maximum": 50
                    }
                }
            }),
        }
    }

    #[instrument(skip(self))]
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let now = Utc::now();
        let start = args
            .start_time
            .unwrap_or_else(|| now.format("%Y-%m-%dT%H:%M:%SZ").to_string());
        let end = args.end_time.unwrap_or_else(|| {
            (now + Duration::days(7))
                .format("%Y-%m-%dT%H:%M:%SZ")
                .to_string()
        });
        let max = args.max_results.unwrap_or(10).min(50);
        let max_str = max.to_string();

        debug!(%start, %end, max, "fetching Outlook calendar events");

        let select = "id,subject,start,end,location,organizer,attendees,bodyPreview";
        let resp = self
            .http_client
            .get(format!("{GRAPH_BASE}/calendarView"))
            .bearer_auth(&self.access_token)
            .query(&[
                ("startDateTime", start.as_str()),
                ("endDateTime", end.as_str()),
                ("$top", max_str.as_str()),
                ("$select", select),
            ])
            .send()
            .await?;

        let status = resp.status();
        let body = resp.text().await?;
        if !status.is_success() {
            return Err(make_api_error(status, body));
        }

        let list: ApiEventListResponse = serde_json::from_str(&body)
            .map_err(|e| OutlookError::Parse(format!("{e}: {body}")))?;

        let events = list.value.into_iter().map(parse_event).collect();
        Ok(ListOutlookEventsOutput { events })
    }
}
