<script lang="ts">
  interface RespondArgs {
    event_id?: string;
    response_status?: string;
  }
  let { args }: { args: Record<string, unknown> } = $props();
  const a = $derived(args as RespondArgs);

  const verb = $derived(() => {
    switch (a.response_status) {
      case 'accepted': return 'Accept';
      case 'declined': return 'Decline';
      case 'tentative': return 'Tentatively accept';
      default: return a.response_status ?? 'Respond to';
    }
  });
</script>

<div class="widget">
  <div class="title">{verb()} invite</div>
  {#if a.event_id}
    <p class="meta">Event ID: <code>{a.event_id}</code></p>
  {/if}
</div>

<style>
  .widget { display: flex; flex-direction: column; gap: 6px; }
  .title { font-weight: 600; color: var(--text-primary); font-size: 13px; }
  .meta { margin: 0; font-size: 11px; color: var(--text-dim); }
  code {
    font-family: 'Menlo', 'Consolas', monospace;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 1px 4px;
  }
</style>
