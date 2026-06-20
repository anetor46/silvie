import {
  createConversation,
  deleteConversation as serviceDelete,
  getConversation,
  listConversations,
  updateConversationTitle,
  type Conversation,
} from '$lib/services/conversations';
import type { Message } from '$lib/types';

/** Map a backend message row to the lighter view-model the UI works with. */
function toUiMessage(m: { id: string; role: string; content: string }): Message | null {
  if (m.role !== 'user' && m.role !== 'assistant') return null;
  return { id: m.id, role: m.role, content: m.content };
}

class ConversationsStore {
  /** Sidebar list (most recent first). */
  list = $state<Conversation[]>([]);
  loaded = $state(false);
  error = $state<string | null>(null);

  /** The currently-open conversation's id, or null if the user is in the
   *  "new chat" empty state (no DB row created yet). */
  currentId = $state<string | null>(null);
  currentMessages = $state<Message[]>([]);

  /** Convenience: the row from the sidebar list matching `currentId`. */
  get active(): Conversation | undefined {
    return this.list.find((c) => c.id === this.currentId);
  }

  // ── Loading ────────────────────────────────────────────────────────────

  async load(): Promise<void> {
    try {
      this.list = await listConversations();
    } catch (e) {
      console.error('[conversations.load]', e);
      this.error = e instanceof Error ? e.message : String(e);
    } finally {
      this.loaded = true;
    }
  }

  async selectConversation(id: string): Promise<void> {
    this.currentId = id;
    this.currentMessages = [];
    this.error = null;
    try {
      const convo = await getConversation(id);
      if (!convo) {
        this.error = 'Conversation not found.';
        return;
      }
      this.currentMessages = convo.messages
        .map(toUiMessage)
        .filter((m): m is Message => m !== null);
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
    }
  }

  // ── Composition flow ───────────────────────────────────────────────────

  /** Reset to the "new chat" empty state — does NOT create a row yet. The
   *  row is created lazily by `ensureConversation()` when the first message
   *  is sent. */
  newChat(): void {
    this.currentId = null;
    this.currentMessages = [];
    this.error = null;
  }

  /** Get the conversation id for the current chat, creating one on the
   *  backend if there isn't one yet. */
  async ensureConversation(): Promise<string> {
    if (this.currentId) return this.currentId;
    const c = await createConversation();
    this.list = [c, ...this.list];
    this.currentId = c.id;
    return c.id;
  }

  appendUserMessage(content: string): void {
    this.currentMessages = [
      ...this.currentMessages,
      { id: crypto.randomUUID(), role: 'user', content },
    ];
  }

  /** Append an empty assistant placeholder and return its temp id so the
   *  caller can stream tokens into it via `appendToAssistant`. */
  startAssistantMessage(): string {
    const id = crypto.randomUUID();
    this.currentMessages = [
      ...this.currentMessages,
      { id, role: 'assistant', content: '' },
    ];
    return id;
  }

  appendToAssistant(id: string, chunk: string): void {
    const idx = this.currentMessages.findIndex((m) => m.id === id);
    if (idx < 0) return;
    // Replace the row immutably so the rune reacts.
    this.currentMessages = this.currentMessages.map((m, i) =>
      i === idx ? { ...m, content: m.content + chunk } : m,
    );
  }

  /** After the first send, the backend auto-titled this conversation. Pull
   *  the fresh row so the sidebar reflects the new title. Best-effort. */
  async refreshCurrentInList(): Promise<void> {
    if (!this.currentId) return;
    try {
      this.list = await listConversations();
    } catch (e) {
      console.error('[conversations.refresh]', e);
    }
  }

  // ── Mutation on the sidebar list ───────────────────────────────────────

  async renameConversation(id: string, title: string): Promise<void> {
    const updated = await updateConversationTitle(id, title);
    this.list = this.list.map((c) => (c.id === id ? updated : c));
  }

  async deleteConversation(id: string): Promise<void> {
    await serviceDelete(id);
    this.list = this.list.filter((c) => c.id !== id);
    if (this.currentId === id) {
      this.newChat();
    }
  }

  // ── Logout cleanup ─────────────────────────────────────────────────────

  reset(): void {
    this.list = [];
    this.loaded = false;
    this.currentId = null;
    this.currentMessages = [];
    this.error = null;
  }
}

export const conversations = new ConversationsStore();
