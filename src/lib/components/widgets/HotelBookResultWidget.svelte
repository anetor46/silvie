<script lang="ts">
  /** Result-widget for `hotel_book` — confirmation summary. */
  interface BookOutput {
    booking_id?: string;
    reservation_id?: string;
    hotel_name?: string;
    check_in?: string;
    check_out?: string;
    total_charged_minor_units?: number;
    currency?: string;
    status?: string;
    refundable?: boolean;
  }

  let { output }: { output: Record<string, unknown> } = $props();
  const o = $derived(output as BookOutput);

  function fmtMoney(minor: number | undefined, currency: string | undefined): string {
    if (minor == null || !currency) return '—';
    try {
      return (minor / 100).toLocaleString(undefined, {
        style: 'currency',
        currency,
      });
    } catch {
      return `${(minor / 100).toFixed(2)} ${currency}`;
    }
  }
</script>

<div class="widget">
  <div class="title">Booking confirmed</div>
  <dl>
    <dt>Hotel</dt>
    <dd>{o.hotel_name || '—'}</dd>
    <dt>Confirmation</dt>
    <dd class="mono">{o.reservation_id || '—'}</dd>
    <dt>Dates</dt>
    <dd>{o.check_in || '—'} → {o.check_out || '—'}</dd>
    <dt>Total</dt>
    <dd class="price">{fmtMoney(o.total_charged_minor_units, o.currency)}</dd>
    <dt>Refund</dt>
    <dd>
      {#if o.refundable}
        <span class="badge refundable">Refundable</span>
      {:else}
        <span class="badge nonrefund">Non-refundable</span>
      {/if}
    </dd>
  </dl>
  {#if o.booking_id}
    <p class="ref">Booking ref: <span class="mono">{o.booking_id}</span></p>
  {/if}
</div>

<style>
  .widget { display: flex; flex-direction: column; gap: 8px; }
  .title { font-weight: 600; color: var(--text-primary); font-size: 13px; }
  dl { display: grid; grid-template-columns: 100px 1fr; gap: 4px 12px; margin: 0; font-size: 13px; }
  dt { color: var(--text-dim); }
  dd { margin: 0; color: var(--text-primary); word-break: break-word; }
  .price { font-weight: 600; }
  .mono { font-family: ui-monospace, SFMono-Regular, monospace; font-size: 12px; }
  .ref { margin: 0; font-size: 11px; color: var(--text-dim); }
  .badge {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    padding: 2px 6px;
    border-radius: 4px;
  }
  .refundable { background: #ecfdf5; color: #047857; }
  .nonrefund { background: #fef2f2; color: #b91c1c; }
</style>
