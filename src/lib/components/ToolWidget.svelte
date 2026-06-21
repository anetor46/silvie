<script lang="ts">
  /**
   * Renders the tool-specific "preview" widget for a tool call, picked by
   * the registry kind. Used in two places:
   *
   *   1. `ConfirmationWidget` — wrapping it with Approve / Reject buttons
   *      while the call is `pending_user`.
   *   2. `ToolCallCard` (expanded) — read-only inspection of an already-
   *      submitted write call.
   *
   * Read tools have no widget — render nothing (the card shows its `brief`
   * instead).
   */
  import type { ToolCallEntry } from '$lib/types';
  import { getToolInfo } from '$lib/data/tools';
  import EmailSendWidget from './widgets/EmailSendWidget.svelte';
  import EmailReplyWidget from './widgets/EmailReplyWidget.svelte';
  import CalendarEventCreateWidget from './widgets/CalendarEventCreateWidget.svelte';
  import CalendarEventUpdateWidget from './widgets/CalendarEventUpdateWidget.svelte';
  import CalendarEventDeleteWidget from './widgets/CalendarEventDeleteWidget.svelte';
  import CalendarEventRespondWidget from './widgets/CalendarEventRespondWidget.svelte';
  import HotelBookWidget from './widgets/HotelBookWidget.svelte';

  let { toolCall }: { toolCall: ToolCallEntry } = $props();
  const widgetKind = $derived(getToolInfo(toolCall.name).widget);
</script>

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
{/if}
