use chrono::{Duration, Utc};
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, instrument};

const BASE_URL: &str =
    "https://www.googleapis.com/calendar/v3/calendars/primary/events";

// ── Error ────────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum CalendarError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Calendar API error (HTTP {status}): {body}")]
    ApiError { status: u16, body: String },
    #[error("Failed to parse Calendar API response: {0}")]
    Parse(String),
}

fn make_api_error(status: reqwest::StatusCode, body: String) -> CalendarError {
    error!("calendar API error ({status}): {body}");
    CalendarError::ApiError {
        status: status.as_u16(),
        body,
    }
}

// ── Shared output types ───────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct EventAttendee {
    pub email: String,
    pub display_name: Option<String>,
    pub response_status: String,
    pub is_self: bool,
}

#[derive(Debug, Serialize)]
pub struct CalendarEvent {
    pub id: String,
    pub summary: String,
    pub start: String,
    pub end: String,
    pub location: Option<String>,
    pub description: Option<String>,
    pub attendees: Vec<EventAttendee>,
}

// ── Internal API types ────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct ApiListResponse {
    items: Option<Vec<ApiEvent>>,
}

#[derive(Deserialize)]
struct ApiEvent {
    id: Option<String>,
    summary: Option<String>,
    start: ApiEventTime,
    end: ApiEventTime,
    location: Option<String>,
    description: Option<String>,
    attendees: Option<Vec<ApiAttendee>>,
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

#[derive(Deserialize, Serialize, Clone)]
struct ApiAttendee {
    email: String,
    #[serde(rename = "displayName", skip_serializing_if = "Option::is_none")]
    display_name: Option<String>,
    #[serde(rename = "responseStatus", skip_serializing_if = "Option::is_none")]
    response_status: Option<String>,
    #[serde(rename = "self", default)]
    is_self: bool,
}

fn parse_api_event(ev: ApiEvent) -> CalendarEvent {
    CalendarEvent {
        id: ev.id.unwrap_or_else(|| "unknown".to_string()),
        summary: ev.summary.unwrap_or_else(|| "(no title)".to_string()),
        start: ev.start.as_str(),
        end: ev.end.as_str(),
        location: ev.location,
        description: ev.description,
        attendees: ev
            .attendees
            .unwrap_or_default()
            .into_iter()
            .map(|a| EventAttendee {
                email: a.email,
                display_name: a.display_name,
                response_status: a
                    .response_status
                    .unwrap_or_else(|| "needsAction".to_string()),
                is_self: a.is_self,
            })
            .collect(),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Tool 1 — get_calendar_events (read)
// ═══════════════════════════════════════════════════════════════════════════

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
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub max_results: Option<u32>,
}

#[derive(Serialize)]
pub struct CalendarOutput {
    pub events: Vec<CalendarEvent>,
}

const LIST_DESCRIPTION: &str = include_str!("../prompts/calendar_tool_description.md");

impl Tool for GoogleCalendarTool {
    const NAME: &'static str = "get_calendar_events";

    type Error = CalendarError;
    type Args = CalendarArgs;
    type Output = CalendarOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: LIST_DESCRIPTION.trim().to_string(),
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
            "fetching calendar events"
        );

        let response = self
            .http_client
            .get(BASE_URL)
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

// ═══════════════════════════════════════════════════════════════════════════
// Tool 2 — create_calendar_event
// ═══════════════════════════════════════════════════════════════════════════

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
}

const CREATE_DESCRIPTION: &str = include_str!("../prompts/calendar_create_description.md");

impl Tool for CreateCalendarEventTool {
    const NAME: &'static str = "create_calendar_event";

    type Error = CalendarError;
    type Args = CreateEventArgs;
    type Output = CalendarEvent;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: CREATE_DESCRIPTION.trim().to_string(),
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
                            (e.g. 2026-06-20T09:00:00+02:00)."
                    },
                    "end_time": {
                        "type": "string",
                        "description": "End time in ISO 8601 with UTC offset."
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
            "creating calendar event"
        );

