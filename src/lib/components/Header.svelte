<script lang="ts">
  import BrandMark from './BrandMark.svelte';

  let {
    onMenuClick,
    onUserClick,
  }: {
    onMenuClick: () => void;
    onUserClick: () => void;
  } = $props();
</script>

<header class="header">
  <button class="icon-btn" aria-label="Open menu" onclick={onMenuClick}>
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
      <line x1="3" y1="6" x2="21" y2="6" />
      <line x1="3" y1="12" x2="21" y2="12" />
      <line x1="3" y1="18" x2="21" y2="18" />
    </svg>
  </button>

  <div class="brand">
    <BrandMark size={22} radius={5} />
    <span class="wordmark">Silvie</span>
  </div>

  <button class="icon-btn account-btn" aria-label="Account" onclick={onUserClick}>
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <circle cx="12" cy="8" r="4" />
      <path d="M4 20c0-4 3.6-7 8-7s8 3 8 7" />
    </svg>
  </button>
</header>

<style>
  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 16px;
    height: 52px;
    background: var(--bg);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .wordmark {
    font-size: 15px;
    font-weight: 600;
    letter-spacing: 0.02em;
    color: var(--text-primary);
  }

  .icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    border-radius: 8px;
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }

  .icon-btn:hover {
    background: var(--surface-hover);
    color: var(--purple-600);
  }

  .account-btn {
    border: 1.5px solid var(--border-strong);
    border-radius: 50%;
  }

  .account-btn:hover {
    border-color: var(--purple-400);
  }
</style>
