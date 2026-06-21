use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};

use super::common::{
    conference_create_block, datetime_field, parse_api_event, ApiAttendee, ApiEvent, CalendarEvent,
    EVENTS_URL,
};
use super::error::{make_api_error, CalendarError};

const DESCRIPTION: &str = include_str!("../../../prompts/google_calendar/update_event.md");

pub struct UpdateCalendarEventTool {
    access_token: String,
    http_client: reqwest::Client,
}

impl UpdateCalendarEventTool {
    pub fn new(access_token: String) -> Self {
        Self {
            access_token,
            http_client: reqwest::Client::new(),
        }
    }

    /// Fetch the current event so we can read its attendees list (needed when
    /// the caller wants to add/remove attendees — PATCH replaces the entire
    /// array, so we have to merge ourselves).
    async fn fetch_current_attendees(
        &self,
        event_id: &str,
    ) -> Result<Vec<ApiAttendee>, CalendarError> {
        let url = format!("{EVENTS_URL}/{event_id}");
        let resp = self
            .http_client
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await?;

        let status = resp.status();
        let body = resp.text().await?;
        debug!("get event (for attendee merge) status: {status}");

        if !status.is_success() {
            return Err(make_api_error(status, body));
        }

        let ev: ApiEvent = serde_json::from_str(&body)
            .map_err(|e| CalendarError::Parse(format!("{e}: {body}")))?;
        Ok(ev.attendees.unwrap_or_default())
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateEventArgs {
    pub event_id: String,
    pub title: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub location: Option<String>,
    pub description: Option<String>,
    /// Replace the entire attendee list with these emails. Use sparingly.
    pub set_attendees: Option<Vec<String>>,
    /// Add these emails to the existing attendee list (idempotent).
    pub add_attendees: Option<Vec<String>>,
    /// Remove these emails from the existing attendee list.
    pub remove_attendees: Option<Vec<String>>,
    /// If true, generate a Google Meet video link for the event.
    pub add_conference: Option<bool>,
}

impl Tool for UpdateCalendarEventTool {
    const NAME: &'static str = "update_calendar_event";

    type Error = CalendarError;
    type Args = UpdateEventArgs;
    type Output = CalendarEvent;

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
                        "description": "ID of the event to update (from get_calendar_events)."
                    },
                    "title": {
                        "type": "string",
                        "description": "New event title."
                    },
                    "start_time": {
                        "type": "string",
                        "description": "New start in ISO 8601 with UTC offset, or YYYY-MM-DD \
                            for all-day."
                    },
                    "end_time": {
                        "type": "string",
                        "description": "New end in ISO 8601 with UTC offset, or YYYY-MM-DD \
                            for all-day (exclusive)."
                    },
                    "location": {
                        "type": "string",
                        "description": "New location."
                    },
                    "description": {
                        "type": "string",
                        "description": "New description or notes."
                    },
                    "set_attendees": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Replace the entire attendee list with these emails. \
                            Prefer add_attendees/remove_attendees when possible."
                    },
                    "add_attendees": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Add these email addresses to the existing attendee \
                            list (existing attendees are preserved)."
                    },
                    "remove_attendees": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Remove these email addresses from the existing \
                            attendee list."
                    },
                    "add_conference": {
                        "type": "boolean",
                        "description": "If true, generate a Google Meet video link and \
                            return it in `meet_link`."
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
            "updating calendar event"
        );

        let needs_attendee_merge = args.set_attendees.is_some()
            || args.add_attendees.is_some()
            || args.remove_attendees.is_some();

        let merged_attendees: Option<Vec<ApiAttendee>> = if needs_attendee_merge {
            let mut list: Vec<ApiAttendee> = if args.set_attendees.is_some() {
                Vec::new()
            } else {
                self.fetch_current_attendees(&args.event_id).await?
            };

            if let Some(set) = args.set_attendees {
                list = set
                    .into_iter()
                    .map(|email| ApiAttendee {
                        email,
                        display_name: None,
                        response_status: None,
                        is_self: false,
                    })
                    .collect();
            } else {
                if let Some(add) = args.add_attendees {
                    for email in add {
                        if !list.iter().any(|a| a.email.eq_ignore_ascii_case(&email)) {
                            list.push(ApiAttendee {
                                email,
                                display_name: None,
                                response_status: None,
                                is_self: false,
                            });
                        }
                    }
                }
                if let Some(remove) = args.remove_attendees {
                    let to_remove: Vec<String> =
                        remove.iter().map(|e| e.to_ascii_lowercase()).collect();
                    list.retain(|a| {
                        !to_remove
                            .iter()
                            .any(|e| e == &a.email.to_ascii_lowercase())
                    });
                }
            }

            Some(list)
        } else {
            None
        };

        let mut patch = serde_json::Map::new();

        if let Some(t) = args.title {
            patch.insert("summary".into(), serde_json::Value::String(t));
        }
        if let Some(start) = args.start_time {
            patch.insert("start".into(), datetime_field(&start));
        }
        if let Some(end) = args.end_time {
            patch.insert("end".into(), datetime_field(&end));
        }
        if let Some(loc) = args.location {
            patch.insert("location".into(), serde_json::Value::String(loc));
        }
        if let Some(desc) = args.description {
            patch.insert("description".into(), serde_json::Value::String(desc));
        }
        if let Some(att) = merged_attendees {
            patch.insert(
                "attendees".into(),
                serde_json::to_value(&att).map_err(|e| CalendarError::Parse(e.to_string()))?,
            );
        }

        let want_conference = args.add_conference.unwrap_or(false);
        if want_conference {
            patch.insert("conferenceData".into(), conference_create_block());
        }

        let mut query: Vec<(&str, &str)> = vec![("sendUpdates", "all")];
        if want_conference {
            query.push(("conferenceDataVersion", "1"));
        }

        let url = format!("{EVENTS_URL}/{}", args.event_id);
        let response = self
            .http_client
            .patch(&url)
            .bearer_auth(&self.access_token)
            .query(&query)
            .json(&serde_json::Value::Object(patch))
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;
        debug!("update event response status: {status}");

        if !status.is_success() {
            return Err(make_api_error(status, text));
        }

        let ev: ApiEvent = serde_json::from_str(&text)
            .map_err(|e| CalendarError::Parse(format!("{e}: {text}")))?;

        Ok(parse_api_event(ev))
    }
}
