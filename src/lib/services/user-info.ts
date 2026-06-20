import { getAccessToken } from '$lib/services/auth';

const BASE_URL =
  (import.meta.env.VITE_SILVIE_SERVER_URL as string | undefined) ?? 'http://localhost:8080';

// ── Wire types (match the Rust shapes 1:1) ──────────────────────────────────

export interface UserProfile {
  user_id: string;
  first_name: string | null;
  last_name: string | null;
  phone: string | null;
  nationality: string | null;
  country_of_residence: string | null;
  preferred_currency: string | null;
  preferred_language: string | null;
  timezone: string | null;
  meal_preference: string | null;
  seat_preference: string | null;
  cabin_class_preference: string | null;
  updated_at: string;
}

export interface Address {
  id: string;
  user_id: string;
  organization_id: string | null;
  type: string;
  label: string | null;
  line1: string | null;
  line2: string | null;
  city: string | null;
  state: string | null;
  postal_code: string | null;
  country: string | null;
  is_default: boolean;
  created_at: string;
  updated_at: string;
  deleted_at: string | null;
}

export interface TravelDocument {
  id: string;
  user_id: string;
  organization_id: string | null;
  type: string;
  document_number: string | null;
  issuing_country: string | null;
  nationality: string | null;
  /** ISO date string "YYYY-MM-DD" */
  issue_date: string | null;
  /** ISO date string "YYYY-MM-DD" */
  expiry_date: string | null;
  is_primary: boolean;
  notes: string | null;
  created_at: string;
  updated_at: string;
  deleted_at: string | null;
}

export interface UserInfo {
  profile: UserProfile | null;
  home_address: Address | null;
  primary_passport: TravelDocument | null;
}

export interface ProfilePatch {
  first_name?: string | null;
  last_name?: string | null;
  phone?: string | null;
  nationality?: string | null;
  country_of_residence?: string | null;
  preferred_currency?: string | null;
  preferred_language?: string | null;
  timezone?: string | null;
  meal_preference?: string | null;
  seat_preference?: string | null;
  cabin_class_preference?: string | null;
}

export interface AddressPatch {
  label?: string | null;
  line1?: string | null;
  line2?: string | null;
  city?: string | null;
  state?: string | null;
  postal_code?: string | null;
  country?: string | null;
}

export interface PassportPatch {
  document_number?: string | null;
  issuing_country?: string | null;
  nationality?: string | null;
  /** ISO date string "YYYY-MM-DD" */
  issue_date?: string | null;
  /** ISO date string "YYYY-MM-DD" */
  expiry_date?: string | null;
}

export interface UpdateUserInfoRequest {
  profile?: ProfilePatch;
  home_address?: AddressPatch;
  primary_passport?: PassportPatch;
}

// ── Fetch helpers ───────────────────────────────────────────────────────────

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
  if (resp.status === 401) {
    resp = await send();
  }
  return resp;
}

async function formatHttpError(resp: Response, fallback: string): Promise<string> {
  const text = await resp.text().catch(() => '');
  if (text && text.length < 200) return `${fallback}: ${text}`;
  return `${fallback} (HTTP ${resp.status})`;
}

// ── Public API ──────────────────────────────────────────────────────────────

export async function fetchUserInfo(): Promise<UserInfo> {
  const resp = await authedFetch('/users/me/info', { method: 'GET' });
  if (!resp.ok) {
    throw new Error(await formatHttpError(resp, 'Failed to load your profile'));
  }
  return (await resp.json()) as UserInfo;
}

export async function updateUserInfo(req: UpdateUserInfoRequest): Promise<UserInfo> {
  const resp = await authedFetch('/users/me/info', {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(req),
  });
  if (!resp.ok) {
    throw new Error(await formatHttpError(resp, "Couldn't save your changes"));
  }
  return (await resp.json()) as UserInfo;
}
