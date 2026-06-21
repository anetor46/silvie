<script lang="ts">
  import type { Provider } from '$lib/data/connectors';

  let {
    provider,
    connectedEmail = $bindable<string | null>(null),
    loading = false,
    error = $bindable<string | null>(null),
    onConnect,
    onDisconnect,
  }: {
    provider: Provider;
    connectedEmail?: string | null;
    loading?: boolean;
    error?: string | null;
    onConnect?: () => Promise<void>;
    onDisconnect?: () => Promise<void>;
  } = $props();

  const initials = $derived(
    provider.name
      .split(/[\s.]/)
      .filter(Boolean)
      .slice(0, 2)
      .map((w) => w[0].toUpperCase())
      .join(''),
  );

  const isConnected = $derived(connectedEmail != null);
  const isOAuth = $derived(provider.requiresOAuth === true);

  async function handleConnect() {
    if (onConnect) await onConnect();
  }

  async function handleDisconnect() {
    if (onDisconnect) await onDisconnect();
  }
</script>

<div class="card">
  <div
    class="provider-icon"
    style="background: {provider.color}; color: {provider.textColor ?? '#fff'}"
  >
    {initials}
  </div>

  <div class="provider-info">
    <span class="provider-name">{provider.name}</span>
    {#if isConnected && connectedEmail}
      <span class="provider-email">{connectedEmail}</span>
    {:else if provider.features.length > 0}
      <span class="provider-features">{provider.features.join(' · ')}</span>
    {/if}
  </div>

  {#if isOAuth}
    {#if isConnected}
      <span class="status-dot" aria-hidden="true"></span>
      <button class="btn disconnect" onclick={handleDisconnect} disabled={loading}>
        {#if loading}
          <span class="spinner" aria-hidden="true"></span>
        {:else}
          Disconnect
        {/if}
      </button>
    {:else}
      <button class="btn connect" onclick={handleConnect} disabled={loading}>
        {#if loading}
          <span class="spinner" aria-hidden="true"></span>
        {:else}
          Connect
        {/if}
      </button>
    {/if}
  {:else}
    <span class="coming-soon">Coming soon</span>
  {/if}
</div>

{#if error}
  <p class="error-msg">{error}</p>
{/if}

<style>
  .card {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 14px 16px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 12px;
    transition: border-color 0.15s, background 0.15s;
  }

  .card:hover {
    border-color: var(--border-strong);
    background: var(--surface);
  }

  .provider-icon {
    width: 38px;
    height: 38px;
    border-radius: 9px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 12px;
    font-weight: 700;
    flex-shrink: 0;
    letter-spacing: 0.02em;
  }

  .provider-info {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .provider-name {
    font-size: 14px;
    color: var(--text-primary);
    font-weight: 500;
  }

  .provider-email {
    font-size: 12px;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .provider-features {
    font-size: 12px;
    color: var(--text-muted);
  }

  .status-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--success);
    flex-shrink: 0;
  }

  .coming-soon {
    font-size: 12px;
    color: var(--text-dim);
    font-style: italic;
    flex-shrink: 0;
  }

  .btn {
    padding: 6px 14px;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 500;
    font-family: inherit;
    cursor: pointer;
    transition: opacity 0.15s, background 0.15s, border-color 0.15s, color 0.15s;
    border: none;
    flex-shrink: 0;
    min-width: 88px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .btn.connect {
    background: var(--purple-600);
    color: #fff;
  }

  .btn.connect:hover:not(:disabled) {
    background: var(--purple-800);
  }

  .btn.disconnect {
    background: var(--bg);
    color: var(--text-secondary);
    border: 1px solid var(--border);
  }

  .btn.disconnect:hover:not(:disabled) {
    background: var(--surface);
    border-color: var(--border-strong);
    color: var(--text-primary);
  }

  .spinner {
    display: inline-block;
    width: 13px;
    height: 13px;
    border: 2px solid currentColor;
    border-top-color: transparent;
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
    opacity: 0.7;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .error-msg {
    font-size: 12px;
    color: var(--error);
    margin-top: 6px;
    padding: 0 4px;
  }
</style>
