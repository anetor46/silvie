<script lang="ts">
  import type { Message } from '$lib/types';

  let { message }: { message: Message } = $props();
</script>

<div class="message {message.role}">
  {#if message.role === 'assistant'}
    <div class="avatar">S</div>
  {/if}
  <div class="bubble">{message.content}</div>
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
</style>
