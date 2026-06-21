//! ORM layer for `conversations` + `messages`. The public CRUD operates on
//! conversations the JWT-validated user owns. The `insert_user_message` /
//! `insert_assistant_message` / `load_history` helpers are used by the chat
//! handler — they persist the back-and-forth as the streaming response unfolds.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use diesel::{
    ExpressionMethods, Insertable, JoinOnDsl, NullableExpressionMethods, OptionalExtension,
    QueryDsl, Queryable, Selectable, SelectableHelper,
};
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};
use uuid::Uuid;

use crate::{
    db::DbPool,
    schema::{conversations, messages},
};

/// Max length of an auto-generated title (chars). Anything longer gets
/// truncated with an ellipsis.
const AUTO_TITLE_MAX_LEN: usize = 60;

// ── Models ──────────────────────────────────────────────────────────────────

#[derive(Queryable, Selectable, Serialize, Debug, Clone)]
#[diesel(table_name = conversations)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Conversation {
    pub id: Uuid,
    pub user_id: Uuid,
    pub organization_id: Option<Uuid>,
    pub title: Option<String>,
    pub model: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Queryable, Selectable, Serialize, Debug, Clone)]
#[diesel(table_name = messages)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Message {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub role: String,
    pub content: String,
    pub tool_name: Option<String>,
    pub tool_call_id: Option<String>,
    pub prompt_tokens: Option<i32>,
    pub completion_tokens: Option<i32>,
    pub latency_ms: Option<i32>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = conversations)]
struct NewConversation {
    user_id: Uuid,
}

#[derive(Insertable)]
#[diesel(table_name = messages)]
struct NewMessage<'a> {
    conversation_id: Uuid,
    role: &'a str,
    content: &'a str,
}

#[derive(Insertable)]
#[diesel(table_name = messages)]
struct NewToolMessage<'a> {
    conversation_id: Uuid,
    role: &'a str,
    content: &'a str,
    tool_name: &'a str,
    tool_call_id: &'a str,
}

// ── Request / response shapes ───────────────────────────────────────────────

#[derive(Serialize, Debug)]
pub struct ConversationWithMessages {
    #[serde(flatten)]
    pub conversation: Conversation,
    pub messages: Vec<Message>,
}

#[derive(Deserialize)]
pub struct UpdateConversationRequest {
    pub title: Option<String>,
}

// ── Public CRUD ─────────────────────────────────────────────────────────────

#[instrument(skip(pool))]
pub async fn list_for_user(pool: &DbPool, user_id: Uuid) -> Result<Vec<Conversation>> {
    let mut conn = pool.get().await.context("Failed to get DB connection")?;
    conversations::table
        .filter(conversations::user_id.eq(user_id))
        .filter(conversations::deleted_at.is_null())
        .order(conversations::updated_at.desc())
        .select(Conversation::as_select())
        .load(&mut conn)
        .await
        .context("Failed to list conversations")
}

#[instrument(skip(pool))]
pub async fn create_for_user(pool: &DbPool, user_id: Uuid) -> Result<Conversation> {
    let mut conn = pool.get().await.context("Failed to get DB connection")?;
    let row: Conversation = diesel::insert_into(conversations::table)
        .values(NewConversation { user_id })
        .returning(Conversation::as_returning())
        .get_result(&mut conn)
        .await
        .context("Failed to insert conversation")?;
    info!(conversation_id = %row.id, "conversation created");
    Ok(row)
}

/// Look up a conversation owned by the given user. Returns `None` if the
/// conversation doesn't exist (or isn't owned by them — same response either
/// way, so we don't leak whether the id is just unknown vs. someone else's).
pub async fn find_owned(
    pool: &DbPool,
    user_id: Uuid,
    conv_id: Uuid,
) -> Result<Option<Conversation>> {
    let mut conn = pool.get().await.context("Failed to get DB connection")?;
    conversations::table
        .filter(conversations::id.eq(conv_id))
        .filter(conversations::user_id.eq(user_id))
        .filter(conversations::deleted_at.is_null())
        .select(Conversation::as_select())
        .first(&mut conn)
        .await
        .optional()
        .context("Failed to look up conversation")
}

#[instrument(skip(pool))]
pub async fn fetch_with_messages(
    pool: &DbPool,
    user_id: Uuid,
    conv_id: Uuid,
) -> Result<Option<ConversationWithMessages>> {
    let convo = match find_owned(pool, user_id, conv_id).await? {
        Some(c) => c,
        None => return Ok(None),
    };
    let mut conn = pool.get().await.context("Failed to get DB connection")?;
    let msgs: Vec<Message> = messages::table
        .filter(messages::conversation_id.eq(convo.id))
        .order(messages::created_at.asc())
        .select(Message::as_select())
        .load(&mut conn)
        .await
        .context("Failed to load messages")?;
    Ok(Some(ConversationWithMessages {
        conversation: convo,
        messages: msgs,
    }))
}

