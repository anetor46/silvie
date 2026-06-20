import type { Stripe, StripeElements } from '@stripe/stripe-js';
import {
  confirmAndSavePaymentMethod,
  deletePaymentMethod as serviceDelete,
  fetchPaymentMethod,
  updateBillingAddress,
  type BillingAddress,
  type BillingPatch,
  type PaymentMethod,
  type PaymentMethodView,
} from '$lib/services/payment';

class PaymentStore {
  /** The user's saved payment method, or null if none. */
  method = $state<PaymentMethod | null>(null);
  /** Billing address linked to the saved card, or null. */
  billing = $state<BillingAddress | null>(null);
  loading = $state(false);
  loaded = $state(false);
  error = $state<string | null>(null);

  async load(): Promise<void> {
    this.loading = true;
    this.error = null;
    try {
      const view = await fetchPaymentMethod();
      this.#applyView(view);
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
    } finally {
      this.loaded = true;
      this.loading = false;
    }
  }

  /**
   * Confirm the Stripe SetupIntent (using the Payment Element that
   * UserPanel has already mounted with the matching clientSecret), then
   * persist the card to our backend.
   */
  async add(stripe: Stripe, elements: StripeElements, customerId: string): Promise<void> {
    this.loading = true;
    this.error = null;
    try {
      const view = await confirmAndSavePaymentMethod(stripe, elements, customerId);
      this.#applyView(view);
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
      await serviceDelete();
      this.method = null;
      this.billing = null;
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
    } finally {
      this.loading = false;
    }
  }

  async updateBilling(patch: BillingPatch): Promise<void> {
    this.loading = true;
    this.error = null;
    try {
      const view = await updateBillingAddress(patch);
      this.#applyView(view);
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
    } finally {
      this.loading = false;
    }
  }

  reset(): void {
    this.method = null;
    this.billing = null;
    this.loaded = false;
    this.error = null;
  }

  clearError(): void {
    this.error = null;
  }

  #applyView(view: PaymentMethodView | null): void {
    this.method = view?.payment_method ?? null;
    this.billing = view?.billing_address ?? null;
  }
}

export const payment = new PaymentStore();
