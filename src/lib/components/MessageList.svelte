<script lang="ts">
  import type { Message } from '$lib/types';
  import MessageItem from './Message.svelte';

  let { messages }: { messages: Message[] } = $props();
</script>

<div class="message-list">
  {#each messages as msg (msg.id)}
    <MessageItem message={msg} />
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
</style>
