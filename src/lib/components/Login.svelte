<script lang="ts">
  import BrandMark from './BrandMark.svelte';
  import { auth } from '$lib/stores/auth.svelte';

  type Mode = 'signin' | 'signup' | 'reset';

  let mode = $state<Mode>('signin');

  let email = $state('');
  let password = $state('');
  let name = $state('');

  let resetSentTo = $state<string | null>(null);

  const titles: Record<Mode, string> = {
    signin: 'Welcome',
    signup: 'Create your account',
    reset: 'Reset your password',
  };

  const submitLabels: Record<Mode, string> = {
    signin: 'Sign in',
    signup: 'Create account',
    reset: 'Send reset email',
  };

  function switchMode(next: Mode) {
    mode = next;
    auth.clearError();
    resetSentTo = null;
  }

  function disabled(): boolean {
    if (auth.loading) return true;
    if (!email.trim()) return true;
    if (mode === 'reset') return false;
    if (!password) return true;
    if (mode === 'signup' && !name.trim()) return true;
    return false;
  }

  async function handleSubmit() {
    auth.clearError();
    if (mode === 'signin') {
      await auth.login(email.trim(), password);
    } else if (mode === 'signup') {
      await auth.signup(email.trim(), password, name.trim());
    } else if (mode === 'reset') {
      try {
        await auth.requestPasswordReset(email.trim());
        resetSentTo = email.trim();
      } catch {
        // error surfaced via auth.error
      }
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !disabled()) {
      e.preventDefault();
      void handleSubmit();
    }
  }
</script>

