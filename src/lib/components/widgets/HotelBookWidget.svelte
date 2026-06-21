<script lang="ts">
  interface BookArgs {
    hotel_name?: string;
    hotel_id?: string;
    check_in?: string;
    check_out?: string;
    guests?: number;
    total_price_minor_units?: number;
    currency?: string;
  }
  let { args }: { args: Record<string, unknown> } = $props();
  const a = $derived(args as BookArgs);

  const price = $derived(() => {
    if (a.total_price_minor_units == null || !a.currency) return '—';
    const major = (a.total_price_minor_units / 100).toLocaleString(undefined, {
      style: 'currency',
      currency: a.currency,
    });
    return major;
  });
</script>

<div class="widget">
  <div class="title">Book hotel</div>
  <dl>
    <dt>Hotel</dt>
    <dd>{a.hotel_name || '—'}</dd>
    <dt>Check-in</dt>
    <dd>{a.check_in || '—'}</dd>
    <dt>Check-out</dt>
    <dd>{a.check_out || '—'}</dd>
    {#if a.guests}
      <dt>Guests</dt>
      <dd>{a.guests}</dd>
    {/if}
    <dt>Total</dt>
    <dd class="price">{price()}</dd>
  </dl>
  <p class="charge-note">
    A virtual card will be charged for the full amount above.
  </p>
</div>

<style>
  .widget { display: flex; flex-direction: column; gap: 8px; }
  .title { font-weight: 600; color: var(--text-primary); font-size: 13px; }
  dl { display: grid; grid-template-columns: 80px 1fr; gap: 4px 12px; margin: 0; font-size: 13px; }
  dt { color: var(--text-dim); }
  dd { margin: 0; color: var(--text-primary); word-break: break-word; }
  .price { font-weight: 600; }
  .charge-note {
    margin: 0;
    font-size: 11px;
    color: var(--text-dim);
    font-style: italic;
  }
</style>
