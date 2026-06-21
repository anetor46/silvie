<script lang="ts">
  interface DeleteArgs {
    event_id?: string;
  }
  let { args }: { args: Record<string, unknown> } = $props();
  const a = $derived(args as DeleteArgs);
</script>

<div class="widget">
  <div class="title">Delete event</div>
  <p class="warning">This will permanently remove the event from your calendar.</p>
  {#if a.event_id}
    <p class="meta">Event ID: <code>{a.event_id}</code></p>
  {/if}
</div>

<style>
  .widget { display: flex; flex-direction: column; gap: 6px; }
  .title { font-weight: 600; color: var(--text-primary); font-size: 13px; }
  .warning { margin: 0; font-size: 13px; color: var(--text-primary); }
  .meta { margin: 0; font-size: 11px; color: var(--text-dim); }
  code {
    font-family: 'Menlo', 'Consolas', monospace;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 1px 4px;
  }
</style>