#[instrument(skip(pool))]
pub async fn soft_delete(pool: &DbPool, user_id: Uuid, conv_id: Uuid) -> Result<bool> {
    let mut conn = pool.get().await.context("Failed to get DB connection")?;
    let n: usize = diesel::update(
        conversations::table
            .filter(conversations::id.eq(conv_id))
            .filter(conversations::user_id.eq(user_id))
            .filter(conversations::deleted_at.is_null()),
    )
    .set((
        conversations::deleted_at.eq(diesel::dsl::now),
        conversations::updated_at.eq(diesel::dsl::now),
    ))
    .execute(&mut conn)
    .await
    .context("Failed to soft-delete conversation")?;
    Ok(n > 0)
}

#[instrument(skip(pool, title))]
pub async fn update_title(
    pool: &DbPool,
    user_id: Uuid,
    conv_id: Uuid,
    title: Option<String>,
) -> Result<Option<Conversation>> {
    let mut conn = pool.get().await.context("Failed to get DB connection")?;
    let n: usize = diesel::update(
        conversations::table
            .filter(conversations::id.eq(conv_id))
            .filter(conversations::user_id.eq(user_id))
            .filter(conversations::deleted_at.is_null()),
    )
    .set((
        conversations::title.eq(title),
        conversations::updated_at.eq(diesel::dsl::now),
    ))
    .execute(&mut conn)
    .await
    .context("Failed to update title")?;
    if n == 0 {
        return Ok(None);
    }
    conversations::table
        .filter(conversations::id.eq(conv_id))
        .select(Conversation::as_select())
        .first(&mut conn)
        .await
        .optional()
        .context("Failed to re-fetch conversation after title update")
}

// ── Internal helpers (used by chat handler) ─────────────────────────────────

/// Insert a user message; bump conversation.updated_at so sidebar order
/// stays fresh. Returns the inserted row.
pub async fn insert_user_message(
    pool: &DbPool,
    conv_id: Uuid,
    content: &str,
) -> Result<Message> {
    insert_message(pool, conv_id, "user", content).await
}

/// Insert an assistant message; bump conversation.updated_at.
pub async fn insert_assistant_message(
    pool: &DbPool,
    conv_id: Uuid,
    content: &str,
) -> Result<Message> {
    insert_message(pool, conv_id, "assistant", content).await
}

async fn insert_message(
    pool: &DbPool,
    conv_id: Uuid,
    role: &str,
    content: &str,
) -> Result<Message> {
    let mut conn = pool.get().await.context("Failed to get DB connection")?;
    let row: Message = diesel::insert_into(messages::table)
        .values(NewMessage {
            conversation_id: conv_id,
            role,
            content,
        })
        .returning(Message::as_returning())
        .get_result(&mut conn)
        .await
        .context("Failed to insert message")?;
    let _: usize = diesel::update(conversations::table.filter(conversations::id.eq(conv_id)))
        .set(conversations::updated_at.eq(diesel::dsl::now))
        .execute(&mut conn)
        .await
        .context("Failed to bump conversation updated_at")?;
    Ok(row)
}

/// Set the conversation's title from the given content IF it's currently
/// NULL. Idempotent — subsequent calls do nothing.
pub async fn set_title_if_unset(
    pool: &DbPool,
    conv_id: Uuid,
    content: &str,
) -> Result<()> {
    let title = generate_title(content);
    if title.is_empty() {
        return Ok(());
    }
    let mut conn = pool.get().await.context("Failed to get DB connection")?;
    let _: usize = diesel::update(
        conversations::table
            .filter(conversations::id.eq(conv_id))
            .filter(conversations::title.is_null()),
    )
    .set((
        conversations::title.eq(&title),
        conversations::updated_at.eq(diesel::dsl::now),
    ))
    .execute(&mut conn)
    .await
    .context("Failed to set title")?;
    Ok(())
}

fn generate_title(content: &str) -> String {
    let one_line = content
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_string();
    if one_line.chars().count() <= AUTO_TITLE_MAX_LEN {
        return one_line;
    }
    let mut truncated: String = one_line.chars().take(AUTO_TITLE_MAX_LEN - 1).collect();
    truncated.push('…');
    truncated
}

