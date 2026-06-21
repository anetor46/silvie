import {
  createConversation,
  deleteConversation as serviceDelete,
  getConversation,
  listConversations,
  updateConversationTitle,
  type Conversation,
} from '$lib/services/conversations';
import type { Message, ToolCallEntry } from '$lib/types';

interface BackendMessage {
  id: string;
  role: string;
  content: string;
  tool_name?: string | null;
  tool_call_id?: string | null;
}

interface StoredToolPayload {
  args?: Record<string, unknown>;
  requires_confirmation?: boolean;
  status?: string;
  summary?: string | null;
  success?: boolean | null;
}

/** Map a backend message row to the UI view-model. Returns null for rows we
 *  don't render (e.g. system rows that snuck in). For tool rows, parse the
 *  JSON content payload into a `ToolCallEntry`. */
function toUiMessage(m: BackendMessage): Message | null {
  if (m.role === 'user' || m.role === 'assistant') {
    return { id: m.id, role: m.role, content: m.content };
  }
  if (m.role === 'tool') {
    let parsed: StoredToolPayload = {};
    try {
      parsed = JSON.parse(m.content) as StoredToolPayload;
    } catch {
      // fall through with empty parsed
    }
    const status = (parsed.status ?? 'success') as ToolCallEntry['status'];
    const decision =
      status === 'success' && parsed.requires_confirmation
        ? 'approved'
        : status === 'error' && parsed.requires_confirmation
          ? 'rejected'
          : undefined;
    const toolCall: ToolCallEntry = {
      callId: m.tool_call_id ?? m.id,
      name: m.tool_name ?? 'unknown',
      args: parsed.args ?? {},
      requiresConfirmation: parsed.requires_confirmation ?? false,
      status,
      summary: parsed.summary ?? undefined,
      decision,
    };
    return { id: m.id, role: 'tool', content: m.content, toolCall };
  }
  return null;
}

class ConversationsStore {
  list = $state<Conversation[]>([]);
  loaded = $state(false);
  error = $state<string | null>(null);

  currentId = $state<string | null>(null);
  currentMessages = $state<Message[]>([]);

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
        .map((m) => toUiMessage(m as BackendMessage))
        .filter((m): m is Message => m !== null);
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
    }
  }

  // ── Composition flow ───────────────────────────────────────────────────

  newChat(): void {
    this.currentId = null;
    this.currentMessages = [];
    this.error = null;
  }

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

  /** Append an empty assistant placeholder and return its temp id. */
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
    this.currentMessages = this.currentMessages.map((m, i) =>
      i === idx ? { ...m, content: m.content + chunk } : m,
    );
  }

  // ── Tool call streaming ────────────────────────────────────────────────

  /** Append a new tool-call row in the running message list. The card
   *  shows as a spinner (or "awaiting approval" badge if requiresConfirmation
   *  is true) until a matching toolResult arrives. */
  appendToolCall(entry: {
    callId: string;
    name: string;
    args: Record<string, unknown>;
    requiresConfirmation: boolean;
  }): void {
    const toolCall: ToolCallEntry = {
      callId: entry.callId,
      name: entry.name,
      args: entry.args,
      requiresConfirmation: entry.requiresConfirmation,
      status: entry.requiresConfirmation ? 'pending_user' : 'running',
    };
    this.currentMessages = [
      ...this.currentMessages,
      {
        id: crypto.randomUUID(),
        role: 'tool',
        content: '',
        toolCall,
      },
    ];
  }

  /** Update the matching tool row's status / summary when the tool result
   *  arrives. No-op if no matching row exists. */
  updateToolResult(callId: string, success: boolean, summary: string | null): void {
    this.currentMessages = this.currentMessages.map((m) => {
      if (m.role !== 'tool' || m.toolCall?.callId !== callId) return m;
      return {
        ...m,
        toolCall: {
          ...m.toolCall!,
          status: success ? 'success' : 'error',
          summary: summary ?? m.toolCall!.summary,
        },
      };
    });
  }

  /** Record the user's Approve / Reject click locally so the buttons
   *  collapse into a badge while waiting for the tool to actually run. */
  setDecision(callId: string, decision: 'approved' | 'rejected'): void {
    this.currentMessages = this.currentMessages.map((m) => {
      if (m.role !== 'tool' || m.toolCall?.callId !== callId) return m;
      return {
        ...m,
        toolCall: {
          ...m.toolCall!,
          decision,
          // Once approved, status transitions to running until the actual
          // execution completes and the ToolResult event flips it again.
          status: decision === 'approved' ? 'running' : 'error',
        },
      };
    });
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
