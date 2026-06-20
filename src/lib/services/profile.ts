import { invoke } from '@tauri-apps/api/core';

export interface StoredProfile {
  // Core — set during onboarding
  first_name: string;
  last_name: string;
  email: string;

  // Contact
  phone?: string | null;

  // Travel documents
  nationality?: string | null;
  passport_number?: string | null;
  /** ISO date string "YYYY-MM-DD" */
  passport_expiry?: string | null;
  country_of_residence?: string | null;

  // Home address (for hotel billing)
  address_line1?: string | null;
  address_city?: string | null;
  address_state?: string | null;
  address_postal_code?: string | null;
  address_country?: string | null;
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
