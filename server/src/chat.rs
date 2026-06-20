use std::sync::Arc;

use anyhow::Error;
use futures::StreamExt;
use poem::{
    handler,
    web::{
        sse::{Event, SSE},
        Data, Json,
    },
    IntoResponse,
};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::{debug, error};

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

use crate::{
    db::DbPool,
    llm::LlmClient,
    types::{ChatRequest, SseEvent},
};

#[handler]
pub async fn chat_handler(
    Data(client): Data<&Arc<LlmClient>>,
    Data(pool): Data<&DbPool>,
    Json(req): Json<ChatRequest>,
) -> impl IntoResponse {
    debug!("/chat received {} messages", req.messages.len());

    let client = client.clone();
    let pool = pool.clone();

    // We need to bridge "model stream produces chunks" → "SSE events".
    // Spawn a task that pulls from the model stream and pushes typed events into
    // an mpsc channel. The SSE response reads from the channel.
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<SseEvent>();

    tokio::spawn(async move {
        match client.stream_chat(req.messages, req.google_access_token, req.timezone, req.current_datetime, req.stripe_customer_id, req.stripe_payment_method_id, pool).await {
            Ok(mut stream) => {
                while let Some(item) = stream.next().await {
                    match item {
                        Ok(chunk) if chunk.is_empty() => continue,
                        Ok(chunk) => {
                            if tx.send(SseEvent::Token { text: chunk }).is_err() {
                                debug!("client disconnected mid-stream");
                                return;
                            }
                        }
                        Err(e) => {
                            error!("model stream error: {e:#}");
                            let _ = tx.send(SseEvent::Token { text: friendly_error_message(&e) });
                            let _ = tx.send(SseEvent::Done);
                            return;
                        }
                    }
                }
                let _ = tx.send(SseEvent::Done);
            }
            Err(e) => {
                error!("failed to start chat stream: {e:#}");
                let _ = tx.send(SseEvent::Token {
                    text: "I encountered an issue getting ready to respond. Please try again."
                        .to_string(),
                });
                let _ = tx.send(SseEvent::Done);
            }
        }
    });

    let event_stream = UnboundedReceiverStream::new(rx).map(|ev| {
        let payload = serde_json::to_string(&ev).unwrap_or_else(|_| {
            r#"{"type":"error","message":"failed to encode event"}"#.to_string()
        });
        Event::message(payload)
    });

    SSE::new(event_stream).keep_alive(std::time::Duration::from_secs(15))
}
