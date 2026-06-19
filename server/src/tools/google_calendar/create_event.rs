use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::Deserialize;
use tracing::{debug, instrument};

use super::common::{
    conference_create_block, datetime_field, parse_api_event, ApiEvent, CalendarEvent, EVENTS_URL,
};
use super::error::{make_api_error, CalendarError};

const DESCRIPTION: &str = include_str!("../../../prompts/google_calendar/create_event.md");

pub struct CreateCalendarEventTool {
    access_token: String,
    http_client: reqwest::Client,
}

impl CreateCalendarEventTool {
    pub fn new(access_token: String) -> Self {
        Self {
            access_token,
            http_client: reqwest::Client::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateEventArgs {
    pub title: String,
    pub start_time: String,
    pub end_time: String,
    pub location: Option<String>,
    pub description: Option<String>,
    pub attendees: Option<Vec<String>>,
    /// If true, generate a Google Meet video link for the event.
    pub add_conference: Option<bool>,
    /// RRULE strings, e.g. ["RRULE:FREQ=WEEKLY;BYDAY=MO;COUNT=10"].
    pub recurrence: Option<Vec<String>>,
}

impl Tool for CreateCalendarEventTool {
    const NAME: &'static str = "create_calendar_event";

    type Error = CalendarError;
    type Args = CreateEventArgs;
    type Output = CalendarEvent;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: DESCRIPTION.trim().to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "required": ["title", "start_time", "end_time"],
                "properties": {
                    "title": {
                        "type": "string",
                        "description": "Title of the event."
                    },
                    "start_time": {
                        "type": "string",
                        "description": "Start time in ISO 8601 with UTC offset \
                            (e.g. 2026-06-20T09:00:00+02:00), or YYYY-MM-DD for all-day."
                    },
                    "end_time": {
                        "type": "string",
                        "description": "End time in ISO 8601 with UTC offset, or YYYY-MM-DD \
                            for all-day (exclusive — pass the day AFTER the all-day event)."
                    },
                    "location": {
                        "type": "string",
                        "description": "Location of the event (optional)."
                    },
                    "description": {
                        "type": "string",
                        "description": "Notes or agenda for the event (optional)."
                    },
                    "attendees": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Attendee email addresses (optional)."
                    },
                    "add_conference": {
                        "type": "boolean",
                        "description": "If true, generate a Google Meet video link \
                            and return it in `meet_link`."
                    },
                    "recurrence": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Recurrence rules as RRULE strings, e.g. \
                            [\"RRULE:FREQ=WEEKLY;BYDAY=MO;COUNT=10\"]."
                    }
                }
            }),
        }
    }

    #[instrument(skip(self))]
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        debug!(
            access_token_len = self.access_token.len(),
            title = %args.title,
            start = %args.start_time,
            end = %args.end_time,
            add_conference = ?args.add_conference,
            "creating calendar event"
        );

        let mut body = serde_json::json!({
            "summary": args.title,
            "start": datetime_field(&args.start_time),
            "end":   datetime_field(&args.end_time),
        });

        if let Some(loc) = args.location {
            body["location"] = serde_json::Value::String(loc);
        }
        if let Some(desc) = args.description {
            body["description"] = serde_json::Value::String(desc);
        }
        if let Some(emails) = args.attendees {
            let att: Vec<serde_json::Value> = emails
                .into_iter()
                .map(|e| serde_json::json!({ "email": e }))
                .collect();
            body["attendees"] = serde_json::Value::Array(att);
        }
        if let Some(rules) = args.recurrence {
            body["recurrence"] = serde_json::Value::Array(
                rules.into_iter().map(serde_json::Value::String).collect(),
            );
        }

        let want_conference = args.add_conference.unwrap_or(false);
        if want_conference {
            body["conferenceData"] = conference_create_block();
        }

        let mut query: Vec<(&str, &str)> = vec![("sendUpdates", "all")];
        if want_conference {
            query.push(("conferenceDataVersion", "1"));
        }

        let response = self
            .http_client
            .post(EVENTS_URL)
            .bearer_auth(&self.access_token)
            .query(&query)
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;
        debug!("create event response status: {status}");

        if !status.is_success() {
            return Err(make_api_error(status, text));
        }

        let ev: ApiEvent = serde_json::from_str(&text)
            .map_err(|e| CalendarError::Parse(format!("{e}: {text}")))?;

        Ok(parse_api_event(ev))
    }
}
