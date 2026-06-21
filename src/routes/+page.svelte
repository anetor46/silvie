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
   *  assistant placeholder created lazily on first token. Each callback
   *  also nudges the `isThinking` flag so the bottom-of-chat 3-dot wave
   *  appears whenever there's no other visible progress. */
  function makeCallbacks(): ChatCallbacks {
    let assistantId: string | null = null;
    return {
      onToken: (chunk) => {
        // Text is now arriving — hide the thinking dots.
        conversations.setThinking(false);
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
        // While a tool is RUNNING we keep the indicator on (the card has
        // its own spinner, but the global indicator reinforces progress).
        // For a PENDING_USER call we hide it — the confirmation widget IS
        // the user-facing prompt now.
        conversations.setThinking(!call.requiresConfirmation);
        scrollToBottom();
      },
      onToolResult: (result) => {
        conversations.updateToolResult(
          result.callId,
          result.success,
          result.summary,
          result.output,
        );
        // Tool finished — model is computing the next text / tool call.
        conversations.setThinking(true);
        scrollToBottom();
      },
    };
  }

  async function sendMessage() {
    const text = inputValue.trim();
    if (!text) return;
    // Defense in depth — the InputBar already disables submission while a
    // stream is in flight, but guard here too in case anything bypasses it.
    if (conversations.isStreaming) return;

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
    // Start thinking before the first event lands so the user sees
    // immediate feedback even if the server takes a moment.
    conversations.setStreaming(true);
    conversations.setThinking(true);
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
        conversations.setThinking(false);
        conversations.setStreaming(false);
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
    if (conversations.isStreaming) return;

    if (response.kind === 'confirmation') {
      conversations.setDecision(callId, response.approved ? 'approved' : 'rejected');
    }
    // Resume stream is starting — flip the streaming flag so the input bar
    // locks until the model finishes its follow-up.
    conversations.setStreaming(true);
    conversations.setThinking(true);
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
        conversations.setThinking(false);
        conversations.setStreaming(false);
        if (currentStream === handle) currentStream = null;
      });
  }

  function handleSuggestion(text: string) {
    inputValue = text;
  }

  /** Abort the current SSE stream. The fetch's AbortController fires, the
   *  reader stops, and the `.finally()` block clears the streaming flag so
   *  the input bar unlocks. Note: the backend's spawn task keeps running
   *  to completion — it persists the assistant message to the DB even
   *  after the client disconnects. Truly stopping the model would need
   *  backend cancellation plumbing. */
  function cancelStream() {
    currentStream?.cancel();
  }
</script>

<main class="chat-area" bind:this={messagesEl}>
  {#if conversations.currentMessages.length === 0}
    <EmptyState {suggestions} onSuggestionClick={handleSuggestion} />
  {:else}
    <MessageList messages={conversations.currentMessages} onToolResponse={handleToolResponse} />
  {/if}
</main>

<InputBar
  bind:value={inputValue}
  onSend={sendMessage}
  onCancel={cancelStream}
  disabled={conversations.isStreaming}
/>

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
