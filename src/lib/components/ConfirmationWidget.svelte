<script lang="ts">
  import type { ToolCallEntry } from '$lib/types';
  import { getToolInfo } from '$lib/data/tools';
  import { postConfirmation } from '$lib/services/confirmations';
  import { conversations } from '$lib/stores/conversations.svelte';
  import EmailSendWidget from './widgets/EmailSendWidget.svelte';
  import EmailReplyWidget from './widgets/EmailReplyWidget.svelte';
  import CalendarEventCreateWidget from './widgets/CalendarEventCreateWidget.svelte';
  import CalendarEventUpdateWidget from './widgets/CalendarEventUpdateWidget.svelte';
  import CalendarEventDeleteWidget from './widgets/CalendarEventDeleteWidget.svelte';
  import CalendarEventRespondWidget from './widgets/CalendarEventRespondWidget.svelte';
  import HotelBookWidget from './widgets/HotelBookWidget.svelte';

  let { toolCall }: { toolCall: ToolCallEntry } = $props();

  const widgetKind = $derived(getToolInfo(toolCall.name).widget);
  let busy = $state(false);
  let error = $state<string | null>(null);

  async function decide(approved: boolean) {
    if (busy) return;
    busy = true;
    error = null;
    try {
      await postConfirmation(toolCall.callId, approved);
      conversations.setDecision(toolCall.callId, approved ? 'approved' : 'rejected');
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="card">
  {#if widgetKind === 'email_send'}
    <EmailSendWidget args={toolCall.args} />
  {:else if widgetKind === 'email_reply'}
    <EmailReplyWidget args={toolCall.args} />
  {:else if widgetKind === 'calendar_event_create'}
    <CalendarEventCreateWidget args={toolCall.args} />
  {:else if widgetKind === 'calendar_event_update'}
    <CalendarEventUpdateWidget args={toolCall.args} />
  {:else if widgetKind === 'calendar_event_delete'}
    <CalendarEventDeleteWidget args={toolCall.args} />
  {:else if widgetKind === 'calendar_event_respond'}
    <CalendarEventRespondWidget args={toolCall.args} />
  {:else if widgetKind === 'hotel_book'}
    <HotelBookWidget args={toolCall.args} />
  {:else}
    <p class="generic">
      The assistant wants to run <code>{toolCall.name}</code>.
    </p>
  {/if}

  {#if toolCall.decision === 'approved'}
    <div class="badge approved">✅ Approved</div>
  {:else if toolCall.decision === 'rejected'}
    <div class="badge rejected">❌ Rejected</div>
  {:else}
    <div class="actions">
      <button class="reject" onclick={() => decide(false)} disabled={busy}>Reject</button>
      <button class="approve" onclick={() => decide(true)} disabled={busy}>
        {busy ? 'Sending…' : 'Approve'}
      </button>
    </div>
  {/if}

  {#if error}
    <p class="error">{error}</p>
  {/if}
</div>

<style>
  .card {
    background: var(--bg);
    border: 1px solid var(--purple-400);
    border-radius: 12px;
    padding: 14px 16px;
    margin-top: 6px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .generic {
    margin: 0;
    font-size: 13px;
    color: var(--text-primary);
  }
  .generic code {
    font-family: 'Menlo', 'Consolas', monospace;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 1px 5px;
    font-size: 12px;
  }

  .actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
  }

  button {
    padding: 7px 16px;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 500;
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--text-primary);
    cursor: pointer;
    transition: background 0.15s, border-color 0.15s;
  }
  button:hover:not(:disabled) {
    background: var(--surface-2);
  }
  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .approve {
    background: var(--purple-600);
    color: #fff;
    border-color: var(--purple-600);
  }
  .approve:hover:not(:disabled) {
    background: var(--purple-800);
  }

  .badge {
    align-self: flex-end;
    font-size: 12px;
    font-weight: 500;
    padding: 4px 10px;
    border-radius: 8px;
  }
  .badge.approved {
    background: #dcfce7;
    color: #166534;
  }
  .badge.rejected {
    background: #fee2e2;
    color: #991b1b;
  }

  .error {
    margin: 0;
    font-size: 12px;
    color: #b91c1c;
  }
</style>
