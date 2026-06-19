<script lang="ts">
  import type { Message } from '$lib/types';
  import { marked } from 'marked';
  import BrandMark from './BrandMark.svelte';

  let { message }: { message: Message } = $props();
</script>

<div class="message {message.role}">
  {#if message.role === 'assistant'}
    <div class="avatar">
      <BrandMark size={30} radius={7} />
    </div>
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
    flex-shrink: 0;
    margin-top: 2px;
    line-height: 0;
  }

  .bubble {
    max-width: 80%;
    line-height: 1.6;
    font-size: 14px;
  }

  .message.user .bubble {
    background: var(--purple-50);
    border: 1px solid var(--border);
    padding: 10px 14px;
    border-radius: 16px 4px 16px 16px;
    color: var(--text-primary);
  }

  .message.assistant .bubble {
    color: var(--text-primary);
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
    color: var(--text-primary);
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
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 1px 5px;
    color: var(--purple-800);
  }

  .message.assistant .bubble :global(pre) {
    background: var(--surface);
    border: 1px solid var(--border);
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
    color: var(--text-primary);
    line-height: 1.5;
  }

  .message.assistant .bubble :global(blockquote) {
    border-left: 3px solid var(--purple-400);
    margin: 0.5em 0;
    padding: 0.25em 0 0.25em 1em;
    color: var(--text-secondary);
  }

  .message.assistant .bubble :global(hr) {
    border: none;
    border-top: 1px solid var(--border);
    margin: 1em 0;
  }

  .message.assistant .bubble :global(strong) {
    color: var(--text-primary);
    font-weight: 600;
  }

  .message.assistant .bubble :global(a) {
    color: var(--purple-600);
    text-decoration: none;
  }
  .message.assistant .bubble :global(a:hover) {
    text-decoration: underline;
  }

  .message.assistant .bubble :global(del) {
    color: var(--text-dim);
  }
</style>
