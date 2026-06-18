<script lang="ts">
  let {
    suggestions,
    onSuggestionClick,
  }: {
    suggestions: string[];
    onSuggestionClick: (text: string) => void;
  } = $props();
</script>

<div class="empty-state">
  <div class="silvie-glyph">S</div>
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

  .silvie-glyph {
    width: 56px;
    height: 56px;
    border-radius: 16px;
    background: linear-gradient(135deg, #7c5cfc, #4f8ef7);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 26px;
    font-weight: 700;
    color: #fff;
    margin-bottom: 20px;
    flex-shrink: 0;
  }

  .empty-title {
    font-size: 22px;
    font-weight: 600;
    color: #ffffff;
    margin-bottom: 8px;
  }

  .empty-sub {
    font-size: 14px;
    color: #666;
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
    border: 1px solid #2a2a2a;
    background: #141414;
    color: #aaa;
    border-radius: 20px;
    font-size: 13px;
    cursor: pointer;
    transition: border-color 0.15s, color 0.15s, background 0.15s;
    text-align: left;
    font-family: inherit;
  }

  .suggestion-chip:hover {
    border-color: #555;
    color: #e8e8e8;
    background: #1f1f1f;
  }
</style>
