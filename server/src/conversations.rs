//! Conversation + message persistence.
//!
//! Public CRUD operates on conversations the JWT-validated user owns. The
//! `insert_user_message` / `insert_assistant_message` / `load_history`
//! helpers are used by the chat handler — they persist the back-and-forth as
//! the streaming response unfolds.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use diesel::{
    ExpressionMethods, Insertable, OptionalExtension, QueryDsl, Queryable, Selectable,
    SelectableHelper,
};
use diesel_async::RunQueryDsl;
use poem::{
    handler,
    http::StatusCode,
    web::{Data, Json, Path},
};
use serde::{Deserialize, Serialize};
use tracing::{error, info, instrument};
use uuid::Uuid;

use crate::{
    auth::Principal,
    db::DbPool,
    schema::{conversations, messages},
    users,
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

#[instrument(skip(pool), fields(sub_len = sub.len()))]
pub async fn list_for_user(pool: &DbPool, sub: &str) -> Result<Vec<Conversation>> {
    let user = match users::find_by_sub(pool, sub).await? {
        Some(u) => u,
        None => return Ok(Vec::new()),
    };
    let mut conn = pool.get().await.context("Failed to get DB connection")?;
    conversations::table
        .filter(conversations::user_id.eq(user.id))
        .filter(conversations::deleted_at.is_null())
        .order(conversations::updated_at.desc())
        .select(Conversation::as_select())
        .load(&mut conn)
        .await
        .context("Failed to list conversations")
}

#[instrument(skip(pool), fields(sub_len = sub.len()))]
pub async fn create_for_user(pool: &DbPool, sub: &str) -> Result<Option<Conversation>> {
    let user = match users::find_by_sub(pool, sub).await? {
        Some(u) => u,
        None => return Ok(None),
    };
    let mut conn = pool.get().await.context("Failed to get DB connection")?;
    let row: Conversation = diesel::insert_into(conversations::table)
        .values(NewConversation { user_id: user.id })
        .returning(Conversation::as_returning())
        .get_result(&mut conn)
        .await
        .context("Failed to insert conversation")?;
    info!(conversation_id = %row.id, "conversation created");
    Ok(Some(row))
}

/// Look up a conversation owned by the given user. Returns `None` if either
/// the user or the conversation doesn't exist (or isn't owned by them).
pub async fn find_owned(
    pool: &DbPool,
    sub: &str,
    conv_id: Uuid,
) -> Result<Option<Conversation>> {
    let user = match users::find_by_sub(pool, sub).await? {
        Some(u) => u,
        None => return Ok(None),
    };
    let mut conn = pool.get().await.context("Failed to get DB connection")?;
    conversations::table
        .filter(conversations::id.eq(conv_id))
        .filter(conversations::user_id.eq(user.id))
        .filter(conversations::deleted_at.is_null())
        .select(Conversation::as_select())
        .first(&mut conn)
        .await
        .optional()
        .context("Failed to look up conversation")
}

#[instrument(skip(pool), fields(sub_len = sub.len()))]
pub async fn fetch_with_messages(
    pool: &DbPool,
    sub: &str,
    conv_id: Uuid,
) -> Result<Option<ConversationWithMessages>> {
    let convo = match find_owned(pool, sub, conv_id).await? {
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

#[instrument(skip(pool), fields(sub_len = sub.len()))]
pub async fn soft_delete(pool: &DbPool, sub: &str, conv_id: Uuid) -> Result<bool> {
    let user = match users::find_by_sub(pool, sub).await? {
        Some(u) => u,
        None => return Ok(false),
    };
    let mut conn = pool.get().await.context("Failed to get DB connection")?;
    let n: usize = diesel::update(
        conversations::table
            .filter(conversations::id.eq(conv_id))
            .filter(conversations::user_id.eq(user.id))
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

#[instrument(skip(pool, title), fields(sub_len = sub.len()))]
pub async fn update_title(
    pool: &DbPool,
    sub: &str,
    conv_id: Uuid,
    title: Option<String>,
) -> Result<Option<Conversation>> {
    let user = match users::find_by_sub(pool, sub).await? {
        Some(u) => u,
        None => return Ok(None),
    };
    let mut conn = pool.get().await.context("Failed to get DB connection")?;
    let n: usize = diesel::update(
        conversations::table
            .filter(conversations::id.eq(conv_id))
            .filter(conversations::user_id.eq(user.id))
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

// ── HTTP handlers ───────────────────────────────────────────────────────────

#[handler]
pub async fn list_conversations_handler(
    principal: Principal,
    Data(pool): Data<&DbPool>,
) -> poem::Result<Json<Vec<Conversation>>> {
    list_for_user(pool, &principal.sub).await.map(Json).map_err(|e| {
        error!("conversations list failed: {e:#}");
        poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
    })
}

#[handler]
pub async fn create_conversation_handler(
    principal: Principal,
    Data(pool): Data<&DbPool>,
) -> poem::Result<Json<Conversation>> {
    let convo = create_for_user(pool, &principal.sub).await.map_err(|e| {
        error!("conversation create failed: {e:#}");
        poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
    })?;
    convo
        .map(Json)
        .ok_or_else(|| poem::Error::from_status(StatusCode::NOT_FOUND))
}

#[handler]
pub async fn get_conversation_handler(
    principal: Principal,
    Data(pool): Data<&DbPool>,
    Path(id): Path<Uuid>,
) -> poem::Result<Json<ConversationWithMessages>> {
    let row = fetch_with_messages(pool, &principal.sub, id)
        .await
        .map_err(|e| {
            error!("conversation fetch failed: {e:#}");
            poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
        })?;
    row.map(Json)
        .ok_or_else(|| poem::Error::from_status(StatusCode::NOT_FOUND))
}

#[handler]
pub async fn update_conversation_handler(
    principal: Principal,
    Data(pool): Data<&DbPool>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateConversationRequest>,
) -> poem::Result<Json<Conversation>> {
    let row = update_title(pool, &principal.sub, id, req.title)
        .await
        .map_err(|e| {
            error!("conversation update failed: {e:#}");
            poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
        })?;
    row.map(Json)
        .ok_or_else(|| poem::Error::from_status(StatusCode::NOT_FOUND))
}

#[handler]
pub async fn delete_conversation_handler(
    principal: Principal,
    Data(pool): Data<&DbPool>,
    Path(id): Path<Uuid>,
) -> poem::Result<StatusCode> {
    let removed = soft_delete(pool, &principal.sub, id).await.map_err(|e| {
        error!("conversation delete failed: {e:#}");
        poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
    })?;
    Ok(if removed {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    })
}