<div class="login-screen">
  <div class="login-card">
    <div class="brand-wrap">
      <BrandMark size={44} radius={10} />
    </div>
    <h1 class="title">{titles[mode]}</h1>
    <p class="subtitle">
      {#if mode === 'signin'}
        Sign in to your Silvie account.
      {:else if mode === 'signup'}
        AI chief of staff for business travel.
      {:else}
        Enter your email and we'll send you a reset link.
      {/if}
    </p>

    {#if resetSentTo}
      <div class="success-banner">
        If an account exists for <strong>{resetSentTo}</strong>, a reset email is on its way.
      </div>
    {/if}

    <form class="form" onsubmit={(e) => { e.preventDefault(); void handleSubmit(); }}>
      {#if mode === 'signup'}
        <div class="field">
          <label class="field-label" for="auth-name">Full name</label>
          <input
            id="auth-name"
            class="field-input"
            type="text"
            bind:value={name}
            placeholder="Jane Smith"
            autocomplete="name"
            onkeydown={handleKeydown}
          />
        </div>
      {/if}

      <div class="field">
        <label class="field-label" for="auth-email">Email</label>
        <input
          id="auth-email"
          class="field-input"
          type="email"
          bind:value={email}
          placeholder="you@company.com"
          autocomplete="email"
          onkeydown={handleKeydown}
        />
      </div>

      {#if mode !== 'reset'}
        <div class="field">
          <div class="field-label-row">
            <label class="field-label" for="auth-password">Password</label>
            {#if mode === 'signin'}
              <button type="button" class="link-btn" onclick={() => switchMode('reset')}>
                Forgot password?
              </button>
            {/if}
          </div>
          <input
            id="auth-password"
            class="field-input"
            type="password"
            bind:value={password}
            placeholder={mode === 'signup' ? 'At least 12 characters' : 'Your password'}
            autocomplete={mode === 'signup' ? 'new-password' : 'current-password'}
            onkeydown={handleKeydown}
          />
        </div>
      {/if}

      {#if auth.error}
        <p class="error-msg">{auth.error}</p>
      {/if}

      <button class="primary-btn" type="submit" disabled={disabled()}>
        {auth.loading ? 'Please wait…' : submitLabels[mode]}
      </button>
    </form>

    <div class="divider"><span>or</span></div>

    <button
      type="button"
      class="social-btn"
      onclick={() => auth.loginBrowser('google-oauth2')}
      disabled={auth.loading}
      aria-label="Continue with Google"
    >
      <svg class="social-icon" viewBox="0 0 18 18" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
        <path fill="#4285F4" d="M17.64 9.205c0-.639-.057-1.252-.164-1.841H9v3.481h4.844a4.14 4.14 0 0 1-1.796 2.716v2.259h2.908c1.702-1.567 2.684-3.875 2.684-6.615z"/>
        <path fill="#34A853" d="M9 18c2.43 0 4.467-.806 5.956-2.18l-2.908-2.259c-.806.54-1.837.86-3.048.86-2.344 0-4.328-1.584-5.036-3.711H.957v2.332A8.997 8.997 0 0 0 9 18z"/>
        <path fill="#FBBC05" d="M3.964 10.71A5.41 5.41 0 0 1 3.682 9c0-.593.102-1.17.282-1.71V4.958H.957A8.996 8.996 0 0 0 0 9c0 1.452.348 2.827.957 4.042l3.007-2.332z"/>
        <path fill="#EA4335" d="M9 3.58c1.321 0 2.508.454 3.44 1.345l2.582-2.58C13.463.891 11.426 0 9 0A8.997 8.997 0 0 0 .957 4.958L3.964 7.29C4.672 5.163 6.656 3.58 9 3.58z"/>
      </svg>
      <span>Continue with Google</span>
    </button>

    <p class="switch-row">
      {#if mode === 'signin'}
        New to Silvie?
        <button type="button" class="link-btn" onclick={() => switchMode('signup')}>
          Create an account
        </button>
      {:else if mode === 'signup'}
        Already have an account?
        <button type="button" class="link-btn" onclick={() => switchMode('signin')}>
          Sign in
        </button>
      {:else}
        <button type="button" class="link-btn" onclick={() => switchMode('signin')}>
          Back to sign in
        </button>
      {/if}
    </p>
  </div>
</div>

<style>
  .login-screen {
    position: fixed;
    inset: 0;
    background: var(--bg);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    overflow-y: auto;
  }

  .login-card {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 14px;
    padding: 40px 36px;
    width: 100%;
    max-width: 380px;
  }

  .brand-wrap {
    display: flex;
    justify-content: center;
  }

  .title {
    font-size: 22px;
    font-weight: 700;
    letter-spacing: -0.02em;
    color: var(--text-primary);
    margin: 8px 0 0;
    text-align: center;
  }

  .subtitle {
    font-size: 13px;
    color: var(--text-muted);
    margin: 0 0 12px;
    text-align: center;
    line-height: 1.5;
  }

  .form {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .field-label-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .field-label {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .field-input {
    padding: 10px 12px;
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

  .primary-btn {
    width: 100%;
    padding: 12px;
    border-radius: 10px;
    font-size: 14px;
    font-weight: 600;
    font-family: inherit;
    cursor: pointer;
    border: none;
    background: var(--purple-600);
    color: #fff;
    transition: background 0.15s;
    margin-top: 4px;
  }

  .primary-btn:hover:not(:disabled) {
    background: var(--purple-800);
  }

  .primary-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .social-btn {
    width: 100%;
    padding: 10px 12px;
    border-radius: 10px;
    font-size: 14px;
    font-weight: 500;
    font-family: inherit;
    cursor: pointer;
    border: 1px solid var(--border-strong);
    background: var(--bg);
    color: var(--text-primary);
    transition: background 0.15s, border-color 0.15s;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
  }

  .social-btn:hover:not(:disabled) {
    background: var(--surface-hover);
    border-color: var(--purple-400);
  }

  .social-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .social-icon {
    width: 18px;
    height: 18px;
    flex-shrink: 0;
  }

  .divider {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 4px 0;
    color: var(--text-dim);
    font-size: 11px;
    font-weight: 500;
    letter-spacing: 0.05em;
    text-transform: uppercase;
  }

  .divider::before,
  .divider::after {
    content: '';
    flex: 1;
    height: 1px;
    background: var(--border);
  }

  .link-btn {
    background: none;
    border: none;
    padding: 0;
    margin: 0;
    color: var(--purple-600);
    font-family: inherit;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    text-decoration: none;
  }

  .link-btn:hover {
    color: var(--purple-800);
    text-decoration: underline;
  }

  .switch-row {
    font-size: 13px;
    color: var(--text-muted);
    text-align: center;
    margin: 8px 0 0;
  }

  .switch-row .link-btn {
    font-size: 13px;
    margin-left: 4px;
  }

  .error-msg {
    font-size: 13px;
    color: var(--error);
    padding: 8px 12px;
    border: 1px solid var(--error);
    border-radius: 8px;
    background: #fff0f0;
    margin: 0;
  }

  .success-banner {
    font-size: 13px;
    color: var(--success);
    padding: 10px 12px;
    border: 1px solid var(--success);
    border-radius: 8px;
    background: #f0fdf4;
    line-height: 1.5;
  }
</style>
