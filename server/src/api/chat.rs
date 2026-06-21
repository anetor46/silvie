//! `POST /chat` — start a new chat turn from a user message and stream the
//! response. The bulk of the streaming machinery lives in `chat_stream` so
//! the `/chat/tool-responses` endpoint can reuse it for continuations.

use std::sync::Arc;

use anyhow::Error;
use futures::StreamExt;
use poem::{
    handler,
    web::{
        sse::{Event, SSE},
        Data, Json,
    },
};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::{debug, error};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    db::DbPool,
    error::{ApiError, ApiResult, ResultOptionExt},
    llm::{
        history::db_rows_to_rig_history, ChatTurn, LlmClient, LocaleContext, StripePaymentRefs,
        ToolAuth,
    },
    repos::{
        conversations::{self, ToolMessageContent},
        integrations::{self, IntegrationsConfig, GOOGLE_PROVIDER},
        payments,
    },
    types::{ChatEvent, ChatRequest, SseEvent},
};

fn friendly_error_message(e: &Error) -> String {
    let msg = format!("{e:#}");
    if msg.contains("MaxTurnError") || msg.contains("max turn limit") {
        "I wasn't able to complete your request — it required more back-and-forth \
         than I'm configured to handle. Please try rephrasing or breaking it into \
         smaller steps."
            .to_string()
    } else if msg.contains("429") || msg.contains("quota") || msg.contains("rate") {
        "I'm temporarily unable to respond due to usage limits. Please try again in a moment."
            .to_string()
    } else {
        "I encountered an unexpected issue while processing your request. Please try again."
            .to_string()
    }
}

#[handler]
pub async fn chat_handler(
    auth: AuthUser,
    Data(client): Data<&Arc<LlmClient>>,
    Data(pool): Data<&DbPool>,
    Data(integ_cfg): Data<&Arc<IntegrationsConfig>>,
    Json(req): Json<ChatRequest>,
) -> ApiResult<SSE> {
    debug!(
        conversation_id = %req.conversation_id,
        content_len = req.content.len(),
        "/chat received"
    );

    // 1. Authorize.
    let convo = conversations::find_owned(pool, auth.user.id, req.conversation_id)
        .await
        .into_required()?;

    // 2. Persist the user message before doing anything else.
    conversations::insert_user_message(pool, convo.id, &req.content)
        .await
        .map_err(ApiError::from)?;

    // 3. Auto-title from the first user message (no-op once title is set).
    if let Err(e) = conversations::set_title_if_unset(pool, convo.id, &req.content).await {
        error!("failed to set conversation title: {e:#}");
    }

    // 4. Build per-request context.
    let locale = LocaleContext {
        timezone: req.timezone,
        current_datetime: req.current_datetime,
    };
    let tool_auth = build_tool_auth(pool, integ_cfg, auth.user.id).await;

    // 5. History excludes the user message we just inserted — pass it as the
    //    explicit prompt for this turn.
    let rows = conversations::load_history(pool, convo.id)
        .await
        .map_err(ApiError::from)?;
    let history_rows: Vec<_> = rows
        .iter()
        .filter(|r| !(r.role == "user" && r.content == req.content))
        .cloned()
        .collect();
    let history = db_rows_to_rig_history(&history_rows);

    let turn = ChatTurn {
        history,
        prompt: req.content,
        locale,
        tool_auth,
    };

    let events = run_turn(client.clone(), pool.clone(), convo.id, turn);
    Ok(SSE::new(sse_events(events)).keep_alive(std::time::Duration::from_secs(15)))
}

/// Wrap a stream of typed `SseEvent`s as poem SSE frames.
pub fn sse_events<S>(events: S) -> impl futures::Stream<Item = Event> + Send + 'static
where
    S: futures::Stream<Item = SseEvent> + Send + 'static,
{
    events.map(|ev| {
        let payload = serde_json::to_string(&ev).unwrap_or_else(|_| {
            r#"{"type":"error","message":"failed to encode event"}"#.to_string()
        });
        Event::message(payload)
    })
}

