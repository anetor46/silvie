<script lang="ts">
  import type { Provider } from '$lib/data/connectors';

  let {
    provider,
    connected = $bindable(false),
  }: {
    provider: Provider;
    connected?: boolean;
  } = $props();

  const initials = $derived(
    provider.name
      .split(/[\s.]/)
      .filter(Boolean)
      .slice(0, 2)
      .map((w) => w[0].toUpperCase())
      .join(''),
  );
</script>

<div class="card">
  <div
    class="provider-icon"
    style="background: {provider.color}; color: {provider.textColor ?? '#fff'}"
  >
    {initials}
  </div>

  <span class="provider-name">{provider.name}</span>

  {#if connected}
    <span class="status-dot" aria-hidden="true"></span>
    <button class="btn disconnect" onclick={() => (connected = false)}>Disconnect</button>
  {:else}
    <button class="btn connect" onclick={() => (connected = true)}>Connect</button>
  {/if}
</div>

<style>
  .card {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 14px 16px;
    background: #141414;
    border: 1px solid #1f1f1f;
    border-radius: 12px;
    transition: border-color 0.15s;
  }

  .card:hover {
    border-color: #2a2a2a;
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

  .provider-name {
    flex: 1;
    font-size: 14px;
    color: #d4d4d4;
    font-weight: 500;
  }

  .status-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: #4ade80;
    flex-shrink: 0;
  }

  .btn {
    padding: 6px 14px;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 500;
    font-family: inherit;
    cursor: pointer;
    transition: opacity 0.15s, background 0.15s;
    border: none;
    flex-shrink: 0;
  }

  .btn.connect {
    background: linear-gradient(135deg, #7c5cfc, #4f8ef7);
    color: #fff;
  }

  .btn.connect:hover {
    opacity: 0.85;
  }

  .btn.disconnect {
    background: #1f1f1f;
    color: #888;
    border: 1px solid #2a2a2a;
  }

  .btn.disconnect:hover {
    background: #2a2a2a;
    color: #ccc;
  }
</style>
