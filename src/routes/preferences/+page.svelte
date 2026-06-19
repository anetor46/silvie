<script lang="ts">
  import { PREFERENCE_CATEGORIES } from '$lib/data/preferences';
  import PreferenceRow from '$lib/components/PreferenceRow.svelte';

  // One selected provider id per category — null means "no preference yet".
  const initialState: Record<string, string | null> = {};
  for (const cat of PREFERENCE_CATEGORIES) {
    initialState[cat.id] = null;
  }
  let selections = $state<Record<string, string | null>>(initialState);
</script>

<div class="page">
  <div class="page-header">
    <h1 class="title">Preferences</h1>
    <p class="subtitle">
      Pick the booking app you'd like Silvie to use when it suggests a ride, a stay, or a table.
    </p>
  </div>

  <div class="categories">
    {#each PREFERENCE_CATEGORIES as category (category.id)}
      <PreferenceRow {category} bind:selected={selections[category.id]} />
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
    margin-bottom: 12px;
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
    max-width: 600px;
  }
</style>
