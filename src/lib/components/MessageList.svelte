<script lang="ts">
  import type { Message } from '$lib/types';
  import MessageItem from './Message.svelte';
  import ToolCallCard from './ToolCallCard.svelte';
  import ConfirmationWidget from './ConfirmationWidget.svelte';

  let { messages }: { messages: Message[] } = $props();
</script>

<div class="message-list">
  {#each messages as msg (msg.id)}
    {#if msg.role === 'tool' && msg.toolCall}
      <div class="tool-block">
        <ToolCallCard toolCall={msg.toolCall} />
        {#if msg.toolCall.requiresConfirmation && msg.toolCall.status === 'pending_user' && !msg.toolCall.decision}
          <ConfirmationWidget toolCall={msg.toolCall} />
        {/if}
      </div>
    {:else}
      <MessageItem message={msg} />
    {/if}
  {/each}
</div>

<style>
  .message-list {
    display: flex;
    flex-direction: column;
    gap: 24px;
    padding: 28px 16px 16px;
    max-width: 720px;
    margin: 0 auto;
    width: 100%;
  }

  .tool-block {
    display: flex;
    flex-direction: column;
    gap: 0;
  }
</style>
