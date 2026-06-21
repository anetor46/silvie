<script lang="ts">
  import { PROVIDERS } from '$lib/data/connectors';
  import ConnectorCard from '$lib/components/ConnectorCard.svelte';
  import { connectors } from '$lib/stores/connectors.svelte';

  $effect(() => {
    connectors.load();
  });
</script>

<div class="page">
  <div class="page-header">
    <h1 class="title">Connectors</h1>
    <p class="subtitle">Link Silvie to your tools so it can keep everything in sync.</p>
  </div>

  <div class="provider-list">
    {#each PROVIDERS as provider (provider.id)}
      {#if provider.id === 'google'}
        <ConnectorCard
          {provider}
          connectedEmail={connectors.google?.provider_account_email ?? null}
          loading={connectors.googleLoading}
          error={connectors.googleError}
          onConnect={() => connectors.connectGoogle()}
          onDisconnect={() => connectors.disconnectGoogle()}
        />
      {:else if provider.id === 'outlook'}
        <ConnectorCard
          {provider}
          connectedEmail={connectors.outlook?.provider_account_email ?? null}
          loading={connectors.outlookLoading}
          error={connectors.outlookError}
          onConnect={() => connectors.connectOutlook()}
          onDisconnect={() => connectors.disconnectOutlook()}
        />
      {:else}
        <ConnectorCard {provider} />
      {/if}
    {/each}
  </div>
</div>

<style>
  .page {
    flex: 1;
    overflow-y: auto;
    padding: 32px 24px 48px;
  }

  .page::-webkit-scrollbar {
    width: 6px;
  }

  .page::-webkit-scrollbar-track {
    background: transparent;
  }

  .page::-webkit-scrollbar-thumb {
    background: var(--border-strong);
    border-radius: 3px;
  }

  .page-header {
    margin-bottom: 36px;
    max-width: 600px;
  }

  .title {
    font-size: 24px;
    font-weight: 700;
    color: var(--text-primary);
    margin-bottom: 6px;
    letter-spacing: -0.01em;
  }

  .subtitle {
    font-size: 14px;
    color: var(--text-secondary);
  }

  .provider-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
    max-width: 600px;
  }
</style>