/// Shared streaming machinery used by both `/chat` and
/// `/chat/tool-responses`. Spawns a task that drives `LlmClient::stream`,
/// persists tool rows and assistant text as events arrive, and returns a
/// stream of typed `SseEvent`s the caller wraps as SSE (optionally
/// prepending its own events).
pub fn run_turn(
    client: Arc<LlmClient>,
    pool: DbPool,
    convo_id: Uuid,
    turn: ChatTurn,
) -> UnboundedReceiverStream<SseEvent> {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<SseEvent>();

    tokio::spawn(async move {
        let mut text_buffer = String::new();
        let mut client_disconnected = false;

        let send = |ev: SseEvent, disconnected: &mut bool| {
            if !*disconnected && tx.send(ev).is_err() {
                *disconnected = true;
            }
        };

        async fn flush_text(buffer: &mut String, pool: &DbPool, convo_id: Uuid) {
            if buffer.is_empty() {
                return;
            }
            let to_save = std::mem::take(buffer);
            if let Err(e) =
                conversations::insert_assistant_message(pool, convo_id, &to_save).await
            {
                error!("failed to persist assistant message segment: {e:#}");
            }
        }

        match client.stream(turn).await {
            Ok(mut stream) => {
                while let Some(item) = stream.next().await {
                    match item {
                        Ok(ChatEvent::Text(chunk)) if chunk.is_empty() => continue,
                        Ok(ChatEvent::Text(chunk)) => {
                            text_buffer.push_str(&chunk);
                            send(SseEvent::Token { text: chunk }, &mut client_disconnected);
                        }
                        Ok(ChatEvent::ToolCall {
                            call_id,
                            name,
                            args,
                            requires_confirmation,
                        }) => {
                            flush_text(&mut text_buffer, &pool, convo_id).await;

                            let status = if requires_confirmation {
                                "pending_user"
                            } else {
                                "running"
                            };
                            let payload = ToolMessageContent {
                                args: args.clone(),
                                requires_confirmation,
                                status: status.into(),
                                summary: None,
                                success: None,
                                output: None,
                            };
                            if let Err(e) = conversations::upsert_tool_call(
                                &pool, convo_id, &call_id, &name, &payload,
                            )
                            .await
                            {
                                error!("failed to persist tool_call row: {e:#}");
                            }
                            send(
                                SseEvent::ToolCall {
                                    call_id,
                                    name,
                                    args,
                                    requires_confirmation,
                                },
                                &mut client_disconnected,
                            );
                        }
                        Ok(ChatEvent::ToolResult {
                            call_id,
                            success,
                            summary,
                        }) => {
                            if let Err(e) = update_tool_status_in_db(
                                &pool,
                                convo_id,
                                &call_id,
                                success,
                                summary.clone(),
                                None,
                            )
                            .await
                            {
                                error!("failed to update tool_call row: {e:#}");
                            }
                            send(
                                SseEvent::ToolResult {
                                    call_id,
                                    success,
                                    summary,
                                },
                                &mut client_disconnected,
                            );
                        }
                        Err(e) => {
                            error!("model stream error: {e:#}");
                            let friendly = friendly_error_message(&e);
                            text_buffer.push_str(&friendly);
                            send(
                                SseEvent::Token { text: friendly },
                                &mut client_disconnected,
                            );
                            send(SseEvent::Done, &mut client_disconnected);
                            break;
                        }
                    }
                }
                send(SseEvent::Done, &mut client_disconnected);
            }
            Err(e) => {
                error!("failed to start chat stream: {e:#}");
                let fallback = "I encountered an issue getting ready to respond. Please try again."
                    .to_string();
                text_buffer.push_str(&fallback);
                send(
                    SseEvent::Token { text: fallback },
                    &mut client_disconnected,
                );
                send(SseEvent::Done, &mut client_disconnected);
            }
        }

        flush_text(&mut text_buffer, &pool, convo_id).await;
    });

    UnboundedReceiverStream::new(rx)
}

/// Look up Google + Stripe credentials for the user. Both are optional —
/// the LLM client gracefully skips tools whose deps are missing.
pub async fn build_tool_auth(
    pool: &DbPool,
    integ_cfg: &IntegrationsConfig,
    user_id: Uuid,
) -> ToolAuth {
    let google_access_token =
        integrations::fresh_access_token(pool, integ_cfg, user_id, GOOGLE_PROVIDER)
            .await
            .inspect_err(|e| error!("failed to fetch Google access token: {e:#}"))
            .ok()
            .flatten()
            .map(|t| t.access_token);

    let stripe_payment = payments::fetch_payment_method(pool, user_id)
        .await
        .ok()
        .flatten()
        .map(|view| StripePaymentRefs {
            customer_id: view.payment_method.stripe_customer_id,
            payment_method_id: view.payment_method.stripe_payment_method_id,
        });

    ToolAuth {
        google_access_token,
        stripe_payment,
    }
}

/// Merge a tool result event into the corresponding DB row's JSON payload.
/// Used by both the live stream loop and the deferred-execution path.
pub async fn update_tool_status_in_db(
    pool: &DbPool,
    conv_id: Uuid,
    call_id: &str,
    success: bool,
    summary: Option<String>,
    output: Option<serde_json::Value>,
) -> anyhow::Result<()> {
    use crate::schema::messages;
    use diesel::{ExpressionMethods, OptionalExtension, QueryDsl};
    use diesel_async::RunQueryDsl;

    let mut conn = pool.get().await?;
    let existing: Option<(String, Option<String>)> = messages::table
        .filter(messages::conversation_id.eq(conv_id))
        .filter(messages::tool_call_id.eq(call_id))
        .select((messages::content, messages::tool_name))
        .first(&mut conn)
        .await
        .optional()?;

    let (content, tool_name) = match existing {
        Some(v) => v,
        None => (
            "{}".to_string(),
            Some("unknown".to_string()),
        ),
    };

    let mut payload: ToolMessageContent =
        serde_json::from_str(&content).unwrap_or(ToolMessageContent {
            args: serde_json::Value::Null,
            requires_confirmation: false,
            status: "running".into(),
            summary: None,
            success: None,
            output: None,
        });
    payload.status = if success { "success".into() } else { "error".into() };
    payload.success = Some(success);
    if summary.is_some() {
        payload.summary = summary;
    }
    if output.is_some() {
        payload.output = output;
    }

    let tool_name = tool_name.as_deref().unwrap_or("unknown");
    conversations::upsert_tool_call(pool, conv_id, call_id, tool_name, &payload)
        .await
        .map(|_| ())
}
