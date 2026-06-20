<script lang="ts">
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { conversations } from '$lib/stores/conversations.svelte';
  import BrandMark from './BrandMark.svelte';

  let {
    open = $bindable(false),
  }: {
    open?: boolean;
  } = $props();

  const navItems = [
    {
      href: '/timeline',
      label: 'Timeline',
      icon: `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="4" width="18" height="18" rx="2"/><line x1="16" y1="2" x2="16" y2="6"/><line x1="8" y1="2" x2="8" y2="6"/><line x1="3" y1="10" x2="21" y2="10"/></svg>`,
    },
    {
      href: '/connectors',
      label: 'Connectors',
      icon: `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"/><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"/></svg>`,
    },
  ];

  function startNewChat() {
    conversations.newChat();
    open = false;
    if (page.url.pathname !== '/') goto('/');
  }

  function openConversation(id: string) {
    conversations.setActive(id);
    open = false;
    if (page.url.pathname !== '/') goto('/');
  }
</script>

<!-- Overlay -->
{#if open}
  <div
    class="overlay"
    onclick={() => (open = false)}
    onkeydown={(e) => e.key === 'Escape' && (open = false)}
    role="button"
    tabindex="-1"
    aria-label="Close menu"
  ></div>
{/if}

<!-- Drawer -->
<nav class="sidebar" class:open aria-label="App menu">
  <div class="sidebar-header">
    <div class="sidebar-brand">
      <BrandMark size={22} radius={5} />
      <span class="sidebar-wordmark">Silvie</span>
    </div>
    <button class="close-btn" onclick={() => (open = false)} aria-label="Close menu">
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
        <line x1="18" y1="6" x2="6" y2="18" />
        <line x1="6" y1="6" x2="18" y2="18" />
      </svg>
    </button>
  </div>

  <button class="new-chat-btn" onclick={startNewChat}>
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
      <line x1="12" y1="5" x2="12" y2="19" />
      <line x1="5" y1="12" x2="19" y2="12" />
    </svg>
    <span>New chat</span>
  </button>

  <div class="conversations">
    {#if conversations.conversations.length === 0}
      <p class="empty-hint">No conversations yet.</p>
    {:else}
      <p class="section-label">Recent</p>
      <ul class="conv-list">
        {#each conversations.conversations as conv (conv.id)}
          <li>
            <button
              class="conv-item"
              class:active={conversations.activeId === conv.id && page.url.pathname === '/'}
              onclick={() => openConversation(conv.id)}
              title={conv.title}
            >
              {conv.title}
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  <ul class="nav-list">
    {#each navItems as item}
      <li>
        <a
          href={item.href}
          class="nav-item"
          class:active={page.url.pathname === item.href}
          onclick={() => (open = false)}
        >
          {@html item.icon}
          <span>{item.label}</span>
        </a>
      </li>
    {/each}
  </ul>
</nav>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(26, 26, 46, 0.4);
    z-index: 99;
    animation: fade-in 0.2s ease;
    cursor: default;
  }

  @keyframes fade-in {
    from { opacity: 0; }
    to   { opacity: 1; }
  }

  .sidebar {
    position: fixed;
    top: 0;
    left: 0;
    height: 100vh;
    width: 240px;
    background: var(--surface);
    border-right: 1px solid var(--border);
    z-index: 100;
    display: flex;
    flex-direction: column;
    transform: translateX(-100%);
    transition: transform 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .sidebar.open {
    transform: translateX(0);
  }

  .sidebar-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 12px 0 16px;
    height: 52px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .sidebar-brand {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .sidebar-wordmark {
    font-size: 15px;
    font-weight: 600;
    letter-spacing: 0.02em;
    color: var(--text-primary);
  }

  .close-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    border-radius: 8px;
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }

  .close-btn:hover {
    background: var(--surface-hover);
    color: var(--purple-600);
  }

  /* New chat button */
  .new-chat-btn {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 12px 8px 6px;
    padding: 9px 12px;
    background: var(--bg);
    border: 1px solid var(--border);
    color: var(--text-primary);
    font-family: inherit;
    font-size: 13px;
    font-weight: 500;
    border-radius: 8px;
    cursor: pointer;
    transition: background 0.15s, border-color 0.15s, color 0.15s;
  }

  .new-chat-btn:hover {
    background: var(--purple-50);
    border-color: var(--purple-400);
    color: var(--purple-600);
  }

  /* Conversations scroll area */
  .conversations {
    flex: 1;
    overflow-y: auto;
    padding: 8px 8px 4px;
    min-height: 0;
  }

  .conversations::-webkit-scrollbar {
    width: 6px;
  }

  .conversations::-webkit-scrollbar-thumb {
    background: var(--border-strong);
    border-radius: 3px;
  }

  .section-label {
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
    padding: 6px 10px 4px;
  }

  .empty-hint {
    font-size: 12px;
    color: var(--text-dim);
    padding: 8px 10px;
  }

  .conv-list {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .conv-item {
    width: 100%;
    text-align: left;
    padding: 8px 10px;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font-family: inherit;
    font-size: 13px;
    border-radius: 6px;
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .conv-item:hover {
    background: var(--surface-2);
    color: var(--text-primary);
  }

  .conv-item.active {
    background: var(--purple-50);
    color: var(--purple-600);
    font-weight: 500;
  }

  /* Bottom nav (Connectors, Preferences) */
  .nav-list {
    list-style: none;
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    border-top: 1px solid var(--border);
    flex-shrink: 0;
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 10px;
    border-radius: 8px;
    color: var(--text-secondary);
    font-size: 14px;
    font-weight: 500;
    text-decoration: none;
    transition: background 0.15s, color 0.15s;
  }

  .nav-item:hover {
    background: var(--surface-2);
    color: var(--text-primary);
  }

  .nav-item.active {
    background: var(--purple-50);
    color: var(--purple-600);
  }
</style>
