<script lang="ts">
  import { conversations } from '$lib/stores/conversations.svelte';
  import { streamChat } from '$lib/services/chat';
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

    // Abort any in-flight generation.
    currentStream?.cancel();

    // Lazily create the conversation on the backend if this is a fresh chat.
    const isFirstMessage = !conversations.currentId;
    let conversationId: string;
    try {
      conversationId = await conversations.ensureConversation();
    } catch (err) {
      console.error('[chat] could not create conversation', err);
      return;
    }

    conversations.appendUserMessage(text);
    inputValue = '';
    scrollToBottom();

    const assistantId = conversations.startAssistantMessage();
    scrollToBottom();

    const now = new Date();
    const timezone = Intl.DateTimeFormat().resolvedOptions().timeZone;
    const offsetMin = -now.getTimezoneOffset();
    const sign = offsetMin >= 0 ? '+' : '-';
    const pad = (n: number) => String(n).padStart(2, '0');
    const hh = pad(Math.floor(Math.abs(offsetMin) / 60));
    const mm = pad(Math.abs(offsetMin) % 60);
    const currentDatetime = new Date(now.getTime() - now.getTimezoneOffset() * 60_000)
      .toISOString()
      .replace('Z', `${sign}${hh}:${mm}`);

    const handle = streamChat(
      conversationId,
      text,
      (chunk) => {
        conversations.appendToAssistant(assistantId, chunk);
        scrollToBottom();
      },
      { timezone, currentDatetime },
    );
    currentStream = handle;

    handle.done
      .catch((err: unknown) => {
        if (err instanceof DOMException && err.name === 'AbortError') return;
        const message = err instanceof Error ? err.message : String(err);
        conversations.appendToAssistant(assistantId, `\n\n_⚠️ ${message}_`);
      })
      .finally(() => {
        if (currentStream === handle) currentStream = null;
        // First-message titles are auto-generated server-side — pull the
        // updated sidebar row so the title appears immediately.
        if (isFirstMessage) {
          void conversations.refreshCurrentInList();
        }
      });
  }

  function handleSuggestion(text: string) {
    inputValue = text;
  }
</script>

<main class="chat-area" bind:this={messagesEl}>
  {#if conversations.currentMessages.length === 0}
    <EmptyState {suggestions} onSuggestionClick={handleSuggestion} />
  {:else}
    <MessageList messages={conversations.currentMessages} />
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
    background: var(--border-strong);
    border-radius: 3px;
  }
</style>
