<script lang="ts">
  interface ReplyArgs {
    thread_id?: string;
    to?: string[];
    cc?: string[] | null;
    subject?: string;
    body?: string;
  }
  let { args }: { args: Record<string, unknown> } = $props();
  const a = $derived(args as ReplyArgs);
</script>

<div class="widget">
  <div class="title">Reply to email</div>
  <dl>
    <dt>To</dt>
    <dd>{(a.to ?? []).join(', ') || '—'}</dd>
    {#if a.cc && a.cc.length > 0}
      <dt>CC</dt>
      <dd>{a.cc.join(', ')}</dd>
    {/if}
    <dt>Subject</dt>
    <dd>{a.subject || '—'}</dd>
  </dl>
  {#if a.body}
    <div class="body-preview">{a.body}</div>
  {/if}
</div>

<style>
  .widget { display: flex; flex-direction: column; gap: 8px; }
  .title { font-weight: 600; color: var(--text-primary); font-size: 13px; }
  dl { display: grid; grid-template-columns: 60px 1fr; gap: 4px 12px; margin: 0; font-size: 13px; }
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
    max-height: 140px;
    overflow-y: auto;
    line-height: 1.5;
  }
</style>
