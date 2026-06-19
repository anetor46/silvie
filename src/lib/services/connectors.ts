import { invoke } from '@tauri-apps/api/core';

export interface ConnectedAccount {
  email: string;
}

function assertTauri(): void {
  if (!(window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__) {
    throw new Error('This feature requires the Silvie desktop app.');
  }
}

export async function startGoogleOAuth(): Promise<ConnectedAccount> {
  assertTauri();
  return await invoke<ConnectedAccount>('start_google_oauth');
}

export async function getGoogleCalendarAccount(): Promise<ConnectedAccount | null> {
  if (!(window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__) {
    return null;
  }
  return await invoke<ConnectedAccount | null>('get_google_calendar_account');
}

export async function disconnectGoogleCalendar(): Promise<void> {
  assertTauri();
  await invoke('disconnect_google_calendar');
}

export async function getGoogleAccessToken(): Promise<string | null> {
  if (!(window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__) {
    return null;
  }
  return await invoke<string | null>('get_google_access_token');
}
