<script lang="ts">
  import { onMount } from 'svelte';
  import { loadStripe, type Stripe, type StripeElements } from '@stripe/stripe-js';
  import { profile } from '$lib/stores/profile.svelte';
  import { payment } from '$lib/stores/payment.svelte';

  let { open = $bindable(false) }: { open?: boolean } = $props();

  const PUBLISHABLE_KEY = import.meta.env.VITE_STRIPE_PUBLISHABLE_KEY as string | undefined;
  let stripe = $state<Stripe | null>(null);
  let elements = $state<StripeElements | null>(null);

  let editFirst = $state('');
  let editLast = $state('');
  let editEmail = $state('');
  let profileSaved = $state(false);

  let showCardForm = $state(false);
  let stripeError = $state<string | null>(null);
  let cardMountEl = $state<HTMLDivElement | undefined>(undefined);

  onMount(async () => {
    if (PUBLISHABLE_KEY) stripe = await loadStripe(PUBLISHABLE_KEY);
  });

  $effect(() => {
    if (open && profile.data) {
      editFirst = profile.data.first_name;
      editLast = profile.data.last_name;
      editEmail = profile.data.email;
      profileSaved = false;
    }
  });

  async function saveProfileEdits() {
    if (!editFirst.trim() || !editLast.trim() || !editEmail.trim()) return;
    await profile.save({
      first_name: editFirst.trim(),
      last_name: editLast.trim(),
      email: editEmail.trim(),
    });
    if (!profile.error) profileSaved = true;
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
            fontSize: '14px',
            '::placeholder': { color: 'var(--text-muted, #888)' },
          },
        },
      }).mount(cardMountEl);
    }
  }

  async function handleCardSubmit() {
    if (!stripe || !elements) return;
    stripeError = null;
    await payment.add(stripe, elements);
    if (!payment.error) {
      showCardForm = false;
    } else {
      stripeError = payment.error;
    }
  }

  function handleClose() {
    open = false;
    showCardForm = false;
    stripeError = null;
  }
</script>

