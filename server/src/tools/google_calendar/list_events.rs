use chrono::{Duration, Utc};
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};

use super::common::{parse_api_event, ApiEvent, CalendarEvent, EVENTS_URL};
use super::error::{make_api_error, CalendarError};

const DESCRIPTION: &str = include_str!("../../../prompts/google_calendar/list_events.md");

pub struct GoogleCalendarTool {
    access_token: String,
    http_client: reqwest::Client,
}

impl GoogleCalendarTool {
    pub fn new(access_token: String) -> Self {
        Self {
            access_token,
            http_client: reqwest::Client::new(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CalendarArgs {
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub max_results: Option<u32>,
    /// Full-text query against event title, description, location, and attendees.
    pub query: Option<String>,
}

#[derive(Serialize)]
pub struct CalendarOutput {
    pub events: Vec<CalendarEvent>,
}

#[derive(Deserialize)]
struct ApiListResponse {
    items: Option<Vec<ApiEvent>>,
}

impl Tool for GoogleCalendarTool {
    const NAME: &'static str = "get_calendar_events";

    type Error = CalendarError;
    type Args = CalendarArgs;
    type Output = CalendarOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: DESCRIPTION.trim().to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "start_time": {
                        "type": "string",
                        "description": "Start of the time range in ISO 8601 format \
                            (e.g. 2026-06-19T00:00:00+02:00). Defaults to now if omitted."
                    },
                    "end_time": {
                        "type": "string",
                        "description": "End of the time range in ISO 8601 format. \
                            Defaults to 7 days from now if omitted."
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum number of events to return (1–50). Defaults to 10.",
                        "minimum": 1,
                        "maximum": 50
                    },
                    "query": {
                        "type": "string",
                        "description": "Optional full-text search across event title, \
                            description, location, and attendees (e.g. 'Sarah' to find \
                            meetings involving Sarah)."
                    }
                }
            }),
        }
    }

    #[instrument(skip(self))]
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let now = Utc::now();
        let time_min = args.start_time.unwrap_or_else(|| now.to_rfc3339());
        let time_max = args
            .end_time
            .unwrap_or_else(|| (now + Duration::days(7)).to_rfc3339());
        let max_results = args.max_results.unwrap_or(10).min(50);

        debug!(
            access_token_len = self.access_token.len(),
            %time_min,
            %time_max,
            max_results,
            query = ?args.query,
            "fetching calendar events"
        );

        let max_results_str = max_results.to_string();
        let mut query_params: Vec<(&str, &str)> = vec![
            ("timeMin", time_min.as_str()),
            ("timeMax", time_max.as_str()),
            ("maxResults", max_results_str.as_str()),
            ("singleEvents", "true"),
            ("orderBy", "startTime"),
        ];
        if let Some(q) = args.query.as_deref() {
            query_params.push(("q", q));
        }

        let response = self
            .http_client
            .get(EVENTS_URL)
            .bearer_auth(&self.access_token)
            .query(&query_params)
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;
        debug!("calendar list response status: {status}");

        if !status.is_success() {
            return Err(make_api_error(status, body));
        }

        let api_resp: ApiListResponse = serde_json::from_str(&body)
            .map_err(|e| CalendarError::Parse(format!("{e}: {body}")))?;

        let events = api_resp
            .items
            .unwrap_or_default()
            .into_iter()
            .map(parse_api_event)
            .collect();

        Ok(CalendarOutput { events })
    }
}
