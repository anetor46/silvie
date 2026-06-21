/**
 * Posts an Approve / Reject decision for a pending tool confirmation back
 * to the backend, which routes it through the `ConfirmationRegistry` to
 * unblock the parked tool call in the still-open `/chat` SSE stream.
 *
 * Returns nothing on success; throws on transport failure or 404 (the
 * confirmation has already expired or never existed).
 */

import { getAccessToken } from '$lib/services/auth';

const BASE_URL =
  (import.meta.env.VITE_SILVIE_SERVER_URL as string | undefined) ?? 'http://localhost:8080';

export async function postConfirmation(
  callId: string,
  approved: boolean,
  reason?: string,
): Promise<void> {
  const token = await getAccessToken();
  if (!token) {
    throw new Error('Not signed in.');
  }

  const response = await fetch(`${BASE_URL}/chat/confirmations`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Authorization: `Bearer ${token}`,
    },
    body: JSON.stringify({
      call_id: callId,
      approved,
      reason: reason ?? null,
    }),
  });

  if (response.status === 404) {
    throw new Error('This confirmation has expired.');
  }
  if (!response.ok) {
    throw new Error(`server returned ${response.status} ${response.statusText}`);
  }
}
