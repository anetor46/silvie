<script lang="ts">
  import { onMount } from 'svelte';
  import { loadStripe, type Stripe, type StripeElements } from '@stripe/stripe-js';
  import { payment } from '$lib/stores/payment.svelte';

  const PUBLISHABLE_KEY = import.meta.env.VITE_STRIPE_PUBLISHABLE_KEY as string | undefined;

  let stripe = $state<Stripe | null>(null);
  let elements = $state<StripeElements | null>(null);
  let showForm = $state(false);
  let stripeError = $state<string | null>(null);
  let cardMountEl = $state<HTMLDivElement | undefined>(undefined);

  onMount(async () => {
    await payment.load();
    if (PUBLISHABLE_KEY) {
      stripe = await loadStripe(PUBLISHABLE_KEY);
    }
  });

  function cardBrandIcon(brand: string): string {
    const icons: Record<string, string> = {
      visa: '💳',
      mastercard: '💳',
      amex: '💳',
      discover: '💳',
    };
    return icons[brand.toLowerCase()] ?? '💳';
  }

  function formatBrand(brand: string): string {
    const names: Record<string, string> = {
      visa: 'Visa',
      mastercard: 'Mastercard',
      amex: 'American Express',
      discover: 'Discover',
    };
    return names[brand.toLowerCase()] ?? brand.charAt(0).toUpperCase() + brand.slice(1);
  }

  async function openCardForm() {
    if (!stripe) {
      stripeError = 'Stripe is not initialised. Check VITE_STRIPE_PUBLISHABLE_KEY.';
      return;
    }
    showForm = true;
    stripeError = null;
    // Wait for the DOM to update so cardMountEl is available
    await Promise.resolve();
    if (cardMountEl) {
      elements = stripe.elements();
      const cardElement = elements.create('card', {
        style: {
          base: {
            color: 'var(--text-primary, #1a1a1a)',
            fontFamily: 'system-ui, sans-serif',
            fontSize: '16px',
            '::placeholder': { color: 'var(--text-muted, #888)' },
          },
        },
      });
      cardElement.mount(cardMountEl);
    }
  }

  async function handleSubmit() {
    if (!stripe || !elements) return;
    stripeError = null;
    await payment.add(stripe, elements);
    if (!payment.error) {
      showForm = false;
    } else {
      stripeError = payment.error;
    }
  }

  function handleCancel() {
    showForm = false;
    stripeError = null;
  }
</script>

<div class="payment-page">
  <header>
    <h1>Payment</h1>
    <p class="subtitle">Your card is stored securely in the OS keychain — Silvie never sees the card number.</p>
  </header>

  {#if payment.method}
    <section class="card-display">
      <div class="card-info">
        <span class="brand-icon">{cardBrandIcon(payment.method.brand)}</span>
        <div>
          <span class="card-number">•••• {payment.method.last4}</span>
          <span class="card-meta">
            {formatBrand(payment.method.brand)} · {payment.method.exp_month}/{payment.method.exp_year}
          </span>
        </div>
      </div>
      <button
        class="btn btn-secondary"
        onclick={() => payment.remove()}
        disabled={payment.loading}
      >
        {payment.loading ? 'Removing…' : 'Remove'}
      </button>
    </section>

    {#if payment.error}
      <p class="error-msg">{payment.error}</p>
    {/if}
  {:else if showForm}
    <section class="card-form">
      <p class="field-label">Card details</p>
      <div class="stripe-element" aria-label="Card details" bind:this={cardMountEl}></div>

      {#if stripeError}
        <p class="error-msg">{stripeError}</p>
      {/if}

      <div class="form-actions">
        <button class="btn btn-primary" onclick={handleSubmit} disabled={payment.loading}>
          {payment.loading ? 'Saving…' : 'Save card'}
        </button>
        <button class="btn btn-ghost" onclick={handleCancel} disabled={payment.loading}>
          Cancel
        </button>
      </div>
    </section>
  {:else}
    <section class="empty-state">
      <p>No payment method added yet.</p>
      {#if !PUBLISHABLE_KEY}
        <p class="error-msg">VITE_STRIPE_PUBLISHABLE_KEY is not set in your .env file.</p>
      {:else}
        <button class="btn btn-primary" onclick={openCardForm}>Add card</button>
      {/if}
    </section>
  {/if}
</div>

<style>
  .payment-page {
    padding: 2rem;
    max-width: 480px;
  }

  header {
    margin-bottom: 2rem;
  }

  h1 {
    font-size: 1.25rem;
    font-weight: 600;
    margin: 0 0 0.25rem;
  }

  .subtitle {
    font-size: 0.875rem;
    color: var(--text-muted, #666);
    margin: 0;
  }

  .card-display {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 1rem 1.25rem;
    border: 1px solid var(--border, #e0e0e0);
    border-radius: 8px;
    background: var(--surface, #fafafa);
  }

  .card-info {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .brand-icon {
    font-size: 1.5rem;
  }

  .card-number {
    display: block;
    font-weight: 600;
    font-size: 0.9375rem;
    letter-spacing: 0.05em;
  }

  .card-meta {
    display: block;
    font-size: 0.8125rem;
    color: var(--text-muted, #666);
  }

  .card-form {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .field-label {
    font-size: 0.875rem;
    font-weight: 500;
  }

  .stripe-element {
    padding: 0.75rem 1rem;
    border: 1px solid var(--border, #e0e0e0);
    border-radius: 6px;
    background: var(--surface, #fff);
  }

  .form-actions {
    display: flex;
    gap: 0.75rem;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 1rem;
    color: var(--text-muted, #666);
    font-size: 0.9375rem;
  }

  .error-msg {
    font-size: 0.875rem;
    color: var(--color-error, #d32f2f);
    margin: 0;
  }

  .btn {
    padding: 0.5rem 1.25rem;
    border-radius: 6px;
    font-size: 0.9375rem;
    font-weight: 500;
    cursor: pointer;
    border: 1px solid transparent;
    transition: opacity 0.15s;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-primary {
    background: var(--color-accent, #1a73e8);
    color: #fff;
  }

  .btn-secondary {
    background: transparent;
    border-color: var(--border, #e0e0e0);
    color: var(--text-primary, #333);
  }

  .btn-ghost {
    background: transparent;
    color: var(--text-muted, #666);
  }
</style>
