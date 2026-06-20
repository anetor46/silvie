import {
  addPaymentMethod,
  getStoredPaymentMethod,
  removeStoredPaymentMethod,
  storePaymentMethod,
  type StoredPaymentMethod,
} from '$lib/services/payment';
import type { Stripe, StripeElements } from '@stripe/stripe-js';

class PaymentStore {
  method = $state<StoredPaymentMethod | null>(null);
  loading = $state(false);
  error = $state<string | null>(null);

  async load(): Promise<void> {
    try {
      this.method = await getStoredPaymentMethod();
    } catch {
      // Silently ignore on load — keychain may simply be empty
    }
  }

  async add(stripe: Stripe, elements: StripeElements): Promise<void> {
    this.loading = true;
    this.error = null;
    try {
      this.method = await addPaymentMethod(stripe, elements);
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
    } finally {
      this.loading = false;
    }
  }

  async updateBilling(patch: {
    billing_line1?: string | null;
    billing_city?: string | null;
    billing_state?: string | null;
    billing_postal_code?: string | null;
    billing_country?: string | null;
  }): Promise<void> {
    if (!this.method) return;
    this.loading = true;
    this.error = null;
    try {
      const updated: StoredPaymentMethod = { ...this.method, ...patch };
      await storePaymentMethod(updated);
      this.method = updated;
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
    } finally {
      this.loading = false;
    }
  }

  async remove(): Promise<void> {
    this.loading = true;
    this.error = null;
    try {
      await removeStoredPaymentMethod();
      this.method = null;
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
    } finally {
      this.loading = false;
    }
  }
}

export const payment = new PaymentStore();
