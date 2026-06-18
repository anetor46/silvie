<script lang="ts">
  import type { Message } from '$lib/types';
  import { marked } from 'marked';

  let { message }: { message: Message } = $props();
</script>

<div class="message {message.role}">
  {#if message.role === 'assistant'}
    <div class="avatar">S</div>
  {/if}
  <div class="bubble">
    {#if message.role === 'assistant'}
      {@html marked(message.content)}
    {:else}
      {message.content}
    {/if}
  </div>
</div>

<style>
  .message {
    display: flex;
    gap: 12px;
    align-items: flex-start;
  }

  .message.user {
    flex-direction: row-reverse;
  }

  .avatar {
    width: 30px;
    height: 30px;
    border-radius: 8px;
    background: linear-gradient(135deg, #7c5cfc, #4f8ef7);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 13px;
    font-weight: 700;
    color: #fff;
    flex-shrink: 0;
    margin-top: 2px;
  }

  .bubble {
    max-width: 80%;
    line-height: 1.6;
    font-size: 14px;
  }

  .message.user .bubble {
    background: #1f1f1f;
    border: 1px solid #2a2a2a;
    padding: 10px 14px;
    border-radius: 16px 4px 16px 16px;
    color: #e8e8e8;
  }

  .message.assistant .bubble {
    color: #d4d4d4;
    padding: 4px 0;
  }

  /* ── Markdown rendered content ── */
  .message.assistant .bubble :global(p) {
    margin: 0 0 0.75em;
  }
  .message.assistant .bubble :global(p:last-child) {
    margin-bottom: 0;
  }

  .message.assistant .bubble :global(h1),
  .message.assistant .bubble :global(h2),
  .message.assistant .bubble :global(h3),
  .message.assistant .bubble :global(h4),
  .message.assistant .bubble :global(h5),
  .message.assistant .bubble :global(h6) {
    color: #e8e8e8;
    font-weight: 600;
    margin: 1em 0 0.4em;
    line-height: 1.3;
  }
  .message.assistant .bubble :global(h1) { font-size: 1.25em; }
  .message.assistant .bubble :global(h2) { font-size: 1.1em; }
  .message.assistant .bubble :global(h3) { font-size: 1em; }

  .message.assistant .bubble :global(ul),
  .message.assistant .bubble :global(ol) {
    padding-left: 1.4em;
    margin: 0.5em 0;
  }
  .message.assistant .bubble :global(li) {
    margin: 0.25em 0;
  }

  .message.assistant .bubble :global(code) {
    font-family: 'Menlo', 'Consolas', monospace;
    font-size: 0.85em;
    background: #1a1a1a;
    border: 1px solid #2e2e2e;
    border-radius: 4px;
    padding: 1px 5px;
    color: #c9d1d9;
  }

  .message.assistant .bubble :global(pre) {
    background: #141414;
    border: 1px solid #2e2e2e;
    border-radius: 8px;
    padding: 12px 14px;
    margin: 0.75em 0;
    overflow-x: auto;
  }
  .message.assistant .bubble :global(pre code) {
    background: none;
    border: none;
    padding: 0;
    font-size: 0.82em;
    color: #c9d1d9;
    line-height: 1.5;
  }

  .message.assistant .bubble :global(blockquote) {
    border-left: 3px solid #7c5cfc;
    margin: 0.5em 0;
    padding: 0.25em 0 0.25em 1em;
    color: #9ca3af;
  }

  .message.assistant .bubble :global(hr) {
    border: none;
    border-top: 1px solid #2a2a2a;
    margin: 1em 0;
  }

  .message.assistant .bubble :global(strong) {
    color: #e8e8e8;
    font-weight: 600;
  }

  .message.assistant .bubble :global(a) {
    color: #7c5cfc;
    text-decoration: none;
  }
  .message.assistant .bubble :global(a:hover) {
    text-decoration: underline;
  }

  .message.assistant .bubble :global(del) {
    color: #6b7280;
  }
</style>
