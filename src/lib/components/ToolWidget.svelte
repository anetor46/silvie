<script lang="ts">
  /**
   * Renders the tool-specific widget for a tool call. Picks one of two kinds
   * from the registry:
   *
   *   - `widget`       — args preview (used by `ConfirmationWidget` for write
   *                       tools, and by `ToolCallCard` when expanded).
   *   - `resultWidget` — output preview (rendered by `ToolCallCard` once the
   *                       call succeeds).
   *
   * Pass `mode="result"` to switch to the resultWidget branch. Default is
   * `proposal` (args).
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
  import HotelCancelWidget from './widgets/HotelCancelWidget.svelte';
  import HotelSearchResultsWidget from './widgets/HotelSearchResultsWidget.svelte';
  import HotelBookResultWidget from './widgets/HotelBookResultWidget.svelte';
  import HotelBookingSummaryWidget from './widgets/HotelBookingSummaryWidget.svelte';

  let {
    toolCall,
    mode = 'proposal',
  }: { toolCall: ToolCallEntry; mode?: 'proposal' | 'result' } = $props();

  const info = $derived(getToolInfo(toolCall.name));
  const kind = $derived(mode === 'result' ? info.resultWidget : info.widget);
  const output = $derived(
    (toolCall.output ?? {}) as Record<string, unknown>,
  );
</script>

{#if mode === 'proposal'}
  {#if kind === 'email_send'}
    <EmailSendWidget args={toolCall.args} />
  {:else if kind === 'email_reply'}
    <EmailReplyWidget args={toolCall.args} />
  {:else if kind === 'calendar_event_create'}
    <CalendarEventCreateWidget args={toolCall.args} />
  {:else if kind === 'calendar_event_update'}
    <CalendarEventUpdateWidget args={toolCall.args} />
  {:else if kind === 'calendar_event_delete'}
    <CalendarEventDeleteWidget args={toolCall.args} />
  {:else if kind === 'calendar_event_respond'}
    <CalendarEventRespondWidget args={toolCall.args} />
  {:else if kind === 'hotel_book'}
    <HotelBookWidget args={toolCall.args} />
  {:else if kind === 'hotel_cancel'}
    <HotelCancelWidget args={toolCall.args} />
  {/if}
{:else if kind === 'hotel_search_results'}
  <HotelSearchResultsWidget output={output} />
{:else if kind === 'hotel_book_result'}
  <HotelBookResultWidget output={output} />
{:else if kind === 'hotel_booking_summary'}
  <HotelBookingSummaryWidget output={output} />
{/if}
