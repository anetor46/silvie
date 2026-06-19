<script lang="ts">
  import BrandMark from './BrandMark.svelte';

  let {
    suggestions,
    onSuggestionClick,
  }: {
    suggestions: string[];
    onSuggestionClick: (text: string) => void;
  } = $props();
</script>

<div class="empty-state">
  <div class="brand-glyph">
    <BrandMark size={56} radius={14} />
  </div>
  <h1 class="empty-title">Good to have you back.</h1>
  <p class="empty-sub">Ask me anything about your trips, meetings, or logistics.</p>
  <div class="suggestions">
    {#each suggestions as s}
      <button class="suggestion-chip" onclick={() => onSuggestionClick(s)}>
        {s}
      </button>
    {/each}
  </div>
</div>

<style>
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    padding: 40px 24px;
    text-align: center;
  }

  .brand-glyph {
    margin-bottom: 20px;
    flex-shrink: 0;
    line-height: 0;
  }

  .empty-title {
    font-size: 22px;
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 8px;
  }

  .empty-sub {
    font-size: 14px;
    color: var(--text-muted);
    margin-bottom: 28px;
    max-width: 320px;
  }

  .suggestions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    justify-content: center;
    max-width: 480px;
  }

  .suggestion-chip {
    padding: 8px 14px;
    border: 1px solid var(--border);
    background: var(--bg);
    color: var(--text-secondary);
    border-radius: 20px;
    font-size: 13px;
    cursor: pointer;
    transition: border-color 0.15s, color 0.15s, background 0.15s;
    text-align: left;
    font-family: inherit;
  }

  .suggestion-chip:hover {
    border-color: var(--purple-400);
    color: var(--purple-600);
    background: var(--purple-50);
  }
</style>
