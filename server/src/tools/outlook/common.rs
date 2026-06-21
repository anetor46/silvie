use serde::{Deserialize, Serialize};

pub(super) const GRAPH_BASE: &str = "https://graph.microsoft.com/v1.0/me";

// ── Output types returned to the LLM ────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct OutlookEmailSummary {
    pub id: String,
    pub subject: String,
    pub from: String,
    pub to: Vec<String>,
    pub received_at: String,
    pub body_preview: String,
    pub is_read: bool,
}

#[derive(Debug, Serialize)]
pub struct OutlookEmailFull {
    pub id: String,
    pub subject: String,
    pub from: String,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub received_at: String,
    pub internet_message_id: Option<String>,
    pub body: String,
    pub truncated: bool,
}

#[derive(Debug, Serialize)]
pub struct OutlookEvent {
    pub id: String,
    pub subject: String,
    pub start: String,
    pub end: String,
    pub location: Option<String>,
    pub organizer: Option<String>,
    pub attendees: Vec<String>,
    pub body_preview: String,
}

// ── Microsoft Graph API wire types ──────────────────────────────────────────

#[derive(Deserialize)]
pub(super) struct ApiEmailAddress {
    pub(super) name: Option<String>,
    pub(super) address: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct ApiRecipient {
    #[serde(rename = "emailAddress")]
    pub(super) email_address: ApiEmailAddress,
}

impl ApiRecipient {
    pub(super) fn display(&self) -> String {
        let addr = self.email_address.address.as_deref().unwrap_or("");
        match self.email_address.name.as_deref() {
            Some(name) if !name.is_empty() && name != addr => {
                format!("{name} <{addr}>")
            }
            _ => addr.to_string(),
        }
    }
}

#[derive(Deserialize)]
pub(super) struct ApiMessageListResponse {
    pub(super) value: Vec<ApiMessageSummary>,
}

#[derive(Deserialize)]
pub(super) struct ApiMessageSummary {
    pub(super) id: String,
    pub(super) subject: Option<String>,
    pub(super) from: Option<ApiRecipient>,
    #[serde(rename = "toRecipients")]
    pub(super) to_recipients: Option<Vec<ApiRecipient>>,
    #[serde(rename = "receivedDateTime")]
    pub(super) received_date_time: Option<String>,
    #[serde(rename = "bodyPreview")]
    pub(super) body_preview: Option<String>,
    #[serde(rename = "isRead")]
    pub(super) is_read: Option<bool>,
}

#[derive(Deserialize)]
pub(super) struct ApiMessageFull {
    pub(super) id: String,
    pub(super) subject: Option<String>,
    pub(super) from: Option<ApiRecipient>,
    #[serde(rename = "toRecipients")]
    pub(super) to_recipients: Option<Vec<ApiRecipient>>,
    #[serde(rename = "ccRecipients")]
    pub(super) cc_recipients: Option<Vec<ApiRecipient>>,
    #[serde(rename = "receivedDateTime")]
    pub(super) received_date_time: Option<String>,
    #[serde(rename = "internetMessageId")]
    pub(super) internet_message_id: Option<String>,
    pub(super) body: Option<ApiBody>,
}

#[derive(Deserialize)]
pub(super) struct ApiBody {
    pub(super) content: Option<String>,
}

pub(super) fn parse_message_summary(m: ApiMessageSummary) -> OutlookEmailSummary {
    let from = m
        .from
        .as_ref()
        .map(|r| r.display())
        .unwrap_or_default();
    let to = m
        .to_recipients
        .unwrap_or_default()
        .iter()
        .map(|r| r.display())
        .collect();
    OutlookEmailSummary {
        id: m.id,
        subject: m.subject.unwrap_or_else(|| "(no subject)".to_string()),
        from,
        to,
        received_at: m.received_date_time.unwrap_or_default(),
        body_preview: m.body_preview.unwrap_or_default(),
        is_read: m.is_read.unwrap_or(true),
    }
}

// ── Calendar wire types ──────────────────────────────────────────────────────

#[derive(Deserialize)]
pub(super) struct ApiEventListResponse {
    pub(super) value: Vec<ApiEventItem>,
}

#[derive(Deserialize)]
pub(super) struct ApiEventItem {
    pub(super) id: String,
    pub(super) subject: Option<String>,
    pub(super) start: Option<ApiDateTime>,
    pub(super) end: Option<ApiDateTime>,
    pub(super) location: Option<ApiLocation>,
    pub(super) organizer: Option<ApiRecipient>,
    pub(super) attendees: Option<Vec<ApiAttendee>>,
    #[serde(rename = "bodyPreview")]
    pub(super) body_preview: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub(super) struct ApiDateTime {
    #[serde(rename = "dateTime")]
    pub(super) date_time: String,
    #[serde(rename = "timeZone")]
    pub(super) time_zone: String,
}

#[derive(Deserialize)]
pub(super) struct ApiLocation {
    #[serde(rename = "displayName")]
    pub(super) display_name: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct ApiAttendee {
    #[serde(rename = "emailAddress")]
    pub(super) email_address: ApiEmailAddress,
}

pub(super) fn parse_event(e: ApiEventItem) -> OutlookEvent {
    let start = e
        .start
        .map(|dt| dt.date_time)
        .unwrap_or_default();
    let end = e
        .end
        .map(|dt| dt.date_time)
        .unwrap_or_default();
    let location = e
        .location
        .and_then(|l| l.display_name)
        .filter(|s| !s.is_empty());
    let organizer = e.organizer.as_ref().map(|r| r.display());
    let attendees = e
        .attendees
        .unwrap_or_default()
        .iter()
        .map(|a| {
            a.email_address
                .address
                .clone()
                .unwrap_or_default()
        })
        .collect();
    OutlookEvent {
        id: e.id,
        subject: e.subject.unwrap_or_else(|| "(no subject)".to_string()),
        start,
        end,
        location,
        organizer,
        attendees,
        body_preview: e.body_preview.unwrap_or_default(),
    }
}
