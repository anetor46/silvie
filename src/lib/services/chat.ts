/**
 * Streaming chat client. Talks to the local Rust backend (server crate) over
 * Server-Sent Events. Uses fetch + ReadableStream (not EventSource) so we can
 * POST the request body AND attach an Authorization header.
 *
 * Wire format (one frame per `data:` line, JSON-encoded payload):
 *   { "type": "token", "text": "<chunk>" }
 *   { "type": "done" }
 *   { "type": "error", "message": "<msg>" }
 *
 * Server is authoritative for conversation history + integration tokens — the
 * client just sends the conversation id, the new user turn, and per-request
 * locale context (timezone / current datetime).
 */

import { getAccessToken } from '$lib/services/auth';

export interface StreamHandle {
  /** Resolves when the stream completes successfully. Rejects on error or abort. */
  done: Promise<void>;
  /** Cancels the in-flight request. */
  cancel: () => void;
}

type ServerEvent =
  | { type: 'token'; text: string }
  | { type: 'done' }
  | { type: 'error'; message: string };

const BASE_URL =
  (import.meta.env.VITE_SILVIE_SERVER_URL as string | undefined) ?? 'http://localhost:8080';

export interface ChatOptions {
  timezone?: string;        // IANA name, e.g. "Europe/Paris"
  currentDatetime?: string; // ISO 8601 with local offset
}

export function streamChat(
  conversationId: string,
  content: string,
  onToken: (text: string) => void,
  opts?: ChatOptions,
): StreamHandle {
  const controller = new AbortController();

  const done = (async () => {
    const token = await getAccessToken();
    if (!token) {
      throw new Error('Not signed in.');
    }

    const response = await fetch(`${BASE_URL}/chat`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Accept: 'text/event-stream',
        Authorization: `Bearer ${token}`,
      },
      body: JSON.stringify({
        conversation_id: conversationId,
        content,
        timezone: opts?.timezone ?? null,
        current_datetime: opts?.currentDatetime ?? null,
      }),
      signal: controller.signal,
    });

    if (!response.ok) {
      throw new Error(`server returned ${response.status} ${response.statusText}`);
    }
    if (!response.body) {
      throw new Error('server returned no body');
    }

    const reader = response.body.getReader();
    const decoder = new TextDecoder();
    let buffer = '';

    while (true) {
      const { done: finished, value } = await reader.read();
      if (finished) return;

      buffer += decoder.decode(value, { stream: true });

      // SSE frames are separated by a blank line ("\n\n").
      let sep: number;
      while ((sep = buffer.indexOf('\n\n')) !== -1) {
        const rawFrame = buffer.slice(0, sep);
        buffer = buffer.slice(sep + 2);

        const event = parseFrame(rawFrame);
        if (!event) continue;

        if (event.type === 'token') {
          onToken(event.text);
        } else if (event.type === 'done') {
          return;
        } else if (event.type === 'error') {
          throw new Error(event.message);
        }
      }
    }
  })();

  return {
    done,
    cancel: () => controller.abort(),
  };
}

/** Parses one SSE frame. Returns null if the frame is a comment or has no data. */
function parseFrame(frame: string): ServerEvent | null {
  const dataLines: string[] = [];
  for (const line of frame.split('\n')) {
    // Comments (lines starting with ":") and other field names (id:, event:, retry:) are ignored.
    if (line.startsWith('data:')) {
      dataLines.push(line.slice(5).trim());
    }
  }
  if (dataLines.length === 0) return null;
  const payload = dataLines.join('\n');
  try {
    return JSON.parse(payload) as ServerEvent;
  } catch {
    return null;
  }
}
