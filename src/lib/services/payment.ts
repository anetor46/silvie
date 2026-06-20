import type { Stripe, StripeElements, StripeError } from '@stripe/stripe-js';
import { getAccessToken } from '$lib/services/auth';

const BASE_URL =
  (import.meta.env.VITE_SILVIE_SERVER_URL as string | undefined) ?? 'http://localhost:8080';

// ── Wire types (match the Rust shapes) ──────────────────────────────────────

export interface PaymentMethod {
  id: string;
  user_id: string;
  organization_id: string | null;
  stripe_customer_id: string;
  stripe_payment_method_id: string;
  last4: string | null;
  brand: string | null;
  exp_month: number | null;
  exp_year: number | null;
  label: string | null;
  is_default: boolean;
  billing_address_id: string | null;
  created_at: string;
  updated_at: string;
  deleted_at: string | null;
}

export interface BillingAddress {
  id: string;
  line1: string | null;
  line2: string | null;
  city: string | null;
  state: string | null;
  postal_code: string | null;
  country: string | null;
}

export interface PaymentMethodView {
  payment_method: PaymentMethod;
  billing_address: BillingAddress | null;
}

export interface BillingPatch {
  line1?: string | null;
  line2?: string | null;
  city?: string | null;
  state?: string | null;
  postal_code?: string | null;
  country?: string | null;
}

// ── Stripe setup-intent flow ────────────────────────────────────────────────

export interface SetupIntentResponse {
  client_secret: string;
  customer_id: string;
}

interface PaymentMethodDetails {
  last4: string;
  brand: string;
  exp_month: number;
  exp_year: number;
}

export async function createSetupIntent(): Promise<SetupIntentResponse> {
  const resp = await fetch(`${BASE_URL}/payment/setup`, { method: 'POST' });
  if (!resp.ok) {
    const text = await resp.text().catch(() => '');
    console.error('[payment.setup]', resp.status, text);
    throw new Error('Could not start payment setup. Please try again in a moment.');
  }
  return (await resp.json()) as SetupIntentResponse;
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
    const text = await resp.text().catch(() => '');
    console.error('[payment.method]', resp.status, text);
    throw new Error("Couldn't read card details from Stripe. Please try again.");
  }
  return (await resp.json()) as PaymentMethodDetails;
}

/**
 * Confirm the SetupIntent the user just filled in via the Payment Element,
 * then persist the resulting PaymentMethod + display metadata to our backend.
 *
 * `elements` must have a Payment Element mounted already — see
 * `UserPanel.svelte` for the mount flow (it has to happen with the
 * `clientSecret` returned by `createSetupIntent()`).
 */
export async function confirmAndSavePaymentMethod(
  stripe: Stripe,
  elements: StripeElements,
  customer_id: string,
): Promise<PaymentMethodView> {
  const { setupIntent, error } = await stripe.confirmSetup({
    elements,
    confirmParams: { return_url: window.location.href },
    redirect: 'if_required',
  });

  if (error) {
    throw stripeError(error);
  }
  if (!setupIntent?.payment_method) {
    console.error('[Stripe] confirmSetup returned no payment_method', setupIntent);
    throw new Error('Stripe did not return a payment method. Please try again.');
  }

  const payment_method_id =
    typeof setupIntent.payment_method === 'string'
      ? setupIntent.payment_method
      : setupIntent.payment_method.id;

  const details = await fetchPaymentMethodDetails(customer_id, payment_method_id);

  return await createPaymentMethod({
    stripe_customer_id: customer_id,
    stripe_payment_method_id: payment_method_id,
    last4: details.last4,
    brand: details.brand,
    exp_month: details.exp_month,
    exp_year: details.exp_year,
  });
}

// ── Backend (authenticated) CRUD ────────────────────────────────────────────

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

/** GET /users/me/payment-method — returns null on 404 (no saved card). */
export async function fetchPaymentMethod(): Promise<PaymentMethodView | null> {
  const resp = await authedFetch('/users/me/payment-method', { method: 'GET' });
  if (resp.status === 404) return null;
  if (!resp.ok) {
    console.error('[payment-method.fetch]', resp.status, await resp.text().catch(() => ''));
    throw new Error("Couldn't load your payment method.");
  }
  return (await resp.json()) as PaymentMethodView;
}

interface CreatePaymentMethodBody {
  stripe_customer_id: string;
  stripe_payment_method_id: string;
  last4: string;
  brand: string;
  exp_month: number;
  exp_year: number;
}

async function createPaymentMethod(body: CreatePaymentMethodBody): Promise<PaymentMethodView> {
  const resp = await authedFetch('/users/me/payment-method', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });
  if (!resp.ok) {
    console.error(
      '[payment-method.create]',
      resp.status,
      await resp.text().catch(() => ''),
    );
    throw new Error("Couldn't save your card. Please try again.");
  }
  return (await resp.json()) as PaymentMethodView;
}

export async function deletePaymentMethod(): Promise<void> {
  const resp = await authedFetch('/users/me/payment-method', { method: 'DELETE' });
  if (!resp.ok && resp.status !== 404) {
    console.error(
      '[payment-method.delete]',
      resp.status,
      await resp.text().catch(() => ''),
    );
    throw new Error("Couldn't remove your card.");
  }
}

export async function updateBillingAddress(
  patch: BillingPatch,
): Promise<PaymentMethodView> {
  const resp = await authedFetch('/users/me/payment-method/billing', {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(patch),
  });
  if (!resp.ok) {
    console.error(
      '[billing.update]',
      resp.status,
      await resp.text().catch(() => ''),
    );
    if (resp.status === 404) {
      throw new Error('Add a card first before saving a billing address.');
    }
    throw new Error("Couldn't save your billing address.");
  }
  return (await resp.json()) as PaymentMethodView;
}

// ── Stripe error → friendly message ─────────────────────────────────────────

/**
 * Convert a Stripe.js error into an Error with a user-friendly message,
 * while logging the raw payload to the console for debugging.
 */
function stripeError(err: StripeError): Error {
  console.error('[Stripe]', err);
  const code = err.code ?? '';
  // Common cases from https://stripe.com/docs/error-codes
  const map: Record<string, string> = {
    card_declined: 'Your card was declined. Please try a different card.',
    expired_card: 'Your card has expired.',
    incorrect_cvc: 'The security code you entered is incorrect.',
    incorrect_number: 'The card number you entered is incorrect.',
    invalid_cvc: 'The security code is invalid.',
    invalid_expiry_month: 'The expiration month is invalid.',
    invalid_expiry_year: 'The expiration year is invalid.',
    invalid_number: 'The card number is invalid.',
    insufficient_funds: 'Your card was declined due to insufficient funds.',
    processing_error: 'Something went wrong processing your card. Please try again.',
    setup_intent_authentication_failure:
      'Your bank declined the authentication. Please try a different card.',
    payment_method_unactivated:
      'This payment method is not active. Please contact your bank or try another card.',
    api_connection_error: "Couldn't reach the payment service. Check your connection.",
  };
  if (code && map[code]) return new Error(map[code]);
  // Generic fallback for anything else
  return new Error('Your card could not be saved. Please check the details and try again.');
}
