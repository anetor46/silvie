<script lang="ts">
  import { tick } from 'svelte';

  let {
    value = $bindable(''),
    onSend,
  }: {
    value?: string;
    onSend: () => void;
  } = $props();

  let textarea = $state<HTMLTextAreaElement | undefined>(undefined);

  function autoResize() {
    if (!textarea) return;
    // Reset first so shrinking works when lines are deleted.
    textarea.style.height = 'auto';
    textarea.style.height = `${textarea.scrollHeight}px`;
  }

  // Re-run whenever value changes — catches both typing and programmatic
  // resets (e.g. value = '' after send).
  $effect(() => {
    void value;
    tick().then(autoResize);
  });

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      onSend();
    }
  }
</script>

<footer class="input-bar">
  <div class="input-wrapper">
    <textarea
      bind:this={textarea}
      bind:value
      oninput={autoResize}
      onkeydown={handleKeydown}
      placeholder="Message Silvie…"
      rows="1"
      class="input"
    ></textarea>
    <button
      class="send-btn"
      onclick={onSend}
      disabled={!value.trim()}
      aria-label="Send"
    >
      <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
        <path d="M2.01 21L23 12 2.01 3 2 10l15 2-15 2z" />
      </svg>
    </button>
  </div>

  <p class="disclaimer">Silvie can make mistakes. Verify important travel details.</p>
</footer>

<style>
  .input-bar {
    padding: 12px 16px 16px;
    flex-shrink: 0;
    background: var(--bg);
    border-top: 1px solid var(--border);
  }

  .input-wrapper {
    position: relative;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 14px;
    /*
     * No right padding: the textarea fills the full width so its native
     * scrollbar lands flush against this border. The textarea's own
     * padding-right reserves the space for the send button + gap.
     */
    padding: 10px 0 10px 12px;
    max-width: 720px;
    margin: 0 auto;
    transition: border-color 0.15s, background 0.15s;
  }

  .input-wrapper:focus-within {
    border-color: var(--purple-400);
    background: var(--bg);
  }

  .input {
    display: block;
    width: 100%;
    background: transparent;
    border: none;
    outline: none;
    color: var(--text-primary);
    font-size: 14px;
    font-family: inherit;
    line-height: 1.6;
    resize: none;
    /* Match button height so the wrapper has room for the absolutely-
       positioned send button (10px top + 32px btn + 10px bottom). */
    min-height: 32px;
    max-height: calc(50dvh - 80px);
    overflow-y: auto;
    /*
     * right padding = button-right-offset(10px) + button-width(32px) +
     *                 gap(6px) + scrollbar-width(4px) = 52px.
     * This keeps text from flowing under the button or the scrollbar.
     */
    padding-right: 52px;
    scrollbar-width: thin;
    scrollbar-color: var(--border-strong) transparent;
  }

  .input::-webkit-scrollbar {
    width: 4px;
  }

  .input::-webkit-scrollbar-track {
    background: transparent;
  }

  .input::-webkit-scrollbar-thumb {
    background: var(--border-strong);
    border-radius: 2px;
  }

  .input::placeholder {
    color: var(--text-dim);
  }

  .send-btn {
    position: absolute;
    right: 10px;
    bottom: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border: none;
    border-radius: 8px;
    background: var(--purple-600);
    color: #fff;
    cursor: pointer;
    transition: background 0.15s, opacity 0.15s;
  }

  .send-btn:disabled {
    opacity: 0.3;
    cursor: default;
  }

  .send-btn:not(:disabled):hover {
    background: var(--purple-800);
  }

  .disclaimer {
    text-align: center;
    font-size: 11px;
    color: var(--text-dim);
    margin-top: 8px;
    max-width: 720px;
    margin-left: auto;
    margin-right: auto;
  }
</style>
