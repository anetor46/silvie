<script lang="ts">
  /**
   * Result-widget for `hotel_search`. Renders the top offers as inline cards
   * with photo, name, stars, address, lowest per-night rate, and a
   * refundable badge.
   */
  interface Offer {
    offer_id: string;
    property_id: string;
    name: string;
    address?: string;
    city?: string;
    stars?: number | null;
    image_url?: string | null;
    lowest_total_minor_units?: number | null;
    lowest_per_night_minor_units?: number | null;
    currency: string;
    refundable: boolean;
  }

  interface SearchOutput {
    hotels?: Offer[];
    destination?: string;
    check_in?: string;
    check_out?: string;
    nights?: number;
  }

  let { output }: { output: Record<string, unknown> } = $props();
  const o = $derived(output as SearchOutput);
  const hotels = $derived(o.hotels ?? []);

  function fmtMoney(minor: number | null | undefined, currency: string): string {
    if (minor == null) return '—';
    const major = minor / 100;
    try {
      return major.toLocaleString(undefined, {
        style: 'currency',
        currency,
        maximumFractionDigits: 0,
      });
    } catch {
      return `${major.toFixed(0)} ${currency}`;
    }
  }

  function starsOf(n: number | null | undefined): string {
    if (!n) return '';
    const full = Math.round(n);
    return '★'.repeat(full);
  }
</script>

<div class="results">
  <header class="head">
    <span class="dest">{o.destination ?? ''}</span>
    {#if o.check_in && o.check_out}
      <span class="dates">{o.check_in} → {o.check_out}{o.nights ? ` (${o.nights}n)` : ''}</span>
    {/if}
    <span class="count">{hotels.length} result{hotels.length === 1 ? '' : 's'}</span>
  </header>

  {#if hotels.length === 0}
    <p class="empty">No hotels matched. Try widening dates or budget.</p>
  {/if}

  <ul class="list">
    {#each hotels as h (h.offer_id)}
      <li class="card">
        {#if h.image_url}
          <img src={h.image_url} alt="" class="photo" />
        {:else}
          <div class="photo placeholder" aria-hidden="true">🏨</div>
        {/if}
        <div class="body">
          <div class="row1">
            <span class="name">{h.name}</span>
            {#if h.stars}
              <span class="stars" title={`${h.stars}/5`}>{starsOf(h.stars)}</span>
            {/if}
          </div>
          <div class="row2">
            {[h.address, h.city].filter(Boolean).join(', ') || ''}
          </div>
          <div class="row3">
            <span class="price">
              {fmtMoney(h.lowest_per_night_minor_units, h.currency)}<span class="suffix"> / night</span>
            </span>
            {#if h.refundable}
              <span class="badge refundable">Refundable</span>
            {:else}
              <span class="badge nonrefund">Non-refundable</span>
            {/if}
          </div>
        </div>
      </li>
    {/each}
  </ul>
</div>

<style>
  .results { display: flex; flex-direction: column; gap: 10px; }
  .head {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: 12px;
    color: var(--text-dim);
  }
  .dest { font-weight: 600; color: var(--text-primary); }
  .count { margin-left: auto; }
  .empty { margin: 0; font-size: 12px; color: var(--text-dim); font-style: italic; }
  .list { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 8px; }
  .card {
    display: flex;
    gap: 10px;
    padding: 8px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
  }
  .photo {
    width: 64px;
    height: 64px;
    border-radius: 6px;
    object-fit: cover;
    flex-shrink: 0;
    background: var(--surface-2);
  }
  .photo.placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 24px;
    color: var(--text-dim);
  }
  .body { display: flex; flex-direction: column; gap: 2px; min-width: 0; flex: 1; }
  .row1 { display: flex; align-items: center; gap: 8px; }
  .name {
    font-weight: 600;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .stars { color: #d97706; font-size: 12px; letter-spacing: 1px; flex-shrink: 0; }
  .row2 { font-size: 12px; color: var(--text-dim); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .row3 { display: flex; align-items: center; gap: 8px; margin-top: 2px; }
  .price { font-weight: 600; color: var(--text-primary); font-size: 13px; }
  .suffix { font-weight: 400; color: var(--text-dim); }
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
