<script lang="ts">
  import type { ToolCallEntry } from '$lib/types';
  import { getToolInfo } from '$lib/data/tools';

  let { toolCall }: { toolCall: ToolCallEntry } = $props();

  const info = $derived(getToolInfo(toolCall.name));
  const labelOverride = $derived(
    toolCall.status === 'pending_user' && !toolCall.decision
      ? 'Awaiting your approval'
      : info.label,
  );

  let expanded = $state(false);

  function toggle() {
    expanded = !expanded;
  }

  function handleKey(e: KeyboardEvent) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      toggle();
    }
  }

  /** Pretty-print a JSON value as a multi-line string. Returns the raw
   *  string if it's not actually JSON-serialisable. */
  function pretty(value: unknown): string {
    if (typeof value === 'string') return value;
    try {
      return JSON.stringify(value, null, 2);
    } catch {
      return String(value);
    }
  }

  const hasArgs = $derived(Object.keys(toolCall.args ?? {}).length > 0);
  const hasOutput = $derived(toolCall.output !== undefined && toolCall.output !== null);
  const hasSummary = $derived(
    toolCall.summary !== undefined && toolCall.summary !== '' && !hasOutput,
  );
  const hasDetails = $derived(hasArgs || hasOutput || hasSummary);
</script>

<div class="tool-card" data-status={toolCall.status} data-expanded={expanded}>
  <button
    type="button"
    class="header"
    onclick={toggle}
    onkeydown={handleKey}
    aria-expanded={expanded}
    aria-label="{expanded ? 'Hide' : 'Show'} tool call details"
    disabled={!hasDetails}
  >
    <div class="icon" aria-hidden="true">
      {#if info.icon === 'inbox'}
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="22 12 16 12 14 15 10 15 8 12 2 12"/><path d="M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11Z"/></svg>
      {:else if info.icon === 'mail'}
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="20" height="16" x="2" y="4" rx="2"/><path d="m22 7-8.97 5.7a1.94 1.94 0 0 1-2.06 0L2 7"/></svg>
      {:else if info.icon === 'send'}
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14.536 21.686a.5.5 0 0 0 .937-.024l6.5-19a.496.496 0 0 0-.635-.635l-19 6.5a.5.5 0 0 0-.024.937l7.93 3.18a2 2 0 0 1 1.112 1.11z"/><path d="m21.854 2.147-10.94 10.939"/></svg>
      {:else if info.icon === 'reply'}
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 17 4 12 9 7"/><path d="M20 18v-2a4 4 0 0 0-4-4H4"/></svg>
      {:else if info.icon === 'calendar'}
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M8 2v4"/><path d="M16 2v4"/><rect width="18" height="18" x="3" y="4" rx="2"/><path d="M3 10h18"/></svg>
      {:else if info.icon === 'calendar-plus'}
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M8 2v4"/><path d="M16 2v4"/><rect width="18" height="18" x="3" y="4" rx="2"/><path d="M3 10h18"/><path d="M12 14v6"/><path d="M9 17h6"/></svg>
      {:else if info.icon === 'calendar-edit'}
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M8 2v4"/><path d="M16 2v4"/><rect width="18" height="18" x="3" y="4" rx="2"/><path d="M3 10h18"/><path d="m17.5 14.5 3 3"/><path d="m14 18 4-4 2.5 2.5L17 20l-3 .5z"/></svg>
      {:else if info.icon === 'calendar-x'}
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M8 2v4"/><path d="M16 2v4"/><rect width="18" height="18" x="3" y="4" rx="2"/><path d="M3 10h18"/><path d="m10 14 4 4"/><path d="m14 14-4 4"/></svg>
      {:else if info.icon === 'calendar-check'}
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M8 2v4"/><path d="M16 2v4"/><rect width="18" height="18" x="3" y="4" rx="2"/><path d="M3 10h18"/><path d="m9 16 2 2 4-4"/></svg>
      {:else if info.icon === 'clock'}
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>
      {:else if info.icon === 'building'}
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="16" height="20" x="4" y="2" rx="2"/><path d="M9 22v-4h6v4"/><path d="M8 6h.01"/><path d="M16 6h.01"/><path d="M12 6h.01"/><path d="M12 10h.01"/><path d="M12 14h.01"/><path d="M16 10h.01"/><path d="M16 14h.01"/><path d="M8 10h.01"/><path d="M8 14h.01"/></svg>
      {:else if info.icon === 'bed'}
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M2 4v16"/><path d="M2 8h18a2 2 0 0 1 2 2v10"/><path d="M2 17h20"/><path d="M6 8v9"/></svg>
      {:else}
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"/></svg>
      {/if}
    </div>
    <div class="body">
      <span class="label">{labelOverride}</span>
      {#if toolCall.summary && toolCall.status !== 'pending_user'}
        <span class="summary">{toolCall.summary}</span>
      {/if}
    </div>
    <div class="status" aria-label={toolCall.status}>
      {#if toolCall.status === 'running'}
        <span class="spinner" aria-hidden="true"></span>
      {:else if toolCall.status === 'pending_user'}
        <span class="pending-dot" aria-hidden="true"></span>
      {:else if toolCall.status === 'success'}
        <svg class="check" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>
      {:else if toolCall.status === 'error'}
        <svg class="cross" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
      {/if}
    </div>
    {#if hasDetails}
      <div class="chevron" aria-hidden="true" data-expanded={expanded}>
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="6 9 12 15 18 9"/></svg>
      </div>
    {/if}
  </button>

  {#if expanded && hasDetails}
    <div class="details">
      {#if hasArgs}
        <section>
          <h4>Input</h4>
          <pre>{pretty(toolCall.args)}</pre>
        </section>
      {/if}
      {#if hasOutput}
        <section>
          <h4>Result</h4>
          <pre>{pretty(toolCall.output)}</pre>
        </section>
      {:else if hasSummary}
        <section>
          <h4>Result</h4>
          <p class="result-text">{toolCall.summary}</p>
        </section>
      {/if}
    </div>
  {/if}
</div>

<style>
  .tool-card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 10px;
    font-size: 13px;
    color: var(--text-secondary);
    overflow: hidden;
  }

  .tool-card[data-status='pending_user'] {
    border-color: var(--purple-400);
    background: var(--purple-50);
  }

  .tool-card[data-status='error'] {
    border-color: #fca5a5;
    background: #fef2f2;
  }

  .header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 12px;
    width: 100%;
    background: none;
    border: none;
    text-align: left;
    cursor: pointer;
    font: inherit;
    color: inherit;
  }
  .header:disabled {
    cursor: default;
  }
  .header:not(:disabled):hover {
    background: var(--surface-2);
  }
  .tool-card[data-status='pending_user'] .header:not(:disabled):hover {
    background: rgba(0, 0, 0, 0.03);
  }

  .icon {
    width: 18px;
    height: 18px;
    flex-shrink: 0;
    color: var(--purple-600);
    line-height: 0;
  }
  .icon :global(svg) {
    width: 100%;
    height: 100%;
  }

  .body {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
  }

  .label {
    color: var(--text-primary);
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .summary {
    color: var(--text-dim);
    font-size: 11px;
    margin-top: 1px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .status {
    flex-shrink: 0;
    width: 16px;
    height: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .spinner {
    width: 14px;
    height: 14px;
    border: 2px solid var(--border-strong);
    border-top-color: var(--purple-600);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  .pending-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--purple-600);
    animation: pulse 1.5s ease-in-out infinite;
  }

  .check { width: 16px; height: 16px; color: #22c55e; }
  .cross { width: 16px; height: 16px; color: #ef4444; }

  .chevron {
    width: 14px;
    height: 14px;
    flex-shrink: 0;
    color: var(--text-dim);
    transition: transform 0.15s;
    line-height: 0;
  }
  .chevron[data-expanded='true'] {
    transform: rotate(180deg);
  }
  .chevron :global(svg) {
    width: 100%;
    height: 100%;
  }

  .details {
    border-top: 1px solid var(--border);
    padding: 10px 12px 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    background: var(--bg);
  }
  .tool-card[data-status='pending_user'] .details {
    background: var(--surface);
  }

  section {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  h4 {
    margin: 0;
    font-size: 11px;
    font-weight: 600;
    color: var(--text-dim);
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }

  pre {
    margin: 0;
    font-family: 'Menlo', 'Consolas', monospace;
    font-size: 11.5px;
    line-height: 1.5;
    color: var(--text-primary);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 8px 10px;
    overflow-x: auto;
    max-height: 280px;
    overflow-y: auto;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .result-text {
    margin: 0;
    font-size: 12px;
    color: var(--text-primary);
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  @keyframes pulse {
    0%, 100% { opacity: 0.4; }
    50% { opacity: 1; }
  }
</style>
