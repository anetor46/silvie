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
        conversations,
        integrations::{self, IntegrationsConfig, GOOGLE_CALENDAR_PROVIDER},
        payments,
    },
    types::{ChatMessage, ChatRequest, Role, SseEvent},
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

    // 2. Persist the user message immediately. Failure here is fatal — we
    //    don't want to call the LLM without recording the prompt.
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
        GOOGLE_CALENDAR_PROVIDER,
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
                _ => return None, // skip 'tool' rows — Gemini handles tools out-of-band
            };
            Some(ChatMessage { role, content: m.content })
        })
        .collect();

    // 6. Spawn the LLM stream. Collect chunks into `full_response` so we can
    //    persist the assistant message at the end — regardless of whether
    //    the client stays connected (closing fetch shouldn't lose the reply).
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
        let mut full_response = String::new();
        let mut client_disconnected = false;

        match client.stream(turn).await {
            Ok(mut stream) => {
                while let Some(item) = stream.next().await {
                    match item {
                        Ok(chunk) if chunk.is_empty() => continue,
                        Ok(chunk) => {
                            full_response.push_str(&chunk);
                            if !client_disconnected
                                && tx.send(SseEvent::Token { text: chunk }).is_err()
                            {
                                debug!("client disconnected mid-stream — continuing to collect for DB save");
                                client_disconnected = true;
                            }
                        }
                        Err(e) => {
                            error!("model stream error: {e:#}");
                            let friendly = friendly_error_message(&e);
                            full_response.push_str(&friendly);
                            if !client_disconnected {
                                let _ = tx.send(SseEvent::Token { text: friendly });
                                let _ = tx.send(SseEvent::Done);
                            }
                            break;
                        }
                    }
                }
                if !client_disconnected {
                    let _ = tx.send(SseEvent::Done);
                }
            }
            Err(e) => {
                error!("failed to start chat stream: {e:#}");
                let fallback =
                    "I encountered an issue getting ready to respond. Please try again."
                        .to_string();
                full_response.push_str(&fallback);
                let _ = tx.send(SseEvent::Token { text: fallback });
                let _ = tx.send(SseEvent::Done);
            }
        }

        if !full_response.is_empty() {
            if let Err(e) =
                conversations::insert_assistant_message(&pool_for_save, convo_id, &full_response)
                    .await
            {
                error!("failed to persist assistant message: {e:#}");
            }
        }
    });

    let event_stream = UnboundedReceiverStream::new(rx).map(|ev| {
        let payload = serde_json::to_string(&ev).unwrap_or_else(|_| {
            r#"{"type":"error","message":"failed to encode event"}"#.to_string()
        });
        Event::message(payload)
    });

    Ok(SSE::new(event_stream).keep_alive(std::time::Duration::from_secs(15)))
}