        let mut body = serde_json::json!({
            "summary": args.title,
            "start": { "dateTime": args.start_time },
            "end":   { "dateTime": args.end_time },
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

        let response = self
            .http_client
            .post(BASE_URL)
            .bearer_auth(&self.access_token)
            .query(&[("sendUpdates", "all")])
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

// ═══════════════════════════════════════════════════════════════════════════
// Tool 3 — update_calendar_event
// ═══════════════════════════════════════════════════════════════════════════

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
}

#[derive(Debug, Deserialize)]
pub struct UpdateEventArgs {
    pub event_id: String,
    pub title: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub location: Option<String>,
    pub description: Option<String>,
}

const UPDATE_DESCRIPTION: &str = include_str!("../prompts/calendar_update_description.md");

impl Tool for UpdateCalendarEventTool {
    const NAME: &'static str = "update_calendar_event";

    type Error = CalendarError;
    type Args = UpdateEventArgs;
    type Output = CalendarEvent;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: UPDATE_DESCRIPTION.trim().to_string(),
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
                        "description": "New start time in ISO 8601 with UTC offset."
                    },
                    "end_time": {
                        "type": "string",
                        "description": "New end time in ISO 8601 with UTC offset."
                    },
                    "location": {
                        "type": "string",
                        "description": "New location."
                    },
                    "description": {
                        "type": "string",
                        "description": "New description or notes."
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

        let mut patch = serde_json::Map::new();

        if let Some(t) = args.title {
            patch.insert("summary".into(), serde_json::Value::String(t));
        }
        if let Some(start) = args.start_time {
            patch.insert("start".into(), serde_json::json!({ "dateTime": start }));
        }
        if let Some(end) = args.end_time {
            patch.insert("end".into(), serde_json::json!({ "dateTime": end }));
        }
        if let Some(loc) = args.location {
            patch.insert("location".into(), serde_json::Value::String(loc));
        }
        if let Some(desc) = args.description {
            patch.insert("description".into(), serde_json::Value::String(desc));
        }

        let url = format!("{BASE_URL}/{}", args.event_id);
        let response = self
            .http_client
            .patch(&url)
            .bearer_auth(&self.access_token)
            .query(&[("sendUpdates", "all")])
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

// ═══════════════════════════════════════════════════════════════════════════
// Tool 4 — delete_calendar_event
// ═══════════════════════════════════════════════════════════════════════════

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

const DELETE_DESCRIPTION: &str = include_str!("../prompts/calendar_delete_description.md");

impl Tool for DeleteCalendarEventTool {
    const NAME: &'static str = "delete_calendar_event";

    type Error = CalendarError;
    type Args = DeleteEventArgs;
    type Output = DeleteOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: DELETE_DESCRIPTION.trim().to_string(),
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

        let url = format!("{BASE_URL}/{}", args.event_id);
        let response = self
            .http_client
            .delete(&url)
            .bearer_auth(&self.access_token)
            .query(&[("sendUpdates", "all")])
            .send()
            .await?;

        let status = response.status();
        debug!("delete event response status: {status}");

        // 204 No Content on success — don't try to read the body
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

// ═══════════════════════════════════════════════════════════════════════════
// Tool 5 — respond_to_event (accept / decline / tentative)
// ═══════════════════════════════════════════════════════════════════════════

pub struct RespondToEventTool {
    access_token: String,
    http_client: reqwest::Client,
}

impl RespondToEventTool {
    pub fn new(access_token: String) -> Self {
        Self {
            access_token,
            http_client: reqwest::Client::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct RespondArgs {
    pub event_id: String,
    pub response_status: String,
}

#[derive(Serialize)]
pub struct RespondOutput {
    pub updated: bool,
    pub response_status: String,
}

const RESPOND_DESCRIPTION: &str = include_str!("../prompts/calendar_respond_description.md");

impl Tool for RespondToEventTool {
    const NAME: &'static str = "respond_to_event";

    type Error = CalendarError;
    type Args = RespondArgs;
    type Output = RespondOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: RESPOND_DESCRIPTION.trim().to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "required": ["event_id", "response_status"],
                "properties": {
                    "event_id": {
                        "type": "string",
                        "description": "ID of the event to respond to (from get_calendar_events)."
                    },
                    "response_status": {
                        "type": "string",
                        "enum": ["accepted", "declined", "tentative"],
                        "description": "Your RSVP response to the invitation."
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
            response_status = %args.response_status,
            "responding to calendar invitation"
        );

        // Step 1: GET the event to read the current full attendees list.
        let url = format!("{BASE_URL}/{}", args.event_id);
        let get_resp = self
            .http_client
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await?;

        let get_status = get_resp.status();
        let get_body = get_resp.text().await?;
        debug!("get event (for respond) status: {get_status}");

        if !get_status.is_success() {
            return Err(make_api_error(get_status, get_body));
        }

        let ev: ApiEvent = serde_json::from_str(&get_body)
            .map_err(|e| CalendarError::Parse(format!("{e}: {get_body}")))?;

        // Step 2: Update the self attendee's responseStatus in the list.
        let mut attendees: Vec<ApiAttendee> = ev.attendees.unwrap_or_default();
        let self_found = attendees.iter_mut().any(|a| {
            if a.is_self {
                a.response_status = Some(args.response_status.clone());
                true
            } else {
                false
            }
        });

        if !self_found {
            debug!(
                event_id = %args.event_id,
                "no self attendee found; RSVP may not apply"
            );
        }

        // Step 3: PATCH the event back with the updated attendees array.
        let patch = serde_json::json!({ "attendees": attendees });
        let patch_resp = self
            .http_client
            .patch(&url)
            .bearer_auth(&self.access_token)
            .query(&[("sendUpdates", "all")])
            .json(&patch)
            .send()
            .await?;

        let patch_status = patch_resp.status();
        let patch_body = patch_resp.text().await?;
        debug!("respond PATCH status: {patch_status}");

        if !patch_status.is_success() {
            return Err(make_api_error(patch_status, patch_body));
        }

        Ok(RespondOutput {
            updated: true,
            response_status: args.response_status,
        })
    }
}
