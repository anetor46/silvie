<script lang="ts">
  /** Proposal-widget for `hotel_cancel_booking` — confirms the user wants to
   *  cancel. The actual refund amount depends on the policy snapshot stored
   *  with the booking; the LLM should quote it in the surrounding chat text. */
  interface CancelArgs {
    booking_id?: string;
    confirm?: boolean;
  }
  let { args }: { args: Record<string, unknown> } = $props();
  const a = $derived(args as CancelArgs);
</script>

<div class="widget">
  <div class="title">Cancel booking</div>
  {#if a.booking_id}
    <p class="ref">Booking <span class="mono">{a.booking_id}</span></p>
  {/if}
  <p class="warn">
    This will cancel the reservation at the supplier. Refund (if any) follows
    the cancellation policy on the booking — review it before approving.
  </p>
</div>

<style>
  .widget { display: flex; flex-direction: column; gap: 8px; }
  .title { font-weight: 600; color: var(--text-primary); font-size: 13px; }
  .ref { margin: 0; font-size: 13px; color: var(--text-primary); }
  .mono { font-family: ui-monospace, SFMono-Regular, monospace; font-size: 12px; }
  .warn {
    margin: 0;
    padding: 8px 10px;
    background: #fef2f2;
    color: #b91c1c;
    border-radius: 6px;
    font-size: 12px;
    line-height: 1.4;
  }
</style>