/// Serialised JSON payload stored in `messages.content` for tool rows. The
/// shape is shared with the frontend (`src/lib/types.ts ToolCallEntry`).
///
/// `output` carries the full successful tool output JSON when available, so
/// we can faithfully replay history into rig as `ToolResult` content. For
/// rejected / errored calls, only `summary` is populated.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ToolMessageContent {
    pub args: serde_json::Value,
    pub requires_confirmation: bool,
    pub status: String,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub success: Option<bool>,
    /// The raw `T::Output` JSON for successful executions. Used when
    /// reconstructing tool history into rig `Message::tool_result`.
    #[serde(default)]
    pub output: Option<serde_json::Value>,
}

/// Look up a still-pending tool call by `tool_call_id`, ensuring the
/// conversation it lives in belongs to `user_id`. Returns `None` if the
/// call doesn't exist, isn't owned by the user, or has already been
/// resolved (status != 'pending_user').
pub struct PendingToolCall {
    pub conversation_id: Uuid,
    pub tool_name: String,
    pub args: serde_json::Value,
}

#[instrument(skip(pool))]
pub async fn find_pending_tool_for_user(
    pool: &DbPool,
    user_id: Uuid,
    call_id: &str,
) -> Result<Option<PendingToolCall>> {
    let mut conn = pool.get().await.context("Failed to get DB connection")?;
    let row: Option<(Uuid, String, String)> = messages::table
        .inner_join(conversations::table.on(conversations::id.eq(messages::conversation_id)))
        .filter(messages::tool_call_id.eq(call_id))
        .filter(messages::role.eq("tool"))
        .filter(conversations::user_id.eq(user_id))
        .filter(conversations::deleted_at.is_null())
        .select((
            messages::conversation_id,
            messages::content,
            messages::tool_name.assume_not_null(),
        ))
        .first(&mut conn)
        .await
        .optional()
        .context("Failed to look up pending tool call")?;

    let Some((conversation_id, content, tool_name)) = row else {
        return Ok(None);
    };

    let payload: ToolMessageContent = match serde_json::from_str(&content) {
        Ok(p) => p,
        Err(_) => return Ok(None),
    };
    if payload.status != "pending_user" {
        return Ok(None);
    }
    Ok(Some(PendingToolCall {
        conversation_id,
        tool_name,
        args: payload.args,
    }))
}

/// Insert (or update) a tool row keyed by `tool_call_id`. The first call
/// per call_id INSERTs with `status = pending_user|running`; subsequent
/// calls UPDATE the JSON content with the new status / summary. Returns
/// the row id.
pub async fn upsert_tool_call(
    pool: &DbPool,
    conv_id: Uuid,
    call_id: &str,
    tool_name: &str,
    payload: &ToolMessageContent,
) -> Result<Uuid> {
    let mut conn = pool.get().await.context("Failed to get DB connection")?;
    let content_json =
        serde_json::to_string(payload).context("Failed to serialise tool message content")?;

    // Check if a row already exists for this tool_call_id.
    let existing: Option<Uuid> = messages::table
        .filter(messages::conversation_id.eq(conv_id))
        .filter(messages::tool_call_id.eq(call_id))
        .select(messages::id)
        .first(&mut conn)
        .await
        .optional()
        .context("Failed to look up existing tool row")?;

    if let Some(id) = existing {
        diesel::update(messages::table.filter(messages::id.eq(id)))
            .set(messages::content.eq(&content_json))
            .execute(&mut conn)
            .await
            .context("Failed to update tool row")?;
        Ok(id)
    } else {
        let row: Message = diesel::insert_into(messages::table)
            .values(NewToolMessage {
                conversation_id: conv_id,
                role: "tool",
                content: &content_json,
                tool_name,
                tool_call_id: call_id,
            })
            .returning(Message::as_returning())
            .get_result(&mut conn)
            .await
            .context("Failed to insert tool row")?;
        let _: usize = diesel::update(conversations::table.filter(conversations::id.eq(conv_id)))
            .set(conversations::updated_at.eq(diesel::dsl::now))
            .execute(&mut conn)
            .await
            .context("Failed to bump conversation updated_at")?;
        Ok(row.id)
    }
}

/// Load the full message history for a conversation, oldest first.
pub async fn load_history(pool: &DbPool, conv_id: Uuid) -> Result<Vec<Message>> {
    let mut conn = pool.get().await.context("Failed to get DB connection")?;
    messages::table
        .filter(messages::conversation_id.eq(conv_id))
        .order(messages::created_at.asc())
        .select(Message::as_select())
        .load(&mut conn)
        .await
        .context("Failed to load message history")
}
