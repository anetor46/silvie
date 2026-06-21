use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};

use super::common::{ApiAttendee, ApiEvent, EVENTS_URL};
use super::error::{make_api_error, CalendarError};

const DESCRIPTION: &str = include_str!("../../../prompts/google_calendar/respond_event.md");

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

#[derive(Debug, Deserialize, Serialize)]
pub struct RespondArgs {
    pub event_id: String,
    pub response_status: String,
}

#[derive(Serialize)]
pub struct RespondOutput {
    pub updated: bool,
    pub response_status: String,
}

impl Tool for RespondToEventTool {
    const NAME: &'static str = "respond_to_event";

    type Error = CalendarError;
    type Args = RespondArgs;
    type Output = RespondOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: DESCRIPTION.trim().to_string(),
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

        let url = format!("{EVENTS_URL}/{}", args.event_id);
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
