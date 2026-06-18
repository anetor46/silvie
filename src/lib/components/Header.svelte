<script lang="ts">
  let { onMenuClick }: { onMenuClick: () => void } = $props();
</script>

<header class="header">
  <button class="icon-btn" aria-label="Open menu" onclick={onMenuClick}>
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
      <line x1="3" y1="6" x2="21" y2="6" />
      <line x1="3" y1="12" x2="21" y2="12" />
      <line x1="3" y1="18" x2="21" y2="18" />
    </svg>
  </button>

  <span class="wordmark">Silvie</span>

  <button class="icon-btn account-btn" aria-label="Account">
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
    border-bottom: 1px solid #1f1f1f;
    flex-shrink: 0;
  }

  .wordmark {
    font-size: 15px;
    font-weight: 600;
    letter-spacing: 0.04em;
    color: #ffffff;
  }

  .icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    border: none;
    background: transparent;
    color: #888;
    border-radius: 8px;
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }

  .icon-btn:hover {
    background: #1f1f1f;
    color: #e8e8e8;
  }

  .account-btn {
    border: 1.5px solid #2a2a2a;
    border-radius: 50%;
  }

  .account-btn:hover {
    border-color: #444;
  }
</style>
