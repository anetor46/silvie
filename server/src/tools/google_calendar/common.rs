use chrono::Utc;
use serde::{Deserialize, Serialize};

pub(super) const EVENTS_URL: &str =
    "https://www.googleapis.com/calendar/v3/calendars/primary/events";
pub(super) const FREE_BUSY_URL: &str = "https://www.googleapis.com/calendar/v3/freeBusy";

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Serialize a date-or-datetime string for the Calendar API. If `value` looks
/// like a bare date (`YYYY-MM-DD`, exactly 10 chars, no `T`), it produces
/// `{"date": value}` (all-day event); otherwise `{"dateTime": value}`.
pub(super) fn datetime_field(value: &str) -> serde_json::Value {
    let trimmed = value.trim();
    if trimmed.len() == 10 && !trimmed.contains('T') && trimmed.matches('-').count() == 2 {
        serde_json::json!({ "date": trimmed })
    } else {
        serde_json::json!({ "dateTime": trimmed })
    }
}

pub(super) fn unique_request_id() -> String {
    let nanos = Utc::now().timestamp_nanos_opt().unwrap_or(0);
    format!("silvie-meet-{nanos}")
}

pub(super) fn conference_create_block() -> serde_json::Value {
    serde_json::json!({
        "createRequest": {
            "requestId": unique_request_id(),
            "conferenceSolutionKey": { "type": "hangoutsMeet" },
        }
    })
}

// ── Output types ─────────────────────────────────────────────────────────────

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
    pub meet_link: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TimeRange {
    pub start: String,
    pub end: String,
}

// ── Internal API types ───────────────────────────────────────────────────────

#[derive(Deserialize)]
pub(super) struct ApiEvent {
    pub(super) id: Option<String>,
    pub(super) summary: Option<String>,
    pub(super) start: ApiEventTime,
    pub(super) end: ApiEventTime,
    pub(super) location: Option<String>,
    pub(super) description: Option<String>,
    pub(super) attendees: Option<Vec<ApiAttendee>>,
    #[serde(rename = "conferenceData")]
    pub(super) conference_data: Option<ApiConferenceData>,
}

#[derive(Deserialize)]
pub(super) struct ApiEventTime {
    #[serde(rename = "dateTime")]
    pub(super) date_time: Option<String>,
    pub(super) date: Option<String>,
}

impl ApiEventTime {
    pub(super) fn as_str(&self) -> String {
        self.date_time
            .clone()
            .or_else(|| self.date.clone())
            .unwrap_or_else(|| "unknown".to_string())
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub(super) struct ApiAttendee {
    pub(super) email: String,
    #[serde(rename = "displayName", skip_serializing_if = "Option::is_none")]
    pub(super) display_name: Option<String>,
    #[serde(rename = "responseStatus", skip_serializing_if = "Option::is_none")]
    pub(super) response_status: Option<String>,
    #[serde(rename = "self", default, skip_serializing_if = "is_false")]
    pub(super) is_self: bool,
}

fn is_false(b: &bool) -> bool {
    !b
}

#[derive(Deserialize)]
pub(super) struct ApiConferenceData {
    #[serde(rename = "entryPoints")]
    pub(super) entry_points: Option<Vec<ApiEntryPoint>>,
}

#[derive(Deserialize)]
pub(super) struct ApiEntryPoint {
    #[serde(rename = "entryPointType")]
    pub(super) entry_point_type: Option<String>,
    pub(super) uri: Option<String>,
}

pub(super) fn extract_meet_link(ev: &ApiEvent) -> Option<String> {
    ev.conference_data
        .as_ref()?
        .entry_points
        .as_ref()?
        .iter()
        .find(|ep| ep.entry_point_type.as_deref() == Some("video"))
        .and_then(|ep| ep.uri.clone())
}

pub(super) fn parse_api_event(ev: ApiEvent) -> CalendarEvent {
    let meet_link = extract_meet_link(&ev);
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
        meet_link,
    }
}
