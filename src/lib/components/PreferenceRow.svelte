<script lang="ts">
  import type { PreferenceCategory } from '$lib/data/preferences';

  let {
    category,
    selected = $bindable<string | null>(null),
  }: {
    category: PreferenceCategory;
    selected?: string | null;
  } = $props();
</script>

<section class="row">
  <div class="meta">
    <h2 class="label">{category.label}</h2>
    <p class="desc">{category.description}</p>
  </div>

  <div class="options" role="radiogroup" aria-label={category.label}>
    {#each category.providers as provider (provider.id)}
      {@const active = selected === provider.id}
      <button
        type="button"
        class="chip"
        class:active
        style:--brand={provider.color}
        style:--brand-fg={provider.textColor ?? '#ffffff'}
        role="radio"
        aria-checked={active}
        onclick={() => (selected = active ? null : provider.id)}
      >
        {provider.name}
      </button>
    {/each}
  </div>
</section>

<style>
  .row {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 18px 0;
    border-bottom: 1px solid #1a1a1a;
  }

  .row:last-child {
    border-bottom: none;
  }

  .label {
    font-size: 14px;
    font-weight: 600;
    color: #e8e8e8;
    margin-bottom: 2px;
  }

  .desc {
    font-size: 12px;
    color: #555;
  }

  .options {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .chip {
    padding: 7px 14px;
    border-radius: 999px;
    font-size: 13px;
    font-weight: 500;
    font-family: inherit;
    background: #141414;
    color: #aaa;
    border: 1px solid #2a2a2a;
    cursor: pointer;
    transition: border-color 0.15s, color 0.15s, background 0.15s;
  }

  .chip:hover {
    border-color: #555;
    color: #e8e8e8;
  }

  .chip.active {
    background: var(--brand);
    color: var(--brand-fg);
    border-color: var(--brand);
  }
</style>
