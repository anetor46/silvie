//! `POST /chat` — start a new chat turn from a user message and stream the
//! response. The bulk of the streaming machinery lives in `chat_stream` so
//! the `/chat/tool-responses` endpoint can reuse it for continuations.

use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use anyhow::Error;
use futures::{Stream, StreamExt};
use poem::{
    handler,
    web::{
        sse::{Event, SSE},
        Data, Json,
    },
};
use tokio::sync::Notify;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    config::Config,
    db::DbPool,
    error::{ApiError, ApiResult, ResultOptionExt},
    llm::{
        history::db_rows_to_rig_history, ChatTurn, LlmClient, LocaleContext, StripePaymentRefs,
        ToolAuth,
    },
    repos::{
        conversations::{self, ToolMessageContent},
        integrations::{self, IntegrationsConfig, GOOGLE_PROVIDER, OUTLOOK_PROVIDER},
        payments,
    },
    tools::travelport::{TravelportClient, TravelportClientCreds, TravelportEnv},
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
    Data(config): Data<&Arc<Config>>,
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

    // 2. Persist the user message before doing anything else. Keep the
    //    inserted row's id so we can exclude it from the history we pass
    //    to the agent — filtering by content would drop EVERY prior user
    //    message with the same text when the user repeats themselves.
    let new_user_row = conversations::insert_user_message(pool, convo.id, &req.content)
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
    let tool_auth = build_tool_auth(pool, integ_cfg, config, auth.user.id, Some(convo.id)).await;

    // 5. History excludes the just-inserted user row (we pass it as the
    //    explicit prompt instead). Filter by ID so repeat questions don't
    //    accidentally erase older identical user messages from history.
    let rows = conversations::load_history(pool, convo.id)
        .await
        .map_err(ApiError::from)?;
    let history_rows: Vec<_> = rows
        .into_iter()
        .filter(|r| r.id != new_user_row.id)
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

/// Stream wrapper that fires a `Notify` when dropped. The spawned task on
/// the other side `select!`s every iteration against `cancel.notified()`;
/// when the consumer (poem's SSE response) gets dropped — which happens on
/// client disconnect OR explicit cancel — this wrapper drops, notify fires,
/// the task breaks out of its loop, drops the rig stream, and that cascades
/// down through `reqwest` to cancel the in-flight Gemini HTTP call.
///
/// This is the canonical "task lifetime tied to stream lifetime" pattern
/// for streaming endpoints — copy it for any future SSE handler.
struct CancelOnDrop<S> {
    inner: S,
    cancel: Arc<Notify>,
}

impl<S: Stream + Unpin> Stream for CancelOnDrop<S> {
    type Item = S::Item;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.inner).poll_next(cx)
    }
}

impl<S> Drop for CancelOnDrop<S> {
    fn drop(&mut self) {
        self.cancel.notify_one();
    }
}

/// Shared streaming machinery used by both `/chat` and
/// `/chat/tool-responses`. Spawns a task that drives `LlmClient::stream`,
/// persists tool rows and assistant text as events arrive, and returns a
/// stream of typed `SseEvent`s wrapped in `CancelOnDrop` so dropping the
/// returned stream aborts the underlying work.
///
/// Cancellation semantics: cooperative — the select happens BETWEEN events,
/// so an in-flight DB write completes before the loop notices the cancel.
/// No partial DB rows; in-flight rig HTTP calls are aborted via future drop.
pub fn run_turn(
    client: Arc<LlmClient>,
    pool: DbPool,
    convo_id: Uuid,
    turn: ChatTurn,
) -> impl Stream<Item = SseEvent> + Send + 'static {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<SseEvent>();
    let cancel = Arc::new(Notify::new());
    let cancel_for_task = cancel.clone();

    tokio::spawn(async move {
        let cancel = cancel_for_task;
        let mut text_buffer = String::new();
        let mut client_disconnected = false;
        let mut cancelled = false;

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
                loop {
                    let item = tokio::select! {
                        biased;
                        _ = cancel.notified() => {
                            cancelled = true;
                            break;
                        }
                        item = stream.next() => item,
                    };
                    let Some(item) = item else { break };
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
                            output,
                        }) => {
                            if let Err(e) = update_tool_status_in_db(
                                &pool,
                                convo_id,
                                &call_id,
                                success,
                                summary.clone(),
                                output.clone(),
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
                                    output,
                                },
                                &mut client_disconnected,
                            );
                        }
                        Err(e) => {
                            error!("model stream error: {e:#}");
                            let friendly = friendly_error_message(&e);
                            text_buffer.push_str(&friendly);
                            send(SseEvent::Token { text: friendly }, &mut client_disconnected);
                            send(SseEvent::Done, &mut client_disconnected);
                            break;
                        }
                    }
                }
                if !cancelled {
                    send(SseEvent::Done, &mut client_disconnected);
                }
            }
            Err(e) => {
                error!("failed to start chat stream: {e:#}");
                let fallback = "I encountered an issue getting ready to respond. Please try again."
                    .to_string();
                text_buffer.push_str(&fallback);
                send(SseEvent::Token { text: fallback }, &mut client_disconnected);
                send(SseEvent::Done, &mut client_disconnected);
            }
        }

        // Persist any trailing assistant text — partial responses are
        // still useful for context even after a cancel.
        flush_text(&mut text_buffer, &pool, convo_id).await;

        // Sweep tool rows that were mid-execution. Runs after cancel,
        // error, and normal completion — idempotent (no-op if nothing was
        // running). Keeps history reconstruction sane on the next turn.
        match conversations::mark_running_tools_cancelled(&pool, convo_id).await {
            Ok(n) if n > 0 => info!(
                conversation_id = %convo_id,
                cancelled_tool_rows = n,
                "marked tool rows cancelled after stream ended"
            ),
            Err(e) => error!("failed to sweep cancelled tool rows: {e:#}"),
            _ => {}
        }

        if cancelled {
            info!(conversation_id = %convo_id, "chat stream cancelled by client");
        }
    });

    CancelOnDrop {
        inner: UnboundedReceiverStream::new(rx),
        cancel,
    }
}

/// Look up Google + Stripe credentials for the user. Each integration is
/// optional — the LLM client gracefully skips tools whose deps are missing.
pub async fn build_tool_auth(
    pool: &DbPool,
    integ_cfg: &IntegrationsConfig,
    config: &Config,
    user_id: Uuid,
    conversation_id: Option<Uuid>,
) -> ToolAuth {
    let google_access_token =
        integrations::fresh_access_token(pool, integ_cfg, user_id, GOOGLE_PROVIDER)
            .await
            .inspect_err(|e| error!("failed to fetch Google access token: {e:#}"))
            .ok()
            .flatten()
            .map(|t| t.access_token);

    let outlook_access_token =
        integrations::fresh_access_token(pool, integ_cfg, user_id, OUTLOOK_PROVIDER)
            .await
            .inspect_err(|e| error!("failed to fetch Outlook access token: {e:#}"))
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

    let travelport = config.travelport.as_ref().map(|tp| {
        TravelportClient::new(TravelportClientCreds {
            client_id: tp.client_id.clone(),
            client_secret: tp.client_secret.clone(),
            username: tp.username.clone(),
            password: tp.password.clone(),
            env: TravelportEnv::parse(&tp.env),
            access_group: tp.access_group.clone(),
        })
    });

    ToolAuth {
        user_id: Some(user_id),
        conversation_id,
        google_access_token,
        outlook_access_token,
        stripe_payment,
        travelport,
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

    let (content, tool_name) = existing.unwrap_or_else(|| (
        "{}".to_string(),
        Some("unknown".to_string()),
    ));

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
