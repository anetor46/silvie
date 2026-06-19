<script lang="ts">
  import { CATEGORIES } from '$lib/data/connectors';
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

  <div class="categories">
    {#each CATEGORIES as category (category.id)}
      <section class="category">
        <h2 class="category-label">{category.label}</h2>
        <div class="provider-list">
          {#each category.providers as provider (provider.id)}
            {#if provider.id === 'google-calendar'}
              <ConnectorCard
                {provider}
                connectedEmail={connectors.googleCalendar?.email ?? null}
                loading={connectors.googleCalendarLoading}
                error={connectors.googleCalendarError}
                onConnect={() => connectors.connectGoogleCalendar()}
                onDisconnect={() => connectors.disconnectGoogleCalendar()}
              />
            {:else}
              <ConnectorCard {provider} />
            {/if}
          {/each}
        </div>
      </section>
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

  .categories {
    display: flex;
    flex-direction: column;
    gap: 32px;
    max-width: 600px;
  }

  .category-label {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--purple-600);
    margin-bottom: 10px;
  }

  .provider-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
</style>
