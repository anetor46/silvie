<script lang="ts">
  import { onMount } from 'svelte';
  import { loadStripe, type Stripe, type StripeElements } from '@stripe/stripe-js';
  import BrandMark from './BrandMark.svelte';
  import { profile } from '$lib/stores/profile.svelte';
  import { payment } from '$lib/stores/payment.svelte';

  let { ondone }: { ondone: () => void } = $props();

  let step = $state<1 | 2>(1);

  // Step 1 — profile
  let firstName = $state('');
  let lastName = $state('');
  let email = $state('');
  let profileError = $state<string | null>(null);

  // Step 2 — payment
  const PUBLISHABLE_KEY = import.meta.env.VITE_STRIPE_PUBLISHABLE_KEY as string | undefined;
  let stripe = $state<Stripe | null>(null);
  let elements = $state<StripeElements | null>(null);
  let showCardForm = $state(false);
  let stripeError = $state<string | null>(null);
  let cardMountEl = $state<HTMLDivElement | undefined>(undefined);

  onMount(async () => {
    if (PUBLISHABLE_KEY) stripe = await loadStripe(PUBLISHABLE_KEY);
  });

  async function submitProfile() {
    profileError = null;
    if (!firstName.trim() || !lastName.trim() || !email.trim()) {
      profileError = 'Please fill in all fields.';
      return;
    }
    await profile.save({
      first_name: firstName.trim(),
      last_name: lastName.trim(),
      email: email.trim(),
    });
    if (profile.error) {
      profileError = profile.error;
    } else {
      step = 2;
    }
  }

  async function openCardForm() {
    if (!stripe) return;
    showCardForm = true;
    stripeError = null;
    await Promise.resolve();
    if (cardMountEl) {
      elements = stripe.elements();
      elements.create('card', {
        style: {
          base: {
            color: 'var(--text-primary, #1a1a1a)',
            fontFamily: 'system-ui, sans-serif',
            fontSize: '15px',
            '::placeholder': { color: 'var(--text-muted, #888)' },
          },
        },
      }).mount(cardMountEl);
    }
  }

  async function submitCard() {
    if (!stripe || !elements) return;
    stripeError = null;
    await payment.add(stripe, elements);
    if (payment.error) {
      stripeError = payment.error;
    } else {
      ondone();
    }
  }
</script>

<div class="overlay">
  <div class="card">
    <div class="ob-header">
      <BrandMark size={36} radius={9} />
      <h1 class="ob-title">Welcome to Silvie</h1>
      <p class="ob-subtitle">Your AI chief of staff for business travel.</p>
    </div>

    <div class="steps">
      <span class="step" class:active={step === 1} class:done={step > 1}>1</span>
      <div class="step-line"></div>
      <span class="step" class:active={step === 2}>2</span>
    </div>

    {#if step === 1}
      <div class="step-body">
        <h2 class="step-title">Tell us about you</h2>
        <div class="name-row">
          <div class="field">
            <label class="field-label" for="ob-first">First name</label>
            <input
              id="ob-first"
              class="field-input"
              type="text"
              bind:value={firstName}
              placeholder="Jane"
              autocomplete="given-name"
            />
          </div>
          <div class="field">
            <label class="field-label" for="ob-last">Last name</label>
            <input
              id="ob-last"
              class="field-input"
              type="text"
              bind:value={lastName}
              placeholder="Smith"
              autocomplete="family-name"
            />
          </div>
        </div>
        <div class="field">
          <label class="field-label" for="ob-email">Work email</label>
          <input
            id="ob-email"
            class="field-input"
            type="email"
            bind:value={email}
            placeholder="jane@company.com"
            autocomplete="email"
          />
        </div>
        {#if profileError}
          <p class="error-msg">{profileError}</p>
        {/if}
        <button
          class="btn btn-primary btn-full"
          onclick={submitProfile}
          disabled={profile.loading}
        >
          {profile.loading ? 'Saving…' : 'Continue'}
        </button>
      </div>
    {:else}
      <div class="step-body">
        <h2 class="step-title">Add a payment card</h2>
        <p class="step-desc">
          Used to book hotels directly in the app. Stored securely in your OS keychain — Silvie
          never sees the card number.
        </p>

        {#if !PUBLISHABLE_KEY}
          <p class="error-msg">Payment is not configured (VITE_STRIPE_PUBLISHABLE_KEY missing).</p>
        {:else if showCardForm}
          <div class="stripe-element" bind:this={cardMountEl}></div>
          {#if stripeError}
            <p class="error-msg">{stripeError}</p>
          {/if}
          <button class="btn btn-primary btn-full" onclick={submitCard} disabled={payment.loading}>
            {payment.loading ? 'Saving…' : 'Save card & finish'}
          </button>
        {:else}
          <button class="btn btn-secondary btn-full" onclick={openCardForm} disabled={!stripe}>
            Add card
          </button>
        {/if}

        <button class="btn btn-ghost btn-full" onclick={ondone} disabled={payment.loading}>
          Skip for now
        </button>
      </div>
    {/if}
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: var(--bg);
    z-index: 200;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
  }

  .card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 16px;
    padding: 40px 36px;
    width: 100%;
    max-width: 420px;
    display: flex;
    flex-direction: column;
    gap: 28px;
  }

  .ob-header {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    text-align: center;
  }

  .ob-title {
    font-size: 20px;
    font-weight: 700;
    letter-spacing: -0.02em;
    color: var(--text-primary);
    margin: 0;
  }

  .ob-subtitle {
    font-size: 13px;
    color: var(--text-muted);
    margin: 0;
  }

  .steps {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0;
  }

  .step {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    border: 2px solid var(--border-strong);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-muted);
    background: var(--bg);
    flex-shrink: 0;
  }

  .step.active {
    border-color: var(--purple-600);
    color: var(--purple-600);
    background: var(--purple-50);
  }

  .step.done {
    border-color: var(--purple-600);
    background: var(--purple-600);
    color: #fff;
  }

  .step-line {
    width: 60px;
    height: 2px;
    background: var(--border);
  }

  .step-body {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .step-title {
    font-size: 16px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }

  .step-desc {
    font-size: 13px;
    color: var(--text-muted);
    margin: -2px 0 0;
    line-height: 1.5;
  }

  .name-row {
    display: flex;
    gap: 10px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 5px;
    flex: 1;
  }

  .field-label {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .field-input {
    padding: 9px 12px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg);
    color: var(--text-primary);
    font-family: inherit;
    font-size: 14px;
    width: 100%;
    transition: border-color 0.15s;
  }

  .field-input:focus {
    outline: none;
    border-color: var(--purple-400);
  }

  .stripe-element {
    padding: 12px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg);
  }

  .btn {
    padding: 10px 20px;
    border-radius: 8px;
    font-size: 14px;
    font-weight: 500;
    font-family: inherit;
    cursor: pointer;
    border: 1px solid transparent;
    transition: opacity 0.15s, background 0.15s;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-full {
    width: 100%;
  }

  .btn-primary {
    background: var(--purple-600);
    color: #fff;
  }

  .btn-primary:hover:not(:disabled) {
    background: var(--purple-800);
  }

  .btn-secondary {
    background: transparent;
    border-color: var(--border-strong);
    color: var(--text-primary);
  }

  .btn-secondary:hover:not(:disabled) {
    background: var(--surface-hover);
  }

  .btn-ghost {
    background: transparent;
    color: var(--text-muted);
    font-size: 13px;
  }

  .error-msg {
    font-size: 12px;
    color: var(--error);
    margin: 0;
  }
</style>
