use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};

use super::error::{make_api_error, OutlookError};

const DESCRIPTION: &str = include_str!("../../../prompts/outlook/find_free_time.md");
const SCHEDULE_URL: &str = "https://graph.microsoft.com/v1.0/me/calendar/getSchedule";

pub struct FindOutlookFreeTimeTool {
    access_token: String,
    http_client: reqwest::Client,
}

impl FindOutlookFreeTimeTool {
    pub fn new(access_token: String) -> Self {
        Self {
            access_token,
            http_client: reqwest::Client::new(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FindOutlookFreeTimeArgs {
    pub start_time: String,
    pub end_time: String,
    /// Granularity in minutes for the availability view. Defaults to 30.
    pub interval_minutes: Option<u32>,
    /// Additional email addresses whose schedules to check. The user's own
    /// schedule is always included.
    pub schedules: Option<Vec<String>>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct BusyBlock {
    pub start: String,
    pub end: String,
    pub status: String,
}

#[derive(Serialize)]
pub struct FindOutlookFreeTimeOutput {
    pub schedule_items: Vec<BusyBlock>,
    pub availability_view: Option<String>,
}

impl Tool for FindOutlookFreeTimeTool {
    const NAME: &'static str = "find_outlook_free_time";

    type Error = OutlookError;
    type Args = FindOutlookFreeTimeArgs;
    type Output = FindOutlookFreeTimeOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: DESCRIPTION.trim().to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "required": ["start_time", "end_time"],
                "properties": {
                    "start_time": {
                        "type": "string",
                        "description": "Start of the range in ISO 8601 UTC."
                    },
                    "end_time": {
                        "type": "string",
                        "description": "End of the range in ISO 8601 UTC."
                    },
                    "interval_minutes": {
                        "type": "integer",
                        "description": "Availability view granularity in minutes (default 30).",
                        "minimum": 5,
                        "maximum": 60
                    },
                    "schedules": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Additional email addresses to check."
                    }
                }
            }),
        }
    }

    #[instrument(skip(self))]
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let interval = args.interval_minutes.unwrap_or(30);
        debug!(%args.start_time, %args.end_time, interval, "checking Outlook free time");

        // Fetch the user's own email first so we can include it in the schedule request.
        let me_resp = self
            .http_client
            .get("https://graph.microsoft.com/v1.0/me")
            .bearer_auth(&self.access_token)
            .query(&[("$select", "mail,userPrincipalName")])
            .send()
            .await?;
        let me_status = me_resp.status();
        let me_body = me_resp.text().await?;
        if !me_status.is_success() {
            return Err(make_api_error(me_status, me_body));
        }
        #[derive(Deserialize)]
        struct Me {
            mail: Option<String>,
            #[serde(rename = "userPrincipalName")]
            upn: Option<String>,
        }
        let me: Me = serde_json::from_str(&me_body)
            .map_err(|e| OutlookError::Parse(format!("{e}: {me_body}")))?;
        let my_email = me.mail.or(me.upn).unwrap_or_default();

        let mut schedules = args.schedules.unwrap_or_default();
        if !my_email.is_empty() && !schedules.contains(&my_email) {
            schedules.insert(0, my_email);
        }

        let payload = serde_json::json!({
            "schedules": schedules,
            "startTime": { "dateTime": args.start_time, "timeZone": "UTC" },
            "endTime":   { "dateTime": args.end_time,   "timeZone": "UTC" },
            "availabilityViewInterval": interval
        });

        let resp = self
            .http_client
            .post(SCHEDULE_URL)
            .bearer_auth(&self.access_token)
            .json(&payload)
            .send()
            .await?;

        let status = resp.status();
        let body = resp.text().await?;
        if !status.is_success() {
            return Err(make_api_error(status, body));
        }

        #[derive(Deserialize)]
        struct ScheduleItem {
            #[serde(rename = "start")]
            start: Option<serde_json::Value>,
            #[serde(rename = "end")]
            end: Option<serde_json::Value>,
            status: Option<String>,
        }

        #[derive(Deserialize)]
        struct ScheduleResponse {
            #[serde(rename = "scheduleItems")]
            schedule_items: Option<Vec<ScheduleItem>>,
            #[serde(rename = "availabilityView")]
            availability_view: Option<String>,
        }

        #[derive(Deserialize)]
        struct GraphListValue {
            value: Vec<ScheduleResponse>,
        }

        let parsed: GraphListValue = serde_json::from_str(&body)
            .map_err(|e| OutlookError::Parse(format!("{e}: {body}")))?;

        let first = parsed.value.into_iter().next().unwrap_or(ScheduleResponse {
            schedule_items: None,
            availability_view: None,
        });

        let busy: Vec<BusyBlock> = first
            .schedule_items
            .unwrap_or_default()
            .into_iter()
            .filter_map(|si| {
                let start = si
                    .start
                    .as_ref()
                    .and_then(|v| v.get("dateTime"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let end = si
                    .end
                    .as_ref()
                    .and_then(|v| v.get("dateTime"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                Some(BusyBlock {
                    start,
                    end,
                    status: si.status.unwrap_or_else(|| "busy".to_string()),
                })
            })
            .collect();

        Ok(FindOutlookFreeTimeOutput {
            schedule_items: busy,
            availability_view: first.availability_view,
        })
    }
}
