import { getAccessToken } from '$lib/services/auth';

const BASE_URL =
  (import.meta.env.VITE_SILVIE_SERVER_URL as string | undefined) ?? 'http://localhost:8080';

export interface Conversation {
  id: string;
  user_id: string;
  organization_id: string | null;
  title: string | null;
  model: string | null;
  created_at: string;
  updated_at: string;
  deleted_at: string | null;
}

export type MessageRole = 'user' | 'assistant' | 'system' | 'tool';

export interface Message {
  id: string;
  conversation_id: string;
  role: MessageRole;
  content: string;
  tool_name: string | null;
  tool_call_id: string | null;
  prompt_tokens: number | null;
  completion_tokens: number | null;
  latency_ms: number | null;
  created_at: string;
}

export interface ConversationWithMessages extends Conversation {
  messages: Message[];
}

async function authedFetch(path: string, init: RequestInit = {}): Promise<Response> {
  const send = async () => {
    const token = await getAccessToken();
    if (!token) throw new Error('Not signed in.');
    return fetch(`${BASE_URL}${path}`, {
      ...init,
      headers: {
        ...(init.headers ?? {}),
        Authorization: `Bearer ${token}`,
      },
    });
  };
  let resp = await send();
  if (resp.status === 401) resp = await send();
  return resp;
}

async function formatHttpError(resp: Response, fallback: string): Promise<string> {
  const text = await resp.text().catch(() => '');
  if (text && text.length < 200) return `${fallback}: ${text}`;
  return `${fallback} (HTTP ${resp.status})`;
}

export async function listConversations(): Promise<Conversation[]> {
  const resp = await authedFetch('/users/me/conversations', { method: 'GET' });
  if (!resp.ok) throw new Error(await formatHttpError(resp, "Couldn't load conversations"));
  return (await resp.json()) as Conversation[];
}

export async function createConversation(): Promise<Conversation> {
  const resp = await authedFetch('/users/me/conversations', { method: 'POST' });
  if (!resp.ok) throw new Error(await formatHttpError(resp, "Couldn't create conversation"));
  return (await resp.json()) as Conversation;
}

export async function getConversation(id: string): Promise<ConversationWithMessages | null> {
  const resp = await authedFetch(`/users/me/conversations/${id}`, { method: 'GET' });
  if (resp.status === 404) return null;
  if (!resp.ok) throw new Error(await formatHttpError(resp, "Couldn't load conversation"));
  return (await resp.json()) as ConversationWithMessages;
}

export async function updateConversationTitle(id: string, title: string | null): Promise<Conversation> {
  const resp = await authedFetch(`/users/me/conversations/${id}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ title }),
  });
  if (!resp.ok) throw new Error(await formatHttpError(resp, "Couldn't rename conversation"));
  return (await resp.json()) as Conversation;
}

export async function deleteConversation(id: string): Promise<void> {
  const resp = await authedFetch(`/users/me/conversations/${id}`, { method: 'DELETE' });
  if (!resp.ok && resp.status !== 404) {
    throw new Error(await formatHttpError(resp, "Couldn't delete conversation"));
  }
}
