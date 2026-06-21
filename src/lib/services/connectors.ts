import { invoke } from '@tauri-apps/api/core';
import { getAccessToken } from '$lib/services/auth';

const BASE_URL =
  (import.meta.env.VITE_SILVIE_SERVER_URL as string | undefined) ?? 'http://localhost:8080';

// ── Tauri side: OAuth handshake only (no persistence) ───────────────────────

/**
 * Tokens returned from the Tauri-side OAuth handshake. The frontend forwards
 * them to the backend (`POST /users/me/integrations`) for storage + refresh.
 */
export interface OAuthTokens {
  access_token: string;
  refresh_token: string | null;
  /** Seconds until the access token expires. */
  expires_in: number | null;
  provider_account_id: string;
  email: string;
  scopes: string[];
}

function isTauri(): boolean {
  return !!(window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
}

function assertTauri(): void {
  if (!isTauri()) {
    throw new Error('This feature requires the Silvie desktop app.');
  }
}

/** Run the Google OAuth flow in the browser; resolves with the tokens. */
export async function startGoogleOAuth(): Promise<OAuthTokens> {
  assertTauri();
  return await invoke<OAuthTokens>('start_google_oauth');
}

// ── Backend (authenticated) CRUD for `integrations` ─────────────────────────

export interface IntegrationView {
  id: string;
  provider: string;
  provider_account_id: string;
  provider_account_email: string | null;
  scopes: string[];
  status: string;
  token_expiry: string | null;
  created_at: string;
  updated_at: string;
}

export interface UpsertIntegrationBody {
  provider: string;
  provider_account_id: string;
  provider_account_email: string | null;
  access_token: string;
  refresh_token: string | null;
  /** Seconds until expiry (server stamps the absolute expiry). */
  expires_in: number | null;
  scopes: string[];
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

export async function listIntegrations(): Promise<IntegrationView[]> {
  const resp = await authedFetch('/users/me/integrations', { method: 'GET' });
  if (!resp.ok) {
    throw new Error(await formatHttpError(resp, "Couldn't load connected accounts"));
  }
  return (await resp.json()) as IntegrationView[];
}

export async function saveIntegration(body: UpsertIntegrationBody): Promise<IntegrationView> {
  const resp = await authedFetch('/users/me/integrations', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });
  if (!resp.ok) {
    throw new Error(await formatHttpError(resp, "Couldn't save the connection"));
  }
  return (await resp.json()) as IntegrationView;
}

export async function deleteIntegration(id: string): Promise<void> {
  const resp = await authedFetch(`/users/me/integrations/${id}`, { method: 'DELETE' });
  if (!resp.ok && resp.status !== 404) {
    throw new Error(await formatHttpError(resp, "Couldn't disconnect"));
  }
}

interface AccessTokenResponse {
  access_token: string;
  expires_at: string | null;
}

/**
 * Fetch a fresh access token for a provider from the backend. The server
 * handles refresh-if-needed transparently. Returns null if the user has no
 * active integration for that provider.
 */
export async function getProviderAccessToken(provider: string): Promise<string | null> {
  const resp = await authedFetch(
    `/users/me/integrations/${provider}/access-token`,
    { method: 'GET' },
  );
  if (resp.status === 404) return null;
  if (!resp.ok) {
    console.error('[integrations.access-token]', resp.status, await resp.text().catch(() => ''));
    throw new Error("Couldn't read the connected account's access token.");
  }
  const json = (await resp.json()) as AccessTokenResponse;
  return json.access_token;
}

// ── Convenience: Google (Gmail + Calendar) ──────────────────────────────────

export const GOOGLE_PROVIDER = 'google';

export async function getGoogleAccessToken(): Promise<string | null> {
  if (!isTauri()) return null;
  return getProviderAccessToken(GOOGLE_PROVIDER);
}
