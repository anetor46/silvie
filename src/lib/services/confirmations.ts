/**
 * Tool response client. When the user clicks Approve / Reject (or, in
 * future, makes a multi-choice selection / fills a form), the frontend POSTs
 * the response here. The backend executes the deferred tool, persists the
 * result, and **resumes the conversation** — so this endpoint is itself an
 * SSE stream emitting the tool result + follow-up assistant text / tool
 * calls.
 *
 * Wire body (extensible — `kind` discriminates future input types):
 *   { call_id, response: { kind: "confirmation", approved: true, reason?: string } }
 */

import { getAccessToken } from '$lib/services/auth';
import type { ChatCallbacks, ChatOptions, StreamHandle } from '$lib/services/chat';

const BASE_URL =
  (import.meta.env.VITE_SILVIE_SERVER_URL as string | undefined) ?? 'http://localhost:8080';

/** A user response to a pending tool call. Extensible: add more kinds (
 *  `choice`, `survey`, `free_form`) as new widget types arrive. */
export type ToolResponse =
  | { kind: 'confirmation'; approved: boolean; reason?: string };

type ServerEvent =
  | { type: 'token'; text: string }
  | {
      type: 'tool_call';
      call_id: string;
      name: string;
      args: Record<string, unknown>;
      requires_confirmation: boolean;
    }
  | {
      type: 'tool_result';
      call_id: string;
      success: boolean;
      summary: string | null;
    }
  | { type: 'done' }
  | { type: 'error'; message: string };

export function postToolResponse(
  callId: string,
  response: ToolResponse,
  callbacks: ChatCallbacks,
  opts?: ChatOptions,
): StreamHandle {
  const controller = new AbortController();

  const done = (async () => {
    const token = await getAccessToken();
    if (!token) {
      throw new Error('Not signed in.');
    }

    const r = await fetch(`${BASE_URL}/chat/tool-responses`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Accept: 'text/event-stream',
        Authorization: `Bearer ${token}`,
      },
      body: JSON.stringify({
        call_id: callId,
        response,
        timezone: opts?.timezone ?? null,
        current_datetime: opts?.currentDatetime ?? null,
      }),
      signal: controller.signal,
    });

    if (r.status === 404) {
      throw new Error('This confirmation is no longer pending.');
    }
    if (!r.ok) {
      throw new Error(`server returned ${r.status} ${r.statusText}`);
    }
    if (!r.body) {
      throw new Error('server returned no body');
    }

    const reader = r.body.getReader();
    const decoder = new TextDecoder();
    let buffer = '';

    while (true) {
      const { done: finished, value } = await reader.read();
      if (finished) return;
      buffer += decoder.decode(value, { stream: true });

      let sep: number;
      while ((sep = buffer.indexOf('\n\n')) !== -1) {
        const rawFrame = buffer.slice(0, sep);
        buffer = buffer.slice(sep + 2);

        const event = parseFrame(rawFrame);
        if (!event) continue;

        switch (event.type) {
          case 'token':
            callbacks.onToken(event.text);
            break;
          case 'tool_call':
            callbacks.onToolCall({
              callId: event.call_id,
              name: event.name,
              args: event.args,
              requiresConfirmation: event.requires_confirmation,
            });
            break;
          case 'tool_result':
            callbacks.onToolResult({
              callId: event.call_id,
              success: event.success,
              summary: event.summary,
            });
            break;
          case 'done':
            return;
          case 'error':
            throw new Error(event.message);
        }
      }
    }
  })();

  return { done, cancel: () => controller.abort() };
}

function parseFrame(frame: string): ServerEvent | null {
  const dataLines: string[] = [];
  for (const line of frame.split('\n')) {
    if (line.startsWith('data:')) dataLines.push(line.slice(5).trim());
  }
  if (dataLines.length === 0) return null;
  try {
    return JSON.parse(dataLines.join('\n')) as ServerEvent;
  } catch {
    return null;
  }
}
