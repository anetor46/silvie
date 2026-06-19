import { invoke } from '@tauri-apps/api/core';
import type { Stripe, StripeElements } from '@stripe/stripe-js';

const BASE_URL =
  (import.meta.env.VITE_SILVIE_SERVER_URL as string | undefined) ?? 'http://localhost:8080';

export interface StoredPaymentMethod {
  customer_id: string;
  payment_method_id: string;
  last4: string;
  brand: string;
  exp_month: number;
  exp_year: number;
}

// ── Tauri keychain commands ───────────────────────────────────────────────────

function isTauri(): boolean {
  return !!(window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
}

export async function getStoredPaymentMethod(): Promise<StoredPaymentMethod | null> {
  if (!isTauri()) return null;
  return await invoke<StoredPaymentMethod | null>('get_payment_method');
}

export async function storePaymentMethod(pm: StoredPaymentMethod): Promise<void> {
  if (!isTauri()) throw new Error('This feature requires the Silvie desktop app.');
  await invoke('store_payment_method', { data: pm });
}

export async function removeStoredPaymentMethod(): Promise<void> {
  if (!isTauri()) throw new Error('This feature requires the Silvie desktop app.');
  await invoke('remove_payment_method');
}

// ── Server API calls ──────────────────────────────────────────────────────────

interface SetupIntentResponse {
  client_secret: string;
  customer_id: string;
}

interface PaymentMethodDetails {
  last4: string;
  brand: string;
  exp_month: number;
  exp_year: number;
}

async function createSetupIntent(): Promise<SetupIntentResponse> {
  const resp = await fetch(`${BASE_URL}/payment/setup`, { method: 'POST' });
  if (!resp.ok) {
    throw new Error(`Payment setup failed: server returned ${resp.status}`);
  }
  return resp.json() as Promise<SetupIntentResponse>;
}

async function fetchPaymentMethodDetails(
  customer_id: string,
  payment_method_id: string,
): Promise<PaymentMethodDetails> {
  const resp = await fetch(`${BASE_URL}/payment/method`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ customer_id, payment_method_id }),
  });
  if (!resp.ok) {
    throw new Error(`Failed to retrieve card details: server returned ${resp.status}`);
  }
  return resp.json() as Promise<PaymentMethodDetails>;
}

// ── Full card setup flow ──────────────────────────────────────────────────────

/**
 * Completes the full card-add flow:
 * 1. Backend creates a Stripe Customer + SetupIntent
 * 2. Stripe Elements confirms the card (tokenises it on Stripe's servers)
 * 3. Backend retrieves card metadata (last4, brand, expiry — no card number)
 * 4. Saves the reference IDs + metadata to the OS keychain
 *
 * Returns the stored payment method on success.
 */
export async function addPaymentMethod(
  stripe: Stripe,
  elements: StripeElements,
): Promise<StoredPaymentMethod> {
  // Step 1: get client_secret + customer_id from our server
  const { client_secret, customer_id } = await createSetupIntent();

  // Step 2: confirm the card via Stripe Elements (card never touches our server)
  const { setupIntent, error } = await stripe.confirmSetup({
    elements,
    confirmParams: { return_url: window.location.href },
    redirect: 'if_required',
  });

  if (error) {
    throw new Error(error.message ?? 'Card confirmation failed');
  }
  if (!setupIntent?.payment_method) {
    throw new Error('Stripe did not return a payment method');
  }

  const payment_method_id =
    typeof setupIntent.payment_method === 'string'
      ? setupIntent.payment_method
      : setupIntent.payment_method.id;

  // Step 3: fetch display metadata (no card number returned)
  const details = await fetchPaymentMethodDetails(customer_id, payment_method_id);

  // Step 4: persist to OS keychain via Tauri
  const pm: StoredPaymentMethod = {
    customer_id,
    payment_method_id,
    last4: details.last4,
    brand: details.brand,
    exp_month: details.exp_month,
    exp_year: details.exp_year,
  };
  await storePaymentMethod(pm);

  return pm;
}
