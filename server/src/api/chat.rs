//! `POST /chat` — SSE-streamed chat handler.

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

use crate::{
    auth::AuthUser,
    db::DbPool,
    error::{ApiError, ApiResult, ResultOptionExt},
    llm::{ChatTurn, LlmClient, LocaleContext, StripePaymentRefs, ToolAuth},
    repos::{
        conversations::{self, ToolMessageContent},
        integrations::{self, IntegrationsConfig, GOOGLE_PROVIDER},
        payments,
    },
    types::{ChatEvent, ChatMessage, ChatRequest, Role, SseEvent},
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

    // 1. Authorize: the user must own this conversation.
    let convo = conversations::find_owned(pool, auth.user.id, req.conversation_id)
        .await
        .into_required()?;

    // 2. Persist the user message immediately.
    conversations::insert_user_message(pool, convo.id, &req.content)
        .await
        .map_err(ApiError::from)?;

    // 3. Auto-title from the first user message (no-op once title is set).
    if let Err(e) = conversations::set_title_if_unset(pool, convo.id, &req.content).await {
        error!("failed to set conversation title: {e:#}");
    }

    // 4. Look up integration tokens + payment IDs server-side.
    let google_access_token = integrations::fresh_access_token(
        pool,
        integ_cfg,
        auth.user.id,
        GOOGLE_PROVIDER,
    )
    .await
    .inspect_err(|e| error!("failed to fetch Google access token: {e:#}"))
    .ok()
    .flatten()
    .map(|t| t.access_token);

    let stripe_payment = payments::fetch_payment_method(pool, auth.user.id)
        .await
        .ok()
        .flatten()
        .map(|view| StripePaymentRefs {
            customer_id: view.payment_method.stripe_customer_id,
            payment_method_id: view.payment_method.stripe_payment_method_id,
        });

    // 5. Load full history (includes the user message we just inserted).
    let history = conversations::load_history(pool, convo.id)
        .await
        .map_err(ApiError::from)?;
    let messages: Vec<ChatMessage> = history
        .into_iter()
        .filter_map(|m| {
            let role = match m.role.as_str() {
                "user" => Role::User,
                "assistant" => Role::Assistant,
                "system" => Role::System,
                // 'tool' rows are filtered out — they're frontend visualization
                // only; rig handles tool round-trips within a single agent stream.
                _ => return None,
            };
            Some(ChatMessage { role, content: m.content })
        })
        .collect();

    let client = client.clone();
    let pool_for_save = pool.clone();
    let convo_id = convo.id;
    let turn = ChatTurn {
        messages,
        locale: LocaleContext {
            timezone: req.timezone,
            current_datetime: req.current_datetime,
        },
        tool_auth: ToolAuth {
            google_access_token,
            stripe_payment,
        },
    };

    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<SseEvent>();

    tokio::spawn(async move {
        // Buffer of assistant text emitted between tool calls. Flushed
        // (as one row) when a tool call arrives, and again at stream end.
        let mut text_buffer = String::new();
        let mut client_disconnected = false;

        // Helper: emit an SSE event if the client is still connected.
        let send = |ev: SseEvent, disconnected: &mut bool| {
            if !*disconnected && tx.send(ev).is_err() {
                *disconnected = true;
            }
        };

        // Helper: persist the current text buffer as one assistant row.
        async fn flush_text(
            buffer: &mut String,
            pool: &DbPool,
            convo_id: uuid::Uuid,
        ) {
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
                            // Flush any accumulated assistant text first so
                            // row ordering reflects interleaving.
                            flush_text(&mut text_buffer, &pool_for_save, convo_id).await;

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
                            };
                            if let Err(e) = conversations::upsert_tool_call(
                                &pool_for_save,
                                convo_id,
                                &call_id,
                                &name,
                                &payload,
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
                            // The args/name/requires_confirmation fields stay
                            // as recorded by the prior ToolCall row — we
                            // reload, mutate, and re-save.
                            let updated = load_and_update_tool_status(
                                &pool_for_save,
                                convo_id,
                                &call_id,
                                success,
                                summary.clone(),
                            )
                            .await;
                            if let Err(e) = updated {
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
                                SseEvent::Token {
                                    text: friendly,
                                },
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
                let fallback =
                    "I encountered an issue getting ready to respond. Please try again."
                        .to_string();
                text_buffer.push_str(&fallback);
                send(
                    SseEvent::Token { text: fallback },
                    &mut client_disconnected,
                );
                send(SseEvent::Done, &mut client_disconnected);
            }
        }

        // Final flush of any trailing assistant text.
        flush_text(&mut text_buffer, &pool_for_save, convo_id).await;
    });

    let event_stream = UnboundedReceiverStream::new(rx).map(|ev| {
        let payload = serde_json::to_string(&ev).unwrap_or_else(|_| {
            r#"{"type":"error","message":"failed to encode event"}"#.to_string()
        });
        Event::message(payload)
    });

    Ok(SSE::new(event_stream).keep_alive(std::time::Duration::from_secs(15)))
}

/// Re-read the tool row's stored content, merge new result fields, save.
/// Done in the chat handler (rather than the repo) since the repo's upsert
/// helper is generic; this is the result-specific mutation pattern.
async fn load_and_update_tool_status(
    pool: &DbPool,
    conv_id: uuid::Uuid,
    call_id: &str,
    success: bool,
    summary: Option<String>,
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

    let Some((content, tool_name)) = existing else {
        // No prior ToolCall row was persisted — synthesise a minimal one.
        let payload = ToolMessageContent {
            args: serde_json::Value::Null,
            requires_confirmation: false,
            status: if success { "success".into() } else { "error".into() },
            summary,
            success: Some(success),
        };
        return conversations::upsert_tool_call(pool, conv_id, call_id, "unknown", &payload)
            .await
            .map(|_| ());
    };

    let mut payload: ToolMessageContent =
        serde_json::from_str(&content).unwrap_or(ToolMessageContent {
            args: serde_json::Value::Null,
            requires_confirmation: false,
            status: "running".into(),
            summary: None,
            success: None,
        });
    payload.status = if success { "success".into() } else { "error".into() };
    payload.success = Some(success);
    if summary.is_some() {
        payload.summary = summary;
    }

    let tool_name = tool_name.as_deref().unwrap_or("unknown");
    conversations::upsert_tool_call(pool, conv_id, call_id, tool_name, &payload)
        .await
        .map(|_| ())
}
