use base64::{engine::general_purpose, Engine};
use serde::{Deserialize, Serialize};

use super::error::GmailError;

pub(super) const GMAIL_BASE: &str = "https://gmail.googleapis.com/gmail/v1/users/me";

// ── Output types returned to the LLM ────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct EmailSummary {
    pub id: String,
    pub thread_id: String,
    pub from: String,
    pub to: String,
    pub subject: String,
    pub date: String,
    pub snippet: String,
    pub unread: bool,
}

#[derive(Debug, Serialize)]
pub struct EmailFull {
    pub id: String,
    pub thread_id: String,
    pub from: String,
    pub to: String,
    pub cc: Option<String>,
    pub subject: String,
    pub date: String,
    /// The `Message-ID` header — pass this to `reply_to_email` as
    /// `message_id_header` so threading works correctly.
    pub message_id_header: Option<String>,
    pub body: String,
    /// True when the body exceeded 6 000 characters and was cut.
    pub truncated: bool,
}

#[derive(Debug, Serialize)]
pub struct SentMessage {
    pub id: String,
    pub thread_id: String,
}

// ── Gmail API wire types ─────────────────────────────────────────────────────

#[derive(Deserialize)]
pub(super) struct ApiListResponse {
    pub(super) messages: Option<Vec<ApiMessageRef>>,
}

#[derive(Deserialize)]
pub(super) struct ApiMessageRef {
    pub(super) id: String,
    #[serde(rename = "threadId")]
    pub(super) thread_id: String,
}

#[derive(Deserialize)]
pub(super) struct ApiMessage {
    pub(super) id: Option<String>,
    #[serde(rename = "threadId")]
    pub(super) thread_id: Option<String>,
    pub(super) snippet: Option<String>,
    #[serde(rename = "labelIds")]
    pub(super) label_ids: Option<Vec<String>>,
    pub(super) payload: Option<ApiMessagePart>,
}

#[derive(Deserialize)]
pub(super) struct ApiMessagePart {
    #[serde(rename = "mimeType")]
    pub(super) mime_type: Option<String>,
    pub(super) headers: Option<Vec<ApiHeader>>,
    pub(super) body: Option<ApiPartBody>,
    pub(super) parts: Option<Vec<ApiMessagePart>>,
}

#[derive(Deserialize)]
pub(super) struct ApiHeader {
    pub(super) name: String,
    pub(super) value: String,
}

#[derive(Deserialize)]
pub(super) struct ApiPartBody {
    pub(super) data: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct ApiSendResponse {
    pub(super) id: String,
    #[serde(rename = "threadId")]
    pub(super) thread_id: String,
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Decode base64url (Gmail's encoding) into a UTF-8 string, lossy.
pub(super) fn decode_base64url(encoded: &str) -> Result<String, GmailError> {
    // Gmail may omit padding — use the no-pad URL-safe engine.
    let bytes = general_purpose::URL_SAFE_NO_PAD
        .decode(encoded)
        .or_else(|_| general_purpose::URL_SAFE.decode(encoded))
        .map_err(|e| GmailError::Parse(format!("base64url decode: {e}")))?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// Encode bytes as base64url (no-pad), as Gmail expects for raw send.
pub(super) fn encode_base64url(input: &[u8]) -> String {
    general_purpose::URL_SAFE_NO_PAD.encode(input)
}

/// Pull a named header out of a flat list.
pub(super) fn header_value<'a>(headers: &'a [ApiHeader], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|h| h.name.eq_ignore_ascii_case(name))
        .map(|h| h.value.as_str())
}

/// Recursively find the first `text/plain` body part and decode it.
pub(super) fn extract_plain_text(part: &ApiMessagePart) -> Option<String> {
    let mime = part.mime_type.as_deref().unwrap_or("");

    if mime == "text/plain" {
        if let Some(data) = part.body.as_ref().and_then(|b| b.data.as_deref()) {
            return decode_base64url(data).ok();
        }
    }

    // Recurse into sub-parts (multipart/alternative, multipart/mixed, etc.).
    if let Some(sub_parts) = &part.parts {
        // Prefer text/plain over html — look for it first.
        for sub in sub_parts {
            if sub.mime_type.as_deref() == Some("text/plain") {
                if let Some(text) = extract_plain_text(sub) {
                    return Some(text);
                }
            }
        }
        // Fall back to any sub-part (e.g. text/html in html-only emails).
        for sub in sub_parts {
            if let Some(text) = extract_plain_text(sub) {
                return Some(text);
            }
        }
    }

    None
}

/// Build an RFC 2822 raw message string for the Gmail send API, then
/// base64url-encode the whole thing into the `raw` field.
pub(super) fn build_raw_message(
    to: &[String],
    cc: &[String],
    subject: &str,
    body: &str,
    in_reply_to: Option<&str>,
    references: Option<&str>,
) -> String {
    let body_b64 = general_purpose::STANDARD.encode(body.as_bytes());
    let subject_enc = if subject.is_ascii() {
        subject.to_owned()
    } else {
        format!(
            "=?UTF-8?B?{}?=",
            general_purpose::STANDARD.encode(subject.as_bytes())
        )
    };

    let mut msg = format!(
        "MIME-Version: 1.0\r\nTo: {}\r\n",
        to.join(", ")
    );
    if !cc.is_empty() {
        msg.push_str(&format!("Cc: {}\r\n", cc.join(", ")));
    }
    msg.push_str(&format!("Subject: {subject_enc}\r\n"));
    if let Some(irt) = in_reply_to {
        msg.push_str(&format!("In-Reply-To: {irt}\r\n"));
        let refs_val = references.unwrap_or(irt);
        msg.push_str(&format!("References: {refs_val}\r\n"));
    }
    msg.push_str("Content-Type: text/plain; charset=UTF-8\r\n");
    msg.push_str("Content-Transfer-Encoding: base64\r\n\r\n");
    // RFC 2045 §6.8: fold at 76 characters.
    let folded = body_b64
        .as_bytes()
        .chunks(76)
        .map(|c| std::str::from_utf8(c).unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\r\n");
    msg.push_str(&folded);
    msg
}
