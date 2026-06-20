import { invoke } from '@tauri-apps/api/core';

export interface AuthUser {
  sub: string;
  email: string;
  name: string;
  picture?: string | null;
}

function isTauri(): boolean {
  return !!(window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
}

function assertTauri(): void {
  if (!isTauri()) {
    throw new Error('This feature requires the Silvie desktop app.');
  }
}

// ── In-app flows (Resource Owner Password Grant) ─────────────────────────────

export async function login(email: string, password: string): Promise<AuthUser> {
  assertTauri();
  return await invoke<AuthUser>('auth0_login', { email, password });
}

export async function signup(
  email: string,
  password: string,
  name: string,
): Promise<AuthUser> {
  assertTauri();
  return await invoke<AuthUser>('auth0_signup', { email, password, name });
}

export async function requestPasswordReset(email: string): Promise<void> {
  assertTauri();
  await invoke('auth0_request_password_reset', { email });
}

// ── Browser-based fallback (PKCE / Universal Login) ─────────────────────────

/**
 * Open the system browser for OAuth login. If `connection` is provided
 * (e.g. "google-oauth2"), Auth0 jumps straight to that identity provider
 * instead of showing the Universal Login chooser.
 */
export async function loginBrowser(connection?: string): Promise<AuthUser> {
  assertTauri();
  return await invoke<AuthUser>('auth0_login_browser', { connection: connection ?? null });
}

// ── Session ─────────────────────────────────────────────────────────────────

export async function getCurrentUser(): Promise<AuthUser | null> {
  if (!isTauri()) return null;
  return await invoke<AuthUser | null>('auth0_get_user');
}

export async function logout(): Promise<void> {
  assertTauri();
  await invoke('auth0_logout');
}

export async function getAccessToken(): Promise<string | null> {
  if (!isTauri()) return null;
  return await invoke<string | null>('auth0_get_access_token');
}
