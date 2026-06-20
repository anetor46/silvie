import { invoke } from '@tauri-apps/api/core';

export interface StoredProfile {
  first_name: string;
  last_name: string;
  email: string;
}

function isTauri(): boolean {
  return !!(window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
}

export async function getStoredProfile(): Promise<StoredProfile | null> {
  if (!isTauri()) return null;
  return await invoke<StoredProfile | null>('get_profile');
}

export async function saveProfile(p: StoredProfile): Promise<void> {
  if (!isTauri()) throw new Error('This feature requires the Silvie desktop app.');
  await invoke('store_profile', { data: p });
}
