<script lang="ts">
  import type { TimelineEvent } from '$lib/data/events';
  import { typeMeta } from '$lib/data/events';

  let { event, past = false }: { event: TimelineEvent; past?: boolean } = $props();

  const meta = $derived(typeMeta(event.type));

  function fmtTime(d: Date): string {
    return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  }

  const timeLabel = $derived(
    event.end ? `${fmtTime(event.start)} – ${fmtTime(event.end)}` : fmtTime(event.start),
  );

  const icons: Record<string, string> = {
    flight: `<path d="M21 16v-2l-8-5V3.5a1.5 1.5 0 0 0-3 0V9l-8 5v2l8-2.5V19l-2 1.5V22l3.5-1L15 22v-1.5L13 19v-5.5z"/>`,
    hotel: `<path d="M2 20v-9h4V8a2 2 0 0 1 2-2h12a2 2 0 0 1 2 2v12h-2v-3H4v3z"/><circle cx="8.5" cy="11.5" r="2"/>`,
    meeting: `<circle cx="9" cy="7" r="4"/><circle cx="17" cy="9" r="3"/><path d="M2 20a7 7 0 0 1 14 0M16 20a5 5 0 0 1 6 0"/>`,
    restaurant: `<path d="M7 2v9a2 2 0 0 0 2 2v9M11 2v9a2 2 0 0 1-2 2M16 2c-2 2-2 6 0 8v12"/>`,
    taxi: `<path d="M5 17h14M3 17v-4a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2v4M5 11l2-5a2 2 0 0 1 2-1h6a2 2 0 0 1 2 1l2 5"/><circle cx="7" cy="17" r="2"/><circle cx="17" cy="17" r="2"/>`,
    train: `<rect x="5" y="3" width="14" height="14" rx="2"/><path d="M9 17l-3 4M15 17l3 4"/><circle cx="9" cy="13" r="1"/><circle cx="15" cy="13" r="1"/>`,
  };
</script>

<article class="card" class:past>
  <div class="icon" style:--accent={meta.color}>
    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
      {@html icons[event.type]}
    </svg>
  </div>

  <div class="body">
    <div class="row1">
      <span class="title">{event.title}</span>
      <span class="time">{timeLabel}</span>
    </div>
    {#if event.subtitle || event.location}
      <p class="sub">
        {#if event.subtitle}{event.subtitle}{/if}
        {#if event.subtitle && event.location} · {/if}
        {#if event.location}{event.location}{/if}
      </p>
    {/if}
  </div>
</article>

<style>
  .card {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 12px 14px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 10px;
    transition: border-color 0.15s, background 0.15s;
  }

  .card:hover {
    border-color: var(--border-strong);
    background: var(--surface);
  }

  .card.past {
    opacity: 0.6;
  }

  .icon {
    width: 32px;
    height: 32px;
    border-radius: 8px;
    background: color-mix(in srgb, var(--accent) 14%, transparent);
    color: var(--accent);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .body {
    flex: 1;
    min-width: 0;
  }

  .row1 {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 10px;
  }

  .title {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .time {
    font-size: 12px;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
    flex-shrink: 0;
  }

  .sub {
    font-size: 12px;
    color: var(--text-muted);
    margin-top: 3px;
  }
</style>
