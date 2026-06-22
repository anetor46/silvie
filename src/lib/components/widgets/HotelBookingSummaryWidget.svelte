<script lang="ts">
  /** Result-widget for `hotel_retrieve_booking` — current state of a booking. */
  interface CancellationPolicy {
    refundable: boolean;
    refund_deadline?: string | null;
    penalty_minor_units?: number | null;
    description?: string | null;
  }

  interface RetrieveOutput {
    booking_id?: string;
    reservation_id?: string | null;
    hotel_name?: string;
    check_in?: string;
    check_out?: string;
    status?: string;
    total_amount_minor_units?: number;
    currency?: string;
    cancellation_policy?: CancellationPolicy | null;
    supplier_status?: string | null;
  }

  let { output }: { output: Record<string, unknown> } = $props();
  const o = $derived(output as RetrieveOutput);

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

  const statusClass = $derived(o.status === 'confirmed' ? 'ok' : o.status === 'cancelled' ? 'cancelled' : o.status === 'failed' ? 'fail' : 'neutral');
</script>

<div class="widget">
  <div class="title">{o.hotel_name || 'Booking'}</div>
  <dl>
    <dt>Status</dt>
    <dd><span class="badge {statusClass}">{o.status || '—'}</span></dd>
    {#if o.supplier_status && o.supplier_status !== o.status}
      <dt>Supplier</dt>
      <dd>{o.supplier_status}</dd>
    {/if}
    <dt>Dates</dt>
    <dd>{o.check_in || '—'} → {o.check_out || '—'}</dd>
    <dt>Total</dt>
    <dd class="price">{fmtMoney(o.total_amount_minor_units, o.currency)}</dd>
    {#if o.reservation_id}
      <dt>Confirmation</dt>
      <dd class="mono">{o.reservation_id}</dd>
    {/if}
    {#if o.cancellation_policy}
      <dt>Refund</dt>
      <dd>
        {#if o.cancellation_policy.refundable}
          <span class="badge ok">Refundable</span>
          {#if o.cancellation_policy.refund_deadline}
            <span class="dim">until {o.cancellation_policy.refund_deadline}</span>
          {/if}
        {:else}
          <span class="badge fail">Non-refundable</span>
        {/if}
      </dd>
    {/if}
  </dl>
</div>

<style>
  .widget { display: flex; flex-direction: column; gap: 8px; }
  .title { font-weight: 600; color: var(--text-primary); font-size: 13px; }
  dl { display: grid; grid-template-columns: 110px 1fr; gap: 4px 12px; margin: 0; font-size: 13px; }
  dt { color: var(--text-dim); }
  dd { margin: 0; color: var(--text-primary); word-break: break-word; display: flex; gap: 6px; align-items: center; flex-wrap: wrap; }
  .price { font-weight: 600; }
  .mono { font-family: ui-monospace, SFMono-Regular, monospace; font-size: 12px; }
  .badge {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    padding: 2px 6px;
    border-radius: 4px;
  }
  .badge.ok { background: #ecfdf5; color: #047857; }
  .badge.cancelled { background: var(--surface-2); color: var(--text-dim); }
  .badge.fail { background: #fef2f2; color: #b91c1c; }
  .badge.neutral { background: var(--surface-2); color: var(--text-primary); }
  .dim { color: var(--text-dim); font-size: 11px; }
</style>
