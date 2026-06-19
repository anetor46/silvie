import {
  addPaymentMethod,
  getStoredPaymentMethod,
  removeStoredPaymentMethod,
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
