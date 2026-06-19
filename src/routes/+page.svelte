<script lang="ts">
  import { conversations } from '$lib/stores/conversations.svelte';
  import { streamChat, type ChatMessage } from '$lib/services/chat';
  import { getGoogleAccessToken } from '$lib/services/connectors';
  import EmptyState from '$lib/components/EmptyState.svelte';
  import MessageList from '$lib/components/MessageList.svelte';
  import InputBar from '$lib/components/InputBar.svelte';

  let inputValue = $state('');
  let messagesEl = $state<HTMLElement | undefined>(undefined);
  let currentStream: { cancel: () => void } | null = null;

  const suggestions = [
    "What's on my schedule today?",
    'Any flights I need to check in for?',
    'Summarise my trip to Paris next week',
    'Do I have a hotel booked for London?',
  ];

  function scrollToBottom() {
    setTimeout(() => messagesEl?.scrollTo({ top: messagesEl.scrollHeight, behavior: 'smooth' }), 10);
  }

  async function sendMessage() {
    const text = inputValue.trim();
    if (!text) return;

    // If a previous generation is still running, abort it.
    currentStream?.cancel();

    conversations.sendUserMessage(text);
    inputValue = '';
    scrollToBottom();

    const history: ChatMessage[] =
      conversations.active?.messages.map((m) => ({ role: m.role, content: m.content })) ?? [];

    const assistantId = conversations.startAssistantMessage();
    scrollToBottom();

    const googleAccessToken = await getGoogleAccessToken();

    const handle = streamChat(
      history,
      (chunk) => {
        conversations.appendToAssistantMessage(assistantId, chunk);
        scrollToBottom();
      },
      { googleAccessToken },
    );
    currentStream = handle;

    handle.done
      .catch((err: unknown) => {
        if (err instanceof DOMException && err.name === 'AbortError') return;
        const message = err instanceof Error ? err.message : String(err);
        conversations.appendToAssistantMessage(
          assistantId,
          `\n\n_⚠️ ${message}_`,
        );
      })
      .finally(() => {
        if (currentStream === handle) currentStream = null;
      });
  }

  function handleSuggestion(text: string) {
    inputValue = text;
  }
</script>

<main class="chat-area" bind:this={messagesEl}>
  {#if !conversations.active || conversations.active.messages.length === 0}
    <EmptyState {suggestions} onSuggestionClick={handleSuggestion} />
  {:else}
    <MessageList messages={conversations.active.messages} />
  {/if}
</main>

<InputBar bind:value={inputValue} onSend={sendMessage} />

<style>
  .chat-area {
    flex: 1;
    overflow-y: auto;
    scroll-behavior: smooth;
  }

  .chat-area::-webkit-scrollbar {
    width: 6px;
  }

  .chat-area::-webkit-scrollbar-track {
    background: transparent;
  }

  .chat-area::-webkit-scrollbar-thumb {
    background: #2a2a2a;
    border-radius: 3px;
  }
</style>
