use std::collections::HashMap;

use chrono::{DateTime, Duration, FixedOffset};
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument, warn};

use super::common::{TimeRange, FREE_BUSY_URL};
use super::error::{make_api_error, CalendarError};

const DESCRIPTION: &str = include_str!("../../../prompts/google_calendar/find_free_time.md");

pub struct FindFreeTimeTool {
    access_token: String,
    http_client: reqwest::Client,
}

impl FindFreeTimeTool {
    pub fn new(access_token: String) -> Self {
        Self {
            access_token,
            http_client: reqwest::Client::new(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FindFreeTimeArgs {
    pub start_time: String,
    pub end_time: String,
    /// Calendar IDs to query. Defaults to ["primary"]. Use email addresses for
    /// other people's calendars (only works if they're shared with the user).
    pub calendars: Option<Vec<String>>,
    /// Minimum free-gap length to report, in minutes. Defaults to 15.
    pub min_duration_minutes: Option<u32>,
}

#[derive(Serialize)]
pub struct FindFreeTimeOutput {
    /// Merged busy ranges across all queried calendars (sorted, coalesced).
    pub busy: Vec<TimeRange>,
    /// Computed free gaps within the requested window, each ≥ `min_duration_minutes`.
    pub free: Vec<TimeRange>,
    /// Per-calendar errors reported by the freeBusy API (e.g. "notFound" if the
    /// user can't read that calendar). Empty on success.
    pub errors: Vec<FreeBusyErrorReport>,
}

#[derive(Serialize)]
pub struct FreeBusyErrorReport {
    pub calendar: String,
    pub reason: String,
}

#[derive(Deserialize)]
struct FreeBusyResponse {
    calendars: HashMap<String, FreeBusyCalendar>,
}

#[derive(Deserialize)]
struct FreeBusyCalendar {
    busy: Option<Vec<FreeBusyRange>>,
    errors: Option<Vec<FreeBusyApiError>>,
}

#[derive(Deserialize)]
struct FreeBusyRange {
    start: String,
    end: String,
}

#[derive(Deserialize)]
struct FreeBusyApiError {
    reason: String,
}

impl Tool for FindFreeTimeTool {
    const NAME: &'static str = "find_free_time";

    type Error = CalendarError;
    type Args = FindFreeTimeArgs;
    type Output = FindFreeTimeOutput;

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
                        "description": "Start of the search window in ISO 8601 with UTC offset."
                    },
                    "end_time": {
                        "type": "string",
                        "description": "End of the search window in ISO 8601 with UTC offset."
                    },
                    "calendars": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Calendar IDs to check. Defaults to [\"primary\"]. \
                            Use email addresses to check other people (requires their calendar \
                            to be shared)."
                    },
                    "min_duration_minutes": {
                        "type": "integer",
                        "description": "Minimum free-gap length to report, in minutes. Defaults to 15.",
                        "minimum": 1
                    }
                }
            }),
        }
    }

    #[instrument(skip(self))]
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let window_start = DateTime::parse_from_rfc3339(&args.start_time).map_err(|e| {
            CalendarError::InvalidArg(format!("start_time not valid RFC3339: {e}"))
        })?;
        let window_end = DateTime::parse_from_rfc3339(&args.end_time)
            .map_err(|e| CalendarError::InvalidArg(format!("end_time not valid RFC3339: {e}")))?;
        if window_end <= window_start {
            return Err(CalendarError::InvalidArg(
                "end_time must be after start_time".to_string(),
            ));
        }

        let calendars = args
            .calendars
            .unwrap_or_else(|| vec!["primary".to_string()]);
        let min_duration = Duration::minutes(args.min_duration_minutes.unwrap_or(15) as i64);

        debug!(
            access_token_len = self.access_token.len(),
            start = %args.start_time,
            end = %args.end_time,
            calendars = ?calendars,
            min_duration_mins = min_duration.num_minutes(),
            "looking up free/busy"
        );

        let request_body = serde_json::json!({
            "timeMin": args.start_time,
            "timeMax": args.end_time,
            "items": calendars.iter().map(|id| serde_json::json!({ "id": id })).collect::<Vec<_>>(),
        });

        let response = self
            .http_client
            .post(FREE_BUSY_URL)
            .bearer_auth(&self.access_token)
            .json(&request_body)
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;
        debug!("freeBusy response status: {status}");

        if !status.is_success() {
            return Err(make_api_error(status, body));
        }

        let api_resp: FreeBusyResponse = serde_json::from_str(&body)
            .map_err(|e| CalendarError::Parse(format!("{e}: {body}")))?;

        let mut raw_busy: Vec<(DateTime<FixedOffset>, DateTime<FixedOffset>)> = Vec::new();
        let mut errors: Vec<FreeBusyErrorReport> = Vec::new();

        for (calendar_id, cal) in api_resp.calendars {
            if let Some(errs) = cal.errors {
                for err in errs {
                    warn!(
                        calendar = %calendar_id,
                        reason = %err.reason,
                        "freeBusy reported per-calendar error"
                    );
                    errors.push(FreeBusyErrorReport {
                        calendar: calendar_id.clone(),
                        reason: err.reason,
                    });
                }
            }
            for r in cal.busy.unwrap_or_default() {
                let s = match DateTime::parse_from_rfc3339(&r.start) {
                    Ok(v) => v,
                    Err(e) => {
                        warn!("skipping busy range with invalid start {}: {e}", r.start);
                        continue;
                    }
                };
                let e = match DateTime::parse_from_rfc3339(&r.end) {
                    Ok(v) => v,
                    Err(err) => {
                        warn!("skipping busy range with invalid end {}: {err}", r.end);
                        continue;
                    }
                };
                raw_busy.push((s, e));
            }
        }

        raw_busy.sort_by_key(|&(s, _)| s);
        let mut merged: Vec<(DateTime<FixedOffset>, DateTime<FixedOffset>)> = Vec::new();
        for (s, e) in raw_busy {
            if let Some(last) = merged.last_mut() {
                if s <= last.1 {
                    if e > last.1 {
                        last.1 = e;
                    }
                    continue;
                }
            }
            merged.push((s, e));
        }

        let mut free: Vec<TimeRange> = Vec::new();
        let mut cursor = window_start;
        for &(s, e) in &merged {
            if s > cursor && (s - cursor) >= min_duration {
                free.push(TimeRange {
                    start: cursor.to_rfc3339(),
                    end: s.to_rfc3339(),
                });
            }
            if e > cursor {
                cursor = e;
            }
        }
        if window_end > cursor && (window_end - cursor) >= min_duration {
            free.push(TimeRange {
                start: cursor.to_rfc3339(),
                end: window_end.to_rfc3339(),
            });
        }

        let busy: Vec<TimeRange> = merged
            .into_iter()
            .map(|(s, e)| TimeRange {
                start: s.to_rfc3339(),
                end: e.to_rfc3339(),
            })
            .collect();

        Ok(FindFreeTimeOutput { busy, free, errors })
    }
}
