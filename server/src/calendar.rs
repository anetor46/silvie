use chrono::{Duration, Utc};
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, instrument};

#[derive(Debug, thiserror::Error)]
pub enum CalendarError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Calendar API error (HTTP {status}): {body}")]
    ApiError { status: u16, body: String },
    #[error("Failed to parse Calendar API response: {0}")]
    Parse(String),
}

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

#[derive(Debug, Deserialize)]
pub struct CalendarArgs {
    /// Start of the time range (ISO 8601). Defaults to now.
    pub start_time: Option<String>,
    /// End of the time range (ISO 8601). Defaults to 7 days from now.
    pub end_time: Option<String>,
    /// Maximum number of events to return. Defaults to 10.
    pub max_results: Option<u32>,
}

#[derive(Serialize)]
pub struct CalendarOutput {
    pub events: Vec<CalendarEvent>,
}

#[derive(Serialize)]
pub struct CalendarEvent {
    pub summary: String,
    pub start: String,
    pub end: String,
    pub location: Option<String>,
    pub description: Option<String>,
}

// Internal types for deserializing the Google Calendar API response.
#[derive(Deserialize)]
struct ApiResponse {
    items: Option<Vec<ApiEvent>>,
}

#[derive(Deserialize)]
struct ApiEvent {
    summary: Option<String>,
    start: ApiEventTime,
    end: ApiEventTime,
    location: Option<String>,
    description: Option<String>,
}

#[derive(Deserialize)]
struct ApiEventTime {
    #[serde(rename = "dateTime")]
    date_time: Option<String>,
    date: Option<String>,
}

impl ApiEventTime {
    fn as_str(&self) -> String {
        self.date_time
            .clone()
            .or_else(|| self.date.clone())
            .unwrap_or_else(|| "unknown".to_string())
    }
}

const TOOL_DESCRIPTION: &str = include_str!("../prompts/calendar_tool_description.md");

impl Tool for GoogleCalendarTool {
    const NAME: &'static str = "get_calendar_events";

    type Error = CalendarError;
    type Args = CalendarArgs;
    type Output = CalendarOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: TOOL_DESCRIPTION.trim().to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "start_time": {
                        "type": "string",
                        "description": "Start of the time range in ISO 8601 format \
                            (e.g. 2024-06-19T00:00:00Z). Defaults to now if omitted."
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
                    }
                }
            }),
        }
    }

    #[instrument(skip(self))]
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let now = Utc::now();
        let time_min = args
            .start_time
            .unwrap_or_else(|| now.to_rfc3339());
        let time_max = args
            .end_time
            .unwrap_or_else(|| (now + Duration::days(7)).to_rfc3339());
        let max_results = args.max_results.unwrap_or(10).min(50);

        debug!(
            access_token_len = self.access_token.len(),
            time_min,
            time_max,
            max_results,
            "fetching calendar events"
        );

        let response = self
            .http_client
            .get("https://www.googleapis.com/calendar/v3/calendars/primary/events")
            .bearer_auth(&self.access_token)
            .query(&[
                ("timeMin", time_min.as_str()),
                ("timeMax", time_max.as_str()),
                ("maxResults", &max_results.to_string()),
                ("singleEvents", "true"),
                ("orderBy", "startTime"),
            ])
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;
        debug!("calendar API response status: {status}");

        if !status.is_success() {
            error!("calendar API error ({status}): {body}");
            return Err(CalendarError::ApiError {
                status: status.as_u16(),
                body,
            });
        }

        let api_resp: ApiResponse = serde_json::from_str(&body)
            .map_err(|e| CalendarError::Parse(format!("{e}: {body}")))?;

        let events = api_resp
            .items
            .unwrap_or_default()
            .into_iter()
            .map(|ev| CalendarEvent {
                summary: ev.summary.unwrap_or_else(|| "(no title)".to_string()),
                start: ev.start.as_str(),
                end: ev.end.as_str(),
                location: ev.location,
                description: ev.description,
            })
            .collect();

        Ok(CalendarOutput { events })
    }
}
