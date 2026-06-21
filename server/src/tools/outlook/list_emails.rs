use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};

use super::common::{parse_message_summary, ApiMessageListResponse, OutlookEmailSummary, GRAPH_BASE};
use super::error::{make_api_error, OutlookError};

const DESCRIPTION: &str = include_str!("../../../prompts/outlook/list_emails.md");

pub struct ListOutlookEmailsTool {
    access_token: String,
    http_client: reqwest::Client,
}

impl ListOutlookEmailsTool {
    pub fn new(access_token: String) -> Self {
        Self {
            access_token,
            http_client: reqwest::Client::new(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ListOutlookEmailsArgs {
    /// OData $filter expression (e.g. "isRead eq false").
    pub filter: Option<String>,
    /// Keyword search query.
    pub search: Option<String>,
    pub max_results: Option<u32>,
}

#[derive(Serialize)]
pub struct ListOutlookEmailsOutput {
    pub emails: Vec<OutlookEmailSummary>,
    pub total_returned: usize,
}

impl Tool for ListOutlookEmailsTool {
    const NAME: &'static str = "list_outlook_emails";

    type Error = OutlookError;
    type Args = ListOutlookEmailsArgs;
    type Output = ListOutlookEmailsOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: DESCRIPTION.trim().to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "filter": {
                        "type": "string",
                        "description": "OData $filter expression \
                            (e.g. 'isRead eq false', \
                            \"from/emailAddress/address eq 'alice@example.com'\"). \
                            Omit for the most recent emails."
                    },
                    "search": {
                        "type": "string",
                        "description": "Keyword search query across subject, body, and sender."
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum number of emails to return (1–25). Defaults to 10.",
                        "minimum": 1,
                        "maximum": 25
                    }
                }
            }),
        }
    }

    #[instrument(skip(self))]
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let max = args.max_results.unwrap_or(10).clamp(1, 25);
        debug!(filter = ?args.filter, search = ?args.search, max, "listing Outlook emails");

        let select = "id,subject,from,toRecipients,receivedDateTime,bodyPreview,isRead";
        let max_str = max.to_string();
        let mut params: Vec<(&str, &str)> = vec![
            ("$select", select),
            ("$top", &max_str),
            ("$orderby", "receivedDateTime desc"),
        ];

        let filter_str;
        if let Some(f) = args.filter.as_deref() {
            filter_str = f.to_string();
            params.push(("$filter", &filter_str));
        }

        let url = if args.search.is_some() {
            format!("{GRAPH_BASE}/messages")
        } else {
            format!("{GRAPH_BASE}/messages")
        };

        let mut req = self.http_client.get(&url).bearer_auth(&self.access_token);

        // $search and $filter are mutually exclusive in Graph.
        if let Some(s) = args.search.as_deref() {
            req = req.query(&[("$search", &format!("\"{s}\""))]);
            req = req.query(&[("$select", select), ("$top", &max_str)]);
        } else {
            req = req.query(&params);
        }

        let resp = req.send().await?;
        let status = resp.status();
        let body = resp.text().await?;
        if !status.is_success() {
            return Err(make_api_error(status, body));
        }

        let list: ApiMessageListResponse = serde_json::from_str(&body)
            .map_err(|e| OutlookError::Parse(format!("{e}: {body}")))?;

        let emails: Vec<_> = list.value.into_iter().map(parse_message_summary).collect();
        let total = emails.len();
        Ok(ListOutlookEmailsOutput { emails, total_returned: total })
    }
}