{#if open}
  <div
    class="overlay"
    onclick={handleClose}
    onkeydown={(e) => e.key === 'Escape' && handleClose()}
    role="button"
    tabindex="-1"
    aria-label="Close account panel"
  ></div>
{/if}

<aside class="panel" class:open aria-label="Account">
  <div class="panel-header">
    <h2 class="panel-title">Account</h2>
    <button class="close-btn" onclick={handleClose} aria-label="Close">
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
        <line x1="18" y1="6" x2="6" y2="18" />
        <line x1="6" y1="6" x2="18" y2="18" />
      </svg>
    </button>
  </div>

  <div class="panel-body">
    <section class="section">
      <h3 class="section-label">Profile</h3>
      <div class="name-row">
        <div class="field">
          <label class="field-label" for="up-first">First name</label>
          <input id="up-first" class="field-input" type="text" bind:value={editFirst} placeholder="Jane" />
        </div>
        <div class="field">
          <label class="field-label" for="up-last">Last name</label>
          <input id="up-last" class="field-input" type="text" bind:value={editLast} placeholder="Smith" />
        </div>
      </div>
      <div class="field">
        <label class="field-label" for="up-email">Email</label>
        <input id="up-email" class="field-input" type="email" bind:value={editEmail} placeholder="jane@company.com" />
      </div>
      {#if profile.error}
        <p class="error-msg">{profile.error}</p>
      {/if}
      <button
        class="btn btn-primary"
        onclick={saveProfileEdits}
        disabled={profile.loading || !editFirst.trim() || !editLast.trim() || !editEmail.trim()}
      >
        {profile.loading ? 'Saving…' : profileSaved ? 'Saved!' : 'Save changes'}
      </button>
    </section>

    <div class="divider"></div>

    <section class="section">
      <h3 class="section-label">Payment</h3>
      <p class="section-desc">Stored in the OS keychain — Silvie never sees your card number.</p>

      {#if payment.method}
        <div class="card-row">
          <div class="card-info">
            <svg class="card-icon" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
              <rect x="1" y="4" width="22" height="16" rx="2" />
              <line x1="1" y1="10" x2="23" y2="10" />
            </svg>
            <div>
              <span class="card-number">•••• {payment.method.last4}</span>
              <span class="card-meta">{formatBrand(payment.method.brand)} · {payment.method.exp_month}/{payment.method.exp_year}</span>
            </div>
          </div>
          <button class="btn btn-ghost btn-sm" onclick={() => payment.remove()} disabled={payment.loading}>
            {payment.loading ? '…' : 'Remove'}
          </button>
        </div>
        {#if payment.error}
          <p class="error-msg">{payment.error}</p>
        {/if}
      {:else if showCardForm}
        <div class="stripe-element" bind:this={cardMountEl}></div>
        {#if stripeError}
          <p class="error-msg">{stripeError}</p>
        {/if}
        <div class="form-actions">
          <button class="btn btn-primary" onclick={handleCardSubmit} disabled={payment.loading}>
            {payment.loading ? 'Saving…' : 'Save card'}
          </button>
          <button
            class="btn btn-ghost"
            onclick={() => { showCardForm = false; stripeError = null; }}
            disabled={payment.loading}
          >
            Cancel
          </button>
        </div>
      {:else}
        {#if !PUBLISHABLE_KEY}
          <p class="error-msg">VITE_STRIPE_PUBLISHABLE_KEY is not configured.</p>
        {:else}
          <button class="btn btn-secondary" onclick={openCardForm}>Add card</button>
        {/if}
      {/if}
    </section>
  </div>
</aside>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(26, 26, 46, 0.4);
    z-index: 99;
    animation: fade-in 0.2s ease;
    cursor: default;
  }

  @keyframes fade-in {
    from { opacity: 0; }
    to   { opacity: 1; }
  }

  .panel {
    position: fixed;
    top: 0;
    right: 0;
    height: 100vh;
    width: 320px;
    background: var(--surface);
    border-left: 1px solid var(--border);
    z-index: 100;
    display: flex;
    flex-direction: column;
    transform: translateX(100%);
    transition: transform 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .panel.open {
    transform: translateX(0);
  }

  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 12px 0 20px;
    height: 52px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .panel-title {
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .close-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    border-radius: 8px;
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }

  .close-btn:hover {
    background: var(--surface-hover);
    color: var(--purple-600);
  }

  .panel-body {
    flex: 1;
    overflow-y: auto;
    padding: 20px;
    display: flex;
    flex-direction: column;
  }

  .section {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 8px 0 20px;
  }

  .section-label {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
    margin: 0;
  }

  .section-desc {
    font-size: 12px;
    color: var(--text-muted);
    margin: -4px 0 0;
  }

  .divider {
    height: 1px;
    background: var(--border);
    margin: 0 0 4px;
  }

  .name-row {
    display: flex;
    gap: 10px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex: 1;
  }

  .field-label {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .field-input {
    padding: 8px 10px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg);
    color: var(--text-primary);
    font-family: inherit;
    font-size: 13px;
    width: 100%;
    transition: border-color 0.15s;
  }

  .field-input:focus {
    outline: none;
    border-color: var(--purple-400);
  }

  .card-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg);
  }

  .card-info {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .card-icon {
    color: var(--text-muted);
  }

  .card-number {
    display: block;
    font-weight: 600;
    font-size: 13px;
    letter-spacing: 0.04em;
    color: var(--text-primary);
  }

  .card-meta {
    display: block;
    font-size: 11px;
    color: var(--text-muted);
  }

  .stripe-element {
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg);
  }

  .form-actions {
    display: flex;
    gap: 8px;
  }

  .btn {
    padding: 8px 14px;
    border-radius: 6px;
    font-size: 13px;
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
  }

  .btn-sm {
    padding: 5px 10px;
    font-size: 12px;
  }

  .error-msg {
    font-size: 12px;
    color: var(--error);
    margin: 0;
  }
</style>
