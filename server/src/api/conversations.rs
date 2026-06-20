//! HTTP layer for conversation CRUD.

use poem::{
    handler,
    http::StatusCode,
    web::{Data, Json, Path},
};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    db::DbPool,
    error::{ApiResult, ResultOptionExt},
    repos::conversations::{
        self, Conversation, ConversationWithMessages, UpdateConversationRequest,
    },
};

#[handler]
pub async fn list_conversations_handler(
    auth: AuthUser,
    Data(pool): Data<&DbPool>,
) -> ApiResult<Json<Vec<Conversation>>> {
    Ok(Json(conversations::list_for_user(pool, auth.user.id).await?))
}

#[handler]
pub async fn create_conversation_handler(
    auth: AuthUser,
    Data(pool): Data<&DbPool>,
) -> ApiResult<Json<Conversation>> {
    Ok(Json(
        conversations::create_for_user(pool, auth.user.id).await?,
    ))
}

#[handler]
pub async fn get_conversation_handler(
    auth: AuthUser,
    Data(pool): Data<&DbPool>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ConversationWithMessages>> {
    let row = conversations::fetch_with_messages(pool, auth.user.id, id)
        .await
        .into_required()?;
    Ok(Json(row))
}

#[handler]
pub async fn update_conversation_handler(
    auth: AuthUser,
    Data(pool): Data<&DbPool>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateConversationRequest>,
) -> ApiResult<Json<Conversation>> {
    let row = conversations::update_title(pool, auth.user.id, id, req.title)
        .await
        .into_required()?;
    Ok(Json(row))
}

#[handler]
pub async fn delete_conversation_handler(
    auth: AuthUser,
    Data(pool): Data<&DbPool>,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let removed = conversations::soft_delete(pool, auth.user.id, id).await?;
    Ok(if removed {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    })
}
