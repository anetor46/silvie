<script lang="ts">
  interface CreateArgs {
    title?: string;
    start_time?: string;
    end_time?: string;
    location?: string;
    description?: string;
    attendees?: string[];
  }
  let { args }: { args: Record<string, unknown> } = $props();
  const a = $derived(args as CreateArgs);

  function fmt(s?: string): string {
    if (!s) return '—';
    try {
      return new Date(s).toLocaleString(undefined, {
        weekday: 'short', month: 'short', day: 'numeric',
        hour: 'numeric', minute: '2-digit',
      });
    } catch {
      return s;
    }
  }
</script>

<div class="widget">
  <div class="title">Create event</div>
  <dl>
    <dt>Title</dt>
    <dd>{a.title || '—'}</dd>
    <dt>Start</dt>
    <dd>{fmt(a.start_time)}</dd>
    <dt>End</dt>
    <dd>{fmt(a.end_time)}</dd>
    {#if a.location}
      <dt>Where</dt>
      <dd>{a.location}</dd>
    {/if}
    {#if a.attendees && a.attendees.length > 0}
      <dt>Guests</dt>
      <dd>{a.attendees.join(', ')}</dd>
    {/if}
  </dl>
  {#if a.description}
    <div class="body-preview">{a.description}</div>
  {/if}
</div>

<style>
  .widget { display: flex; flex-direction: column; gap: 8px; }
  .title { font-weight: 600; color: var(--text-primary); font-size: 13px; }
  dl { display: grid; grid-template-columns: 70px 1fr; gap: 4px 12px; margin: 0; font-size: 13px; }
  dt { color: var(--text-dim); }
  dd { margin: 0; color: var(--text-primary); word-break: break-word; }
  .body-preview {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 8px 10px;
    font-size: 12px;
    color: var(--text-secondary);
    white-space: pre-wrap;
    max-height: 100px;
    overflow-y: auto;
    line-height: 1.5;
  }
</style>
