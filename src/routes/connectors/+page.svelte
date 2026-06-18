<script lang="ts">
  import { CATEGORIES } from '$lib/data/connectors';
  import ConnectorCard from '$lib/components/ConnectorCard.svelte';

  // Track connected state per provider id — initialise every provider to false
  // so bind:connected never receives undefined (Svelte 5 forbids it).
  const initialState: Record<string, boolean> = {};
  for (const category of CATEGORIES) {
    for (const provider of category.providers) {
      initialState[provider.id] = false;
    }
  }
  let connected = $state<Record<string, boolean>>(initialState);
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
            <ConnectorCard
              {provider}
              bind:connected={connected[provider.id]}
            />
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
    background: #2a2a2a;
    border-radius: 3px;
  }

  .page-header {
    margin-bottom: 36px;
    max-width: 600px;
  }

  .title {
    font-size: 24px;
    font-weight: 700;
    color: #ffffff;
    margin-bottom: 6px;
  }

  .subtitle {
    font-size: 14px;
    color: #555;
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
    color: #444;
    margin-bottom: 10px;
  }

  .provider-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
</style>
