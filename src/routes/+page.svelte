<script lang="ts">
  import { conversations } from '$lib/stores/conversations.svelte';
  import { streamChat, type ChatMessage } from '$lib/services/chat';
  import { getGoogleAccessToken } from '$lib/services/connectors';
  import { getStoredPaymentMethod } from '$lib/services/payment';
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
    const storedPm = await getStoredPaymentMethod();

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
      history,
      (chunk) => {
        conversations.appendToAssistantMessage(assistantId, chunk);
        scrollToBottom();
      },
      {
        googleAccessToken,
        timezone,
        currentDatetime,
        stripeCustomerId: storedPm?.customer_id ?? null,
        stripePaymentMethodId: storedPm?.payment_method_id ?? null,
      },
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
    background: var(--border-strong);
    border-radius: 3px;
  }
</style>
