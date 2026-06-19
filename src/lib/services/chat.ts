/**
 * Streaming chat client. Talks to the local Rust backend (server crate) over
 * Server-Sent Events. Uses fetch + ReadableStream (not EventSource) so we can
 * POST the message history in the body.
 *
 * Wire format (one frame per `data:` line, JSON-encoded payload):
 *   { "type": "token", "text": "<chunk>" }
 *   { "type": "done" }
 *   { "type": "error", "message": "<msg>" }
 */

export type Role = 'system' | 'user' | 'assistant';

export interface ChatMessage {
  role: Role;
  content: string;
}

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
  googleAccessToken?: string | null;
  timezone?: string;        // IANA name, e.g. "Europe/Paris"
  currentDatetime?: string; // ISO 8601 with local offset, e.g. "2026-06-19T14:32:00+02:00"
}

export function streamChat(
  messages: ChatMessage[],
  onToken: (text: string) => void,
  opts?: ChatOptions,
): StreamHandle {
  const controller = new AbortController();

  const done = (async () => {
    const response = await fetch(`${BASE_URL}/chat`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', Accept: 'text/event-stream' },
      body: JSON.stringify({
        messages,
        google_access_token: opts?.googleAccessToken ?? null,
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
