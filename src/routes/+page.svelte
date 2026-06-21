<script lang="ts">
  import { conversations } from '$lib/stores/conversations.svelte';
  import { streamChat, type ChatCallbacks } from '$lib/services/chat';
  import { postToolResponse, type ToolResponse } from '$lib/services/confirmations';
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
    setTimeout(
      () => messagesEl?.scrollTo({ top: messagesEl.scrollHeight, behavior: 'smooth' }),
      10,
    );
  }

  function localeContext() {
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
    return { timezone, currentDatetime };
  }

  /** Build a fresh set of callbacks that pipe SSE events into the
   *  conversation store. Tokens for the resume stream go into a *new*
   *  assistant placeholder created lazily on first token. */
  function makeCallbacks(): ChatCallbacks {
    let assistantId: string | null = null;
    return {
      onToken: (chunk) => {
        if (!assistantId) {
          assistantId = conversations.startAssistantMessage();
        }
        conversations.appendToAssistant(assistantId, chunk);
        scrollToBottom();
      },
      onToolCall: (call) => {
        // Close out any in-progress assistant text so the next text token
        // creates a new bubble below the tool card.
        assistantId = null;
        conversations.appendToolCall(call);
        scrollToBottom();
      },
      onToolResult: (result) => {
        conversations.updateToolResult(result.callId, result.success, result.summary);
        scrollToBottom();
      },
    };
  }

  async function sendMessage() {
    const text = inputValue.trim();
    if (!text) return;

    currentStream?.cancel();

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

    const handle = streamChat(conversationId, text, makeCallbacks(), localeContext());
    currentStream = handle;

    handle.done
      .catch((err: unknown) => {
        if (err instanceof DOMException && err.name === 'AbortError') return;
        const message = err instanceof Error ? err.message : String(err);
        const id = conversations.startAssistantMessage();
        conversations.appendToAssistant(id, `\n\n_⚠️ ${message}_`);
      })
      .finally(() => {
        if (currentStream === handle) currentStream = null;
        if (isFirstMessage) {
          void conversations.refreshCurrentInList();
        }
      });
  }

  /** Called by the confirmation widget when the user clicks Approve / Reject
   *  (or, in future, makes any other kind of tool-response choice). Records
   *  the local decision so the buttons collapse into a badge, then starts a
   *  new SSE stream that emits the tool result + any follow-up assistant
   *  text the resumed agent produces. */
  async function handleToolResponse(callId: string, response: ToolResponse): Promise<void> {
    currentStream?.cancel();

    if (response.kind === 'confirmation') {
      conversations.setDecision(callId, response.approved ? 'approved' : 'rejected');
    }
    scrollToBottom();

    const handle = postToolResponse(callId, response, makeCallbacks(), localeContext());
    currentStream = handle;

    await handle.done
      .catch((err: unknown) => {
        if (err instanceof DOMException && err.name === 'AbortError') return;
        const message = err instanceof Error ? err.message : String(err);
        const id = conversations.startAssistantMessage();
        conversations.appendToAssistant(id, `\n\n_⚠️ ${message}_`);
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
  {#if conversations.currentMessages.length === 0}
    <EmptyState {suggestions} onSuggestionClick={handleSuggestion} />
  {:else}
    <MessageList messages={conversations.currentMessages} onToolResponse={handleToolResponse} />
  {/if}
</main>

<InputBar bind:value={inputValue} onSend={sendMessage} />

<style>
  .chat-area {
    flex: 1;
    overflow-y: auto;
    scroll-behavior: smooth;
  }

  .chat-area::-webkit-scrollbar { width: 6px; }
  .chat-area::-webkit-scrollbar-track { background: transparent; }
  .chat-area::-webkit-scrollbar-thumb {
    background: var(--border-strong);
    border-radius: 3px;
  }
</style>
