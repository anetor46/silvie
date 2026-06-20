import { getAccessToken } from '$lib/services/auth';

const BASE_URL =
  (import.meta.env.VITE_SILVIE_SERVER_URL as string | undefined) ?? 'http://localhost:8080';

export interface User {
  id: string;
  auth0_sub: string;
  email: string;
  name: string;
  created_at: string;
  updated_at: string;
  deleted_at: string | null;
}

/** Result type for /users/me — null when the user has no DB row yet. */
export type CurrentUser = User | null;

/**
 * Fetch with `Authorization: Bearer <access_token>`. On 401, the access token
 * may have expired between checks — call `getAccessToken` again (which
 * triggers Tauri-side refresh if needed) and retry once.
 */
async function authedFetch(path: string, init: RequestInit = {}): Promise<Response> {
  const url = `${BASE_URL}${path}`;

  const send = async () => {
    const token = await getAccessToken();
    if (!token) throw new Error('Not signed in.');
    return fetch(url, {
      ...init,
      headers: {
        ...(init.headers ?? {}),
        Authorization: `Bearer ${token}`,
      },
    });
  };

  let resp = await send();
  if (resp.status === 401) {
    resp = await send();
  }
  return resp;
}

/** GET /users/me — returns null on 404 (no DB row yet). */
export async function fetchCurrentUser(): Promise<CurrentUser> {
  const resp = await authedFetch('/users/me', { method: 'GET' });
  if (resp.status === 404) return null;
  if (!resp.ok) {
    throw new Error(await formatHttpError(resp, 'Failed to load your account'));
  }
  return (await resp.json()) as User;
}

/** POST /users — find-or-create. Returns the existing or newly-created row. */
export async function syncUser(body: { email: string; name: string }): Promise<User> {
  const resp = await authedFetch('/users', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });
  if (!resp.ok) {
    throw new Error(await formatHttpError(resp, "Couldn't sync your account"));
  }
  return (await resp.json()) as User;
}

async function formatHttpError(resp: Response, fallback: string): Promise<string> {
  // Backend currently returns plain status codes for error paths; surface the
  // friendliest message we can without leaking internal detail.
  const text = await resp.text().catch(() => '');
  if (text && text.length < 200) return `${fallback}: ${text}`;
  return `${fallback} (HTTP ${resp.status})`;
}
