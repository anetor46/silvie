<script lang="ts">
  let {
    value = $bindable(''),
    onSend,
  }: {
    value?: string;
    onSend: () => void;
  } = $props();

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
      bind:value
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
    border-top: 1px solid #1a1a1a;
  }

  .input-wrapper {
    display: flex;
    align-items: flex-end;
    gap: 10px;
    background: #141414;
    border: 1px solid #2a2a2a;
    border-radius: 14px;
    padding: 10px 12px;
    max-width: 720px;
    margin: 0 auto;
    transition: border-color 0.15s;
  }

  .input-wrapper:focus-within {
    border-color: #444;
  }

  .input {
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    color: #e8e8e8;
    font-size: 14px;
    font-family: inherit;
    line-height: 1.6;
    resize: none;
    max-height: 160px;
    overflow-y: auto;
  }

  .input::placeholder {
    color: #444;
  }

  .send-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border: none;
    border-radius: 8px;
    background: linear-gradient(135deg, #7c5cfc, #4f8ef7);
    color: #fff;
    cursor: pointer;
    flex-shrink: 0;
    transition: opacity 0.15s;
  }

  .send-btn:disabled {
    opacity: 0.25;
    cursor: default;
  }

  .send-btn:not(:disabled):hover {
    opacity: 0.85;
  }

  .disclaimer {
    text-align: center;
    font-size: 11px;
    color: #333;
    margin-top: 8px;
    max-width: 720px;
    margin-left: auto;
    margin-right: auto;
  }
</style>
