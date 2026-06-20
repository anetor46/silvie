<script lang="ts">
  import { onMount } from 'svelte';
  import { loadStripe, type Stripe, type StripeElements } from '@stripe/stripe-js';
  import { profile } from '$lib/stores/profile.svelte';
  import { payment } from '$lib/stores/payment.svelte';
  import type { StoredProfile } from '$lib/services/profile';

  let { open = $bindable(false) }: { open?: boolean } = $props();

  type Screen = 'main' | 'personal' | 'travel' | 'payment';

  const screenTitles: Record<Screen, string> = {
    main: 'Account',
    personal: 'Personal Information',
    travel: 'Passport & Travel',
    payment: 'Payment',
  };

  let screen = $state<Screen>('main');
  let title = $derived(screenTitles[screen]);
  let isDetail = $derived(screen !== 'main');

  // ── Personal edit state ───────────────────────────────────────────────────
  let editFirst = $state('');
  let editLast = $state('');
  let editEmail = $state('');
  let editPhone = $state('');
  let editAddressLine1 = $state('');
  let editAddressCity = $state('');
  let editAddressState = $state('');
  let editAddressPostal = $state('');
  let editAddressCountry = $state('');

  // ── Travel edit state ─────────────────────────────────────────────────────
  let editNationality = $state('');
  let editPassportNumber = $state('');
  let editPassportExpiry = $state('');
  let editCountryOfResidence = $state('');

  // ── Billing address edit state ────────────────────────────────────────────
  let editBillingLine1 = $state('');
  let editBillingCity = $state('');
  let editBillingState = $state('');
  let editBillingPostal = $state('');
  let editBillingCountry = $state('');

  // ── Stripe ────────────────────────────────────────────────────────────────
  const PUBLISHABLE_KEY = import.meta.env.VITE_STRIPE_PUBLISHABLE_KEY as string | undefined;
  let stripe = $state<Stripe | null>(null);
  let elements = $state<StripeElements | null>(null);
  let showCardForm = $state(false);
  let stripeError = $state<string | null>(null);
  let cardMountEl = $state<HTMLDivElement | undefined>(undefined);

  onMount(async () => {
    if (PUBLISHABLE_KEY) stripe = await loadStripe(PUBLISHABLE_KEY);
  });

  // ── Navigation ────────────────────────────────────────────────────────────
  function initScreen(s: Screen) {
    const d = profile.data;
    if (!d) return;
    if (s === 'personal') {
      editFirst = d.first_name;
      editLast = d.last_name;
      editEmail = d.email;
      editPhone = d.phone ?? '';
      editAddressLine1 = d.address_line1 ?? '';
      editAddressCity = d.address_city ?? '';
      editAddressState = d.address_state ?? '';
      editAddressPostal = d.address_postal_code ?? '';
      editAddressCountry = d.address_country ?? '';
    } else if (s === 'travel') {
      editNationality = d.nationality ?? '';
      editPassportNumber = d.passport_number ?? '';
      editPassportExpiry = d.passport_expiry ?? '';
      editCountryOfResidence = d.country_of_residence ?? '';
    } else if (s === 'payment') {
      const pm = payment.method;
      editBillingLine1 = pm?.billing_line1 ?? '';
      editBillingCity = pm?.billing_city ?? '';
      editBillingState = pm?.billing_state ?? '';
      editBillingPostal = pm?.billing_postal_code ?? '';
      editBillingCountry = pm?.billing_country ?? '';
    }
  }

  function goTo(s: Screen) {
    initScreen(s);
    screen = s;
  }

  function goBack() {
    screen = 'main';
    showCardForm = false;
    stripeError = null;
  }

  function handleClose() {
    open = false;
    screen = 'main';
    showCardForm = false;
    stripeError = null;
  }

  // ── Save helpers ──────────────────────────────────────────────────────────
  async function savePersonal() {
    if (!editFirst.trim() || !editLast.trim() || !editEmail.trim()) return;
    if (!profile.data) return;
    await profile.save({
      ...profile.data,
      first_name: editFirst.trim(),
      last_name: editLast.trim(),
      email: editEmail.trim(),
      phone: editPhone.trim() || null,
      address_line1: editAddressLine1.trim() || null,
      address_city: editAddressCity.trim() || null,
      address_state: editAddressState.trim() || null,
      address_postal_code: editAddressPostal.trim() || null,
      address_country: editAddressCountry.trim() || null,
    } satisfies StoredProfile);
    if (!profile.error) goBack();
  }

  async function saveTravel() {
    if (!profile.data) return;
    await profile.save({
      ...profile.data,
      nationality: editNationality.trim() || null,
      passport_number: editPassportNumber.trim() || null,
      passport_expiry: editPassportExpiry.trim() || null,
      country_of_residence: editCountryOfResidence.trim() || null,
    } satisfies StoredProfile);
    if (!profile.error) goBack();
  }

  async function saveBilling() {
    await payment.updateBilling({
      billing_line1: editBillingLine1.trim() || null,
      billing_city: editBillingCity.trim() || null,
      billing_state: editBillingState.trim() || null,
      billing_postal_code: editBillingPostal.trim() || null,
      billing_country: editBillingCountry.trim() || null,
    });
    if (!payment.error) goBack();
  }

  function useHomeAddressForBilling() {
    const d = profile.data;
    if (!d) return;
    editBillingLine1 = d.address_line1 ?? '';
    editBillingCity = d.address_city ?? '';
    editBillingState = d.address_state ?? '';
    editBillingPostal = d.address_postal_code ?? '';
    editBillingCountry = d.address_country ?? '';
  }

  // ── Card form ─────────────────────────────────────────────────────────────
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
      // Pre-fill billing edit state from what just got stored
      editBillingLine1 = payment.method?.billing_line1 ?? '';
      editBillingCity = payment.method?.billing_city ?? '';
      editBillingState = payment.method?.billing_state ?? '';
      editBillingPostal = payment.method?.billing_postal_code ?? '';
      editBillingCountry = payment.method?.billing_country ?? '';
    } else {
      stripeError = payment.error;
    }
  }

  // ── Display helpers ───────────────────────────────────────────────────────
  const initials = $derived(
    profile.data
      ? `${profile.data.first_name.charAt(0)}${profile.data.last_name.charAt(0)}`.toUpperCase()
      : '?',
  );

  const fullName = $derived(
    profile.data ? `${profile.data.first_name} ${profile.data.last_name}` : '—',
  );

  function val(v: string | null | undefined, fallback = '—'): string {
    return v?.trim() || fallback;
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

  const hasBillingAddress = $derived(
    !!(payment.method?.billing_city || payment.method?.billing_line1),
  );

  const hasHomeAddress = $derived(
    !!(profile.data?.address_city || profile.data?.address_line1),
  );
</script>

<!-- Full-screen panel (no separate overlay — panel covers everything) -->
<div
  class="panel"
  class:open
  role="dialog"
  aria-modal="true"
  aria-label={title}
  tabindex="-1"
  onkeydown={(e) => e.key === 'Escape' && handleClose()}
>
  <!-- Header -->
  <div class="panel-header">
    {#if isDetail}
      <button class="header-btn" onclick={goBack} aria-label="Back">
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="15 18 9 12 15 6" />
        </svg>
      </button>
    {:else}
      <div class="header-btn-placeholder"></div>
    {/if}

    <span class="header-title">{title}</span>

    <button class="header-btn" onclick={handleClose} aria-label="Close">
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
        <line x1="18" y1="6" x2="6" y2="18" />
        <line x1="6" y1="6" x2="18" y2="18" />
      </svg>
    </button>
  </div>

  <!-- Scrollable content -->
  <div class="panel-body">
    <div class="content">

      <!-- ── MAIN ──────────────────────────────────────────────────── -->
      {#if screen === 'main'}
        <!-- Profile identity card -->
        <button class="profile-card" onclick={() => goTo('personal')}>
          <div class="avatar">{initials}</div>
          <div class="profile-text">
            <span class="profile-name">{fullName}</span>
            <span class="profile-email">{val(profile.data?.email)}</span>
          </div>
          <svg class="chevron" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="9 18 15 12 9 6" />
          </svg>
        </button>

        <!-- Travel -->
        <p class="section-header">Travel</p>
        <div class="group">
          <button class="row" onclick={() => goTo('travel')}>
            <span class="row-label">Passport & Documents</span>
            <span class="row-right">
              <span class="row-value">{val(profile.data?.passport_number, 'Not set')}</span>
              <svg class="chevron" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 18 15 12 9 6" /></svg>
            </span>
          </button>
        </div>

        <!-- Payment -->
        <p class="section-header">Payment</p>
        <div class="group">
          <button class="row" onclick={() => goTo('payment')}>
            <span class="row-label">Payment Card</span>
            <span class="row-right">
              <span class="row-value">{payment.method ? `•••• ${payment.method.last4}` : 'Not set'}</span>
              <svg class="chevron" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 18 15 12 9 6" /></svg>
            </span>
          </button>
        </div>

      <!-- ── PERSONAL ───────────────────────────────────────────────── -->
      {:else if screen === 'personal'}
        <p class="section-header">Identity</p>
        <div class="group">
          <div class="field">
            <label class="field-label" for="p-first">First name</label>
            <input id="p-first" class="field-input" type="text" bind:value={editFirst} placeholder="Jane" autocomplete="given-name" />
          </div>
          <div class="field">
            <label class="field-label" for="p-last">Last name</label>
            <input id="p-last" class="field-input" type="text" bind:value={editLast} placeholder="Smith" autocomplete="family-name" />
          </div>
        </div>

        <p class="section-header">Contact</p>
        <div class="group">
          <div class="field">
            <label class="field-label" for="p-email">Email</label>
            <input id="p-email" class="field-input" type="email" bind:value={editEmail} placeholder="jane@company.com" autocomplete="email" />
          </div>
          <div class="field">
            <label class="field-label" for="p-phone">Phone</label>
            <input id="p-phone" class="field-input" type="tel" bind:value={editPhone} placeholder="+1 555 000 0000" autocomplete="tel" />
          </div>
        </div>

        <p class="section-header">Home Address</p>
        <div class="group">
          <div class="field">
            <label class="field-label" for="p-line1">Street</label>
            <input id="p-line1" class="field-input" type="text" bind:value={editAddressLine1} placeholder="123 Main St" autocomplete="street-address" />
          </div>
          <div class="field">
            <label class="field-label" for="p-city">City</label>
            <input id="p-city" class="field-input" type="text" bind:value={editAddressCity} placeholder="Paris" autocomplete="address-level2" />
          </div>
          <div class="field">
            <label class="field-label" for="p-state">State / Region</label>
            <input id="p-state" class="field-input" type="text" bind:value={editAddressState} placeholder="Île-de-France" autocomplete="address-level1" />
          </div>
          <div class="field">
            <label class="field-label" for="p-postal">Postal code</label>
            <input id="p-postal" class="field-input" type="text" bind:value={editAddressPostal} placeholder="75001" autocomplete="postal-code" />
          </div>
          <div class="field">
            <label class="field-label" for="p-country">Country</label>
            <input id="p-country" class="field-input" type="text" bind:value={editAddressCountry} placeholder="France" autocomplete="country-name" />
          </div>
        </div>

        {#if profile.error}<p class="error-msg">{profile.error}</p>{/if}
        <button
          class="primary-btn"
          onclick={savePersonal}
          disabled={profile.loading || !editFirst.trim() || !editLast.trim() || !editEmail.trim()}
        >
          {profile.loading ? 'Saving…' : 'Save'}
        </button>

      <!-- ── TRAVEL ─────────────────────────────────────────────────── -->
      {:else if screen === 'travel'}
        <div class="group">
          <div class="field">
            <label class="field-label" for="t-nationality">Nationality</label>
            <input id="t-nationality" class="field-input" type="text" bind:value={editNationality} placeholder="French" />
          </div>
          <div class="field">
            <label class="field-label" for="t-residence">Country of residence</label>
            <input id="t-residence" class="field-input" type="text" bind:value={editCountryOfResidence} placeholder="France" />
          </div>
          <div class="field">
            <label class="field-label" for="t-passport">Passport number</label>
            <input id="t-passport" class="field-input" type="text" bind:value={editPassportNumber} placeholder="12AB34567" autocomplete="off" />
          </div>
          <div class="field">
            <label class="field-label" for="t-expiry">Passport expiry</label>
            <input id="t-expiry" class="field-input" type="date" bind:value={editPassportExpiry} />
          </div>
        </div>

        {#if profile.error}<p class="error-msg">{profile.error}</p>{/if}
        <button class="primary-btn" onclick={saveTravel} disabled={profile.loading}>
          {profile.loading ? 'Saving…' : 'Save'}
        </button>

      <!-- ── PAYMENT ────────────────────────────────────────────────── -->
      {:else if screen === 'payment'}
        <p class="section-header">Card</p>
        {#if payment.method}
          <div class="group">
            <div class="card-display row">
              <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" style="color:var(--text-muted);flex-shrink:0">
                <rect x="1" y="4" width="22" height="16" rx="2" />
                <line x1="1" y1="10" x2="23" y2="10" />
              </svg>
              <div class="card-text">
                <span class="card-number">•••• {payment.method.last4}</span>
                <span class="card-meta">{formatBrand(payment.method.brand)} · {payment.method.exp_month}/{payment.method.exp_year}</span>
              </div>
            </div>
          </div>
          {#if payment.error && !payment.loading}<p class="error-msg">{payment.error}</p>{/if}
          <button class="danger-btn" onclick={() => payment.remove()} disabled={payment.loading}>
            {payment.loading ? 'Removing…' : 'Remove card'}
          </button>
        {:else if showCardForm}
          <p class="hint">Your card is stored in the OS keychain — Silvie never sees the card number.</p>
          <div class="stripe-element" bind:this={cardMountEl}></div>
          {#if stripeError}<p class="error-msg">{stripeError}</p>{/if}
          <button class="primary-btn" onclick={handleCardSubmit} disabled={payment.loading}>
            {payment.loading ? 'Saving…' : 'Save card'}
          </button>
          <button class="ghost-btn" onclick={() => { showCardForm = false; stripeError = null; }} disabled={payment.loading}>Cancel</button>
        {:else}
          <p class="hint">Your card is stored in the OS keychain — Silvie never sees the card number.</p>
          {#if !PUBLISHABLE_KEY}
            <p class="error-msg">VITE_STRIPE_PUBLISHABLE_KEY is not configured.</p>
          {:else}
            <button class="primary-btn" onclick={openCardForm}>Add card</button>
          {/if}
        {/if}

        <!-- Billing address — only shown when a card is set up -->
        {#if payment.method}
          <p class="section-header">
            Billing Address
            {#if hasBillingAddress}<span class="section-badge">Saved</span>{/if}
          </p>
          <div class="group">
            <div class="field">
              <label class="field-label" for="b-line1">Street</label>
              <input id="b-line1" class="field-input" type="text" bind:value={editBillingLine1} placeholder="123 Main St" autocomplete="street-address" />
            </div>
            <div class="field">
              <label class="field-label" for="b-city">City</label>
              <input id="b-city" class="field-input" type="text" bind:value={editBillingCity} placeholder="Paris" autocomplete="address-level2" />
            </div>
            <div class="field">
              <label class="field-label" for="b-state">State / Region</label>
              <input id="b-state" class="field-input" type="text" bind:value={editBillingState} placeholder="Île-de-France" autocomplete="address-level1" />
            </div>
            <div class="field">
              <label class="field-label" for="b-postal">Postal code</label>
              <input id="b-postal" class="field-input" type="text" bind:value={editBillingPostal} placeholder="75001" autocomplete="postal-code" />
            </div>
            <div class="field">
              <label class="field-label" for="b-country">Country</label>
              <input id="b-country" class="field-input" type="text" bind:value={editBillingCountry} placeholder="France" autocomplete="country-name" />
            </div>
          </div>
          {#if hasHomeAddress}
            <button class="ghost-btn" onclick={useHomeAddressForBilling}>
              Use my home address
            </button>
          {/if}
          {#if payment.error && payment.loading === false}<p class="error-msg">{payment.error}</p>{/if}
          <button class="primary-btn" onclick={saveBilling} disabled={payment.loading}>
            {payment.loading ? 'Saving…' : 'Save billing address'}
          </button>
        {/if}
      {/if}

    </div>
  </div>
</div>

<style>
  /* ── Full-screen panel ─────────────────────────────────────────── */
  .panel {
    position: fixed;
    inset: 0;
    background: var(--surface);
    z-index: 100;
    display: flex;
    flex-direction: column;
    /* Slide up from bottom */
    transform: translateY(100%);
    visibility: hidden;
    transition:
      transform 0.3s cubic-bezier(0.4, 0, 0.2, 1),
      visibility 0s 0.3s;
  }

  .panel.open {
    transform: translateY(0);
    visibility: visible;
    transition:
      transform 0.3s cubic-bezier(0.4, 0, 0.2, 1),
      visibility 0s 0s;
  }

  /* ── Header ────────────────────────────────────────────────────── */
  .panel-header {
    display: flex;
    align-items: center;
    padding: 0 8px;
    height: 52px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    background: var(--bg);
  }

  .header-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    border-radius: 8px;
    cursor: pointer;
    flex-shrink: 0;
    transition: background 0.15s, color 0.15s;
  }

  .header-btn:hover {
    background: var(--surface-hover);
    color: var(--purple-600);
  }

  .header-btn-placeholder {
    width: 36px;
    flex-shrink: 0;
  }

  .header-title {
    flex: 1;
    text-align: center;
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary);
  }

  /* ── Scrollable body ───────────────────────────────────────────── */
  .panel-body {
    flex: 1;
    overflow-y: auto;
    padding: 20px 16px 48px;
  }

  .panel-body::-webkit-scrollbar { width: 4px; }
  .panel-body::-webkit-scrollbar-thumb { background: var(--border-strong); border-radius: 2px; }

  /* Center & constrain content width on wide screens */
  .content {
    max-width: 560px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  /* ── Profile card (main screen) ────────────────────────────────── */
  .profile-card {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 16px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 14px;
    cursor: pointer;
    text-align: left;
    width: 100%;
    margin-bottom: 12px;
    transition: background 0.15s;
  }

  .profile-card:hover { background: var(--surface-hover); }

  .avatar {
    width: 52px;
    height: 52px;
    border-radius: 50%;
    background: var(--purple-100);
    color: var(--purple-800);
    font-size: 18px;
    font-weight: 700;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    letter-spacing: 0.02em;
  }

  .profile-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .profile-name {
    font-size: 16px;
    font-weight: 600;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .profile-email {
    font-size: 13px;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* ── Section headers ───────────────────────────────────────────── */
  .section-header {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
    padding: 14px 4px 5px;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .section-badge {
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--success);
    background: #f0fdf4;
    border: 1px solid #bbf7d0;
    border-radius: 4px;
    padding: 1px 5px;
  }

  /* ── Grouped rows (main screen navigation) ─────────────────────── */
  .group {
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 12px;
    overflow: hidden;
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 13px 16px;
    width: 100%;
    background: transparent;
    border: none;
    border-bottom: 1px solid var(--border);
    cursor: pointer;
    text-align: left;
    transition: background 0.12s;
    gap: 8px;
  }

  .row:last-child { border-bottom: none; }
  .row:hover { background: var(--surface-hover); }

  .row-label {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
    white-space: nowrap;
    flex-shrink: 0;
  }

  .row-right {
    display: flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
  }

  .row-value {
    font-size: 13px;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 180px;
  }

  .chevron { color: var(--text-dim); flex-shrink: 0; }

  /* ── Card display row (payment screen) ─────────────────────────── */
  .card-display {
    cursor: default !important;
    gap: 12px;
  }

  .card-display:hover { background: transparent !important; }

  .card-text {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .card-number {
    font-size: 14px;
    font-weight: 600;
    letter-spacing: 0.04em;
    color: var(--text-primary);
  }

  .card-meta {
    font-size: 12px;
    color: var(--text-muted);
  }

  /* ── Detail form fields ─────────────────────────────────────────── */
  .field {
    display: flex;
    flex-direction: column;
    padding: 10px 16px;
    border-bottom: 1px solid var(--border);
    gap: 3px;
  }

  .field:last-child { border-bottom: none; }

  .field-label {
    font-size: 11px;
    font-weight: 500;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .field-input {
    font-family: inherit;
    font-size: 14px;
    color: var(--text-primary);
    background: transparent;
    border: none;
    outline: none;
    padding: 2px 0;
    width: 100%;
  }

  .field-input::placeholder { color: var(--text-dim); }

  /* ── Stripe element ─────────────────────────────────────────────── */
  .stripe-element {
    padding: 14px 16px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 12px;
  }

  /* ── Buttons ────────────────────────────────────────────────────── */
  .primary-btn {
    margin-top: 8px;
    padding: 13px;
    width: 100%;
    border-radius: 12px;
    font-size: 15px;
    font-weight: 600;
    font-family: inherit;
    cursor: pointer;
    border: none;
    background: var(--purple-600);
    color: #fff;
    transition: background 0.15s;
  }

  .primary-btn:hover:not(:disabled) { background: var(--purple-800); }
  .primary-btn:disabled { opacity: 0.45; cursor: not-allowed; }

  .danger-btn {
    margin-top: 8px;
    padding: 13px;
    width: 100%;
    border-radius: 12px;
    font-size: 15px;
    font-weight: 500;
    font-family: inherit;
    cursor: pointer;
    border: 1px solid var(--error);
    background: transparent;
    color: var(--error);
    transition: background 0.15s;
  }

  .danger-btn:hover:not(:disabled) { background: #fff0f0; }
  .danger-btn:disabled { opacity: 0.45; cursor: not-allowed; }

  .ghost-btn {
    margin-top: 4px;
    padding: 11px;
    width: 100%;
    border-radius: 12px;
    font-size: 14px;
    font-weight: 500;
    font-family: inherit;
    cursor: pointer;
    border: 1px solid var(--border-strong);
    background: var(--bg);
    color: var(--text-secondary);
    transition: background 0.15s;
  }

  .ghost-btn:hover:not(:disabled) { background: var(--surface-hover); }
  .ghost-btn:disabled { opacity: 0.45; cursor: not-allowed; }

  .hint {
    font-size: 13px;
    color: var(--text-muted);
    line-height: 1.5;
    padding: 4px 2px 8px;
  }

  .error-msg {
    font-size: 12px;
    color: var(--error);
    padding: 4px 2px;
  }
</style>
